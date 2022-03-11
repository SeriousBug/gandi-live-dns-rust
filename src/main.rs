use crate::config::Config;
use anyhow;
use futures;
use reqwest::{header, Client, ClientBuilder, StatusCode};
use std::{collections::HashMap, num::NonZeroU32, sync::Arc, time::Duration};
use structopt::StructOpt;
use tokio::{self, task::JoinHandle};
mod config;
mod opts;
use die_exit::*;

use governor;

/// 30 requests per minute, see https://api.gandi.net/docs/reference/
const GANDI_RATE_LIMIT: u32 = 30;
/// If we hit the rate limit, wait up to this many seconds before next attempt
const GANDI_DELAY_JITTER: u64 = 20;

fn gandi_api_url(fqdn: &str, rrset_name: &str, rrset_type: &str) -> String {
    return format!(
        " https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}",
        fqdn, rrset_name, rrset_type
    );
}

fn api_client(api_key: &str) -> anyhow::Result<Client> {
    let client_builder = ClientBuilder::new();

    let key = format!("Apikey {}", api_key);
    let mut auth_value = header::HeaderValue::from_str(&key)?;
    let mut headers = header::HeaderMap::new();
    auth_value.set_sensitive(true);
    headers.insert(header::AUTHORIZATION, auth_value);
    let accept_value = header::HeaderValue::from_static("application/json");
    headers.insert(header::ACCEPT, accept_value);
    let client = client_builder.default_headers(headers).build()?;
    return Ok(client);
}

async fn get_ip(api_url: &str) -> anyhow::Result<String> {
    let response = reqwest::get(api_url).await?;
    let text = response.text().await?;
    Ok(text)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = opts::Opts::from_args();
    let conf = config::load_config(&opts)
        .die_with(|error| format!("Failed to read config file: {}", error));
    config::validate_config(&conf).die_with(|error| format!("Invalid config: {}", error));
    println!("Finding out the IP address...");
    let ipv4_result = get_ip("https://api.ipify.org").await;
    let ipv6_result = get_ip("https://api6.ipify.org").await;
    let ipv4 = ipv4_result.as_ref();
    let ipv6 = ipv6_result.as_ref();
    println!("Found these:");
    match ipv4 {
        Ok(ip) => println!("\tIPv4: {}", ip),
        Err(err) => eprintln!("\tIPv4 failed: {}", err),
    }
    match ipv6 {
        Ok(ip) => println!("\tIPv6: {}", ip),
        Err(err) => eprintln!("\tIPv6 failed: {}", err),
    }

    let client = api_client(&conf.api_key)?;
    let mut tasks: Vec<JoinHandle<(StatusCode, String)>> = Vec::new();
    println!("Attempting to update DNS entries now");

    let governor = Arc::new(governor::RateLimiter::direct(governor::Quota::per_minute(
        NonZeroU32::new(GANDI_RATE_LIMIT).die("Governor rate is 0"),
    )));
    let retry_jitter =
        governor::Jitter::new(Duration::ZERO, Duration::from_secs(GANDI_DELAY_JITTER));

    for entry in &conf.entry {
        for entry_type in Config::types(entry) {
            let fqdn = Config::fqdn(&entry, &conf).to_string();
            let url = gandi_api_url(&fqdn, entry.name.as_str(), entry_type);
            let ip = match entry_type {
                "A" => ipv4.die_with(|error| format!("Needed IPv4 for {}: {}", fqdn, error)),
                "AAAA" => ipv6.die_with(|error| format!("Needed IPv6 for {}: {}", fqdn, error)),
                bad_entry_type => die!("Unexpected type in config: {}", bad_entry_type),
            };
            let mut map = HashMap::new();
            map.insert("rrset_values", vec![ip]);
            let req = client.put(url).json(&map);
            let task_governor = governor.clone();
            let task = tokio::task::spawn(async move {
                task_governor.until_ready_with_jitter(retry_jitter).await;
                println!("Updating {}", &fqdn);
                match req.send().await {
                    Ok(response) => (
                        response.status(),
                        response
                            .text()
                            .await
                            .unwrap_or_else(|error| error.to_string()),
                    ),
                    Err(error) => (StatusCode::IM_A_TEAPOT, error.to_string()),
                }
            });
            tasks.push(task);
        }
    }

    let results = futures::future::try_join_all(tasks).await?;
    println!("Updates done for {} entries", results.len());
    for (status, body) in results {
        println!("{} - {}", status, body);
    }

    return Ok(());
}
