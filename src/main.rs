use crate::config::Config;
use anyhow;
use futures;
use reqwest::{header, Client, ClientBuilder, StatusCode};
use std::{collections::HashMap, process::exit};
use structopt::StructOpt;
use tokio::{self, task::JoinHandle};
mod config;
mod opts;
use die_exit::*;

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
    println!("Finding out the API address...");
    let ipv4 = get_ip("https://api.ipify.org").await;
    let ipv6 = get_ip("https://api6.ipify.org").await;
    println!("Found these:");
    println!("\tIPv4: {}", ipv4.unwrap_or_else(|error| error.to_string()));
    println!("\tIPv6: {}", ipv6.unwrap_or_else(|error| error.to_string()));

    let client = api_client(&conf.api_key)?;
    let mut tasks: Vec<JoinHandle<(StatusCode, String)>> = Vec::new();
    println!("Attempting to update DNS entries now");

    for entry in &conf.entry {
        for entry_type in Config::types(entry) {
            let fqdn = Config::fqdn(&entry, &conf);
            let url = gandi_api_url(fqdn, entry.name.as_str(), entry_type);
            let ip = match entry_type {
                "A" => ipv4.unwrap_or_else(|error| {
                    panic!(
                        "Need IPv4 address for {} but failed to get it: {}",
                        fqdn, error
                    )
                }),
                "AAA" => ipv6.unwrap_or_else(|error| {
                    panic!(
                        "Need IPv6 address for {} but failed to get it: {}",
                        fqdn, error
                    )
                }),
                &_ => panic!("Unexpected entry type {}", entry_type),
            };
            let mut map = HashMap::new();
            map.insert("rrset_values", ip);
            let req = client.put(url).json(&map);
            let task = tokio::task::spawn(async move {
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
