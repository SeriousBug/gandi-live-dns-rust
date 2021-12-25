use std::collections::HashMap;
use crate::config::Config;
use reqwest::{header, Client, ClientBuilder};
use std::error::Error;
use structopt::StructOpt;
use tokio;
use futures;
mod config;
mod opts;

fn gandi_api_get(fqdn: &str) -> String {
    return format!("https://api.gandi.net/v5/livedns/domains/{}/records", fqdn);
}

fn gandi_api_url(fqdn: &str, rrset_name: &str, rrset_type: &str) -> String {
    return format!(" https://api.gandi.net/v5/livedns/domains/{}/records/{}/{}", fqdn, rrset_name, rrset_type);
}

fn api_client(api_key: &str) -> Result<Client, Box<dyn Error>> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opts = opts::Opts::from_args();
    let conf_path = config::config_path(&opts);
    println!("Loading config from {:#?}", conf_path);
    let conf = config::load_config(conf_path)?;
    config::validate_config(&conf)?;

    let client = api_client(&conf.api_key)?;

    let ipv4 = String::from("173.89.215.91");
    let ipv6 = String::from("2603:6011:be07:302:79f4:50dd:6abe:be38");

    let mut results = Vec::new();

    for entry in &conf.entry {
        for entry_type in Config::types(entry) {
            let fqdn = Config::fqdn(&entry, &conf);
            let url = gandi_api_url(fqdn, entry.name.as_str(), entry_type);
            let ip = if entry_type.eq("A") { ipv4.as_str() } else { ipv6.as_str() };
            let mut map = HashMap::new();
            map.insert("rrset_values", ip);
            let req = client.put(url).json(&map);
            let task = tokio::task::spawn(async move {
                let response = req.send().await?;
                return (response.status(), response.text().await?);
            });
            results.push(task);
        }
    }
    
    let results = futures::future::try_join_all(results).await?;
    
    for (status, body) in results {
        println!("{} - {}", status, body);
    }

    return Ok(());
}
