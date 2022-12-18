use crate::config::Config;
use crate::gandi::GandiAPI;
use crate::ip_source::{ip_source::IPSource, ipify::IPSourceIpify};
use clap::Parser;
use config::IPSourceName;
use ip_source::icanhazip::IPSourceIcanhazip;
use reqwest::{header, Client, ClientBuilder, StatusCode};
use serde::Serialize;
use std::{num::NonZeroU32, sync::Arc, time::Duration};
use tokio::{self, task::JoinHandle, time::sleep};
mod config;
mod gandi;
mod ip_source;
mod opts;
use die_exit_2::*;

/// 30 requests per minute, see https://api.gandi.net/docs/reference/
const GANDI_RATE_LIMIT: u32 = 30;
/// If we hit the rate limit, wait up to this many seconds before next attempt
const GANDI_DELAY_JITTER: u64 = 20;

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
    Ok(client)
}

#[derive(Serialize)]
pub struct APIPayload {
    pub rrset_values: Vec<String>,
    pub rrset_ttl: u32,
}

async fn run<IP: IPSource>(base_url: &str, conf: &Config) -> anyhow::Result<()> {
    config::validate_config(conf).die_with(|error| format!("Invalid config: {}", error));
    println!("Finding out the IP address...");
    let ipv4_result = IP::get_ipv4().await;
    let ipv6_result = IP::get_ipv6().await;
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
            let fqdn = Config::fqdn(entry, conf).to_string();
            let url = GandiAPI {
                fqdn: &fqdn,
                rrset_name: &entry.name,
                rrset_type: entry_type,
                base_url,
            }
            .url();
            let ip = match entry_type {
                "A" => ipv4.die_with(|error| format!("Needed IPv4 for {fqdn}: {error}")),
                "AAAA" => ipv6.die_with(|error| format!("Needed IPv6 for {fqdn}: {error}")),
                bad_entry_type => die!("Unexpected type in config: {}", bad_entry_type),
            };
            let payload = APIPayload {
                rrset_values: vec![ip.to_string()],
                rrset_ttl: Config::ttl(entry, conf),
            };
            let req = client.put(url).json(&payload);
            let task_governor = governor.clone();
            let entry_type = entry_type.to_string();
            let task = tokio::task::spawn(async move {
                task_governor.until_ready_with_jitter(retry_jitter).await;
                println!("Updating {} record for {}", entry_type, &fqdn);
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

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let opts = opts::Opts::parse();
    let conf = config::load_config(&opts)
        .die_with(|error| format!("Failed to read config file: {}", error));

    // run indefinitely if repeat is given
    if let Some(delay) = opts.repeat {
        loop {
            run_dispatch(&conf).await.ok();
            sleep(Duration::from_secs(delay)).await
        }
    }
    // otherwise run just once
    else {
        run_dispatch(&conf).await?;
        Ok(())
    }
}

async fn run_dispatch(conf: &Config) -> anyhow::Result<()> {
    match conf.ip_source {
        IPSourceName::Ipify => run::<IPSourceIpify>("https://api.gandi.net", conf).await,
        IPSourceName::Icanhazip => run::<IPSourceIcanhazip>("https://api.gandi.net", conf).await,
    }
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use crate::{config, ip_source::ip_source::IPSource, opts::Opts, run};
    use async_trait::async_trait;
    use httpmock::MockServer;
    use tokio::fs;

    struct IPSourceMock {}

    #[async_trait]
    impl IPSource for IPSourceMock {
        async fn get_ipv4() -> anyhow::Result<String> {
            Ok("192.168.0.0".to_string())
        }
        async fn get_ipv6() -> anyhow::Result<String> {
            Ok("fe80:0000:0208:74ff:feda:625c".to_string())
        }
    }

    #[tokio::test]
    async fn create_repo_success_test() {
        let mut temp = temp_dir().join("gandi-live-dns-test");
        fs::create_dir_all(&temp)
            .await
            .expect("Failed to create test dir");
        temp.push("test.toml");
        fs::write(
            &temp,
            "fqdn = \"example.com\"\napi_key = \"xxx\"\nttl = 300\n[[entry]]\nname =\"@\"\n",
        )
        .await
        .expect("Failed to write test config file");
        let fqdn = "example.com";
        let rname = "@";
        let rtype = "A";
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method("PUT")
                .path(format!(
                    "/v5/livedns/domains/{fqdn}/records/{rname}/{rtype}"
                ))
                .body_contains("192.168.0.0");
            then.status(200);
        });

        let opts = Opts {
            config: Some(temp.to_string_lossy().to_string()),
            ..Opts::default()
        };
        let conf = config::load_config(&opts).expect("Failed to load config");
        run::<IPSourceMock>(server.base_url().as_str(), &conf)
            .await
            .expect("Failed when running the update");

        // Assert
        mock.assert();
    }
}
