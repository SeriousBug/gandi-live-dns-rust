use crate::config::Config;
use anyhow;
use clap::Parser;
use futures;
use opts::SilenceLevel;
use reqwest::{header, Client, ClientBuilder, StatusCode};
use std::{collections::HashMap, num::NonZeroU32, sync::Arc, time::Duration};
use tokio::{self, task::JoinHandle};
use tracing::metadata::LevelFilter;
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

/// Sets up the logging based on the command line options given.
///
/// As a reminder, the use the following level should be used when printing
/// output:
/// - error: Error messages
/// - info: Regular operational messages
/// - debug: Any messages that contain private information, like domain names
///
fn setup_logging(level: Option<SilenceLevel>) {
    tracing_subscriber::fmt()
        .with_level(false)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_max_level(match level {
            Some(SilenceLevel::All) => LevelFilter::WARN,
            Some(SilenceLevel::Domains) => LevelFilter::INFO,
            None => LevelFilter::DEBUG,
        })
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opts = opts::Opts::parse();
    setup_logging(opts.silent);
    // setup_logging needs to come first, before anything else

    let conf = config::load_config(&opts)
        .die_with(|error| format!("Failed to read config file: {}", error));
    config::validate_config(&conf).die_with(|error| format!("Invalid config: {}", error));
    tracing::info!("Finding out the IP address...");
    let ipv4_result = get_ip("https://api.ipify.org").await;
    let ipv6_result = get_ip("https://api6.ipify.org").await;
    let ipv4 = ipv4_result.as_ref();
    let ipv6 = ipv6_result.as_ref();
    tracing::debug!("Found these:");
    match ipv4 {
        Ok(ip) => tracing::debug!("\tIPv4: {}", ip),
        Err(err) => tracing::error!("\tIPv4 failed: {}", err),
    }
    match ipv6 {
        Ok(ip) => tracing::debug!("\tIPv6: {}", ip),
        Err(err) => tracing::error!("\tIPv6 failed: {}", err),
    }

    let client = api_client(&conf.api_key)?;
    let mut tasks: Vec<JoinHandle<(StatusCode, String)>> = Vec::new();
    tracing::info!("Attempting to update DNS entries now");

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
                tracing::debug!("Updating {}", &fqdn);
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
    tracing::info!("Updates done for {} entries", results.len());
    results
        .into_iter()
        .filter(|(status, _)| !StatusCode::is_success(&status))
        .for_each(|(status, body)| tracing::warn!("Error {}: {}", status, body));

    return Ok(());
}
