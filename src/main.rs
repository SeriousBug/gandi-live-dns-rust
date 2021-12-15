use reqwest::{header, Client, ClientBuilder};
use std::error::Error;
use structopt::StructOpt;
use tokio;
mod config;
mod opts;

fn gandi_api(fqdn: &str) -> String {
    return format!("https://api.gandi.net/v5/livedns/domains/{}/records", fqdn);
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
    println!("Checking domain {:#?}", conf.fqdn);
    let url = gandi_api(&conf.fqdn);
    let client = api_client(&conf.api_key)?;

    let out = client.get(url).send().await?;
    println!("Output: {:#?}", out);
    println!("Output: {:#?}", out.json().await?);

    return Ok(());
}
