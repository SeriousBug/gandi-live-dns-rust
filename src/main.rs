use crate::config::Config;
use crate::gandi::GandiAPI;
use crate::ip_source::{common::IPSource, ipify::IPSourceIpify};
use clap::Parser;
use config::{ConfigError, IPSourceName};
use ip_source::icanhazip::IPSourceIcanhazip;
use ip_source::seeip::IPSourceSeeIP;
use opts::Opts;
use reqwest::header::InvalidHeaderValue;
use reqwest::{header, Client, ClientBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, sync::Arc, time::Duration};
use tokio::join;
use tokio::{self, task::JoinHandle, time::sleep};
mod config;
mod gandi;
mod ip_source;
mod opts;
use die_exit::*;
use thiserror::Error;

/// 30 requests per minute, see https://api.gandi.net/docs/reference/
const GANDI_RATE_LIMIT: u32 = 30;
/// If we hit the rate limit, wait up to this many seconds before next attempt
const GANDI_DELAY_JITTER: u64 = 20;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Error occured while reading config: {0}")]
    Config(#[from] ConfigError),
    #[error("Error while accessing the Gandi API: {0}")]
    Api(#[from] ApiError),
    #[error("Error while converting the API key to a header: {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),
    #[error("Error while sending request: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Error while joining async tasks: {0}")]
    TaskJoin(#[from] tokio::task::JoinError),
    #[error("Unexpected type in config: {0}")]
    BadEntry(String),
    #[error("Entry '{0}' includes type A which requires an IPv4 adress but no IPv4 adress could be determined because: {1}")]
    Ipv4missing(String, String),
    #[error("Entry '{0}' includes type AAAA which requires an IPv6 adress but no IPv6 adress could be determined because: {1}")]
    Ipv6missing(String, String),
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("API returned 403 - Forbidden. Message: {message:?}")]
    Forbidden { message: String },
    #[error("API returned 403 - Unauthorized. Provided API key is possibly incorrect")]
    Unauthorized(),
    #[error("API returned {0} - {0}")]
    Unknown(StatusCode, String),
}

fn api_client(api_key: &str) -> Result<Client, ClientError> {
    let client_builder = ClientBuilder::new();

    let key = format!("Apikey {api_key}");
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

#[derive(Debug)]
struct ResponseFeedback {
    entry_name: String,
    entry_type: String,
    response: Result<String, ApiError>,
}

#[derive(Deserialize)]
// Allowing dead code because this is the API response we get from Gandi.
// We don't necessarily need all the fields, but we get them anyway.
#[allow(dead_code)]
struct ApiResponse {
    message: String,
    cause: Option<String>,
    code: Option<i32>,
    object: Option<String>,
}

async fn run(
    base_url: &str,
    ip_source: &Box<dyn IPSource>,
    conf: &Config,
    opts: &Opts,
) -> Result<(), ClientError> {
    let mut last_ipv4: Option<String> = None;
    let mut last_ipv6: Option<String> = None;

    loop {
        println!("Finding out the IP address...");
        let (ipv4_result, ipv6_result) = join!(ip_source.get_ipv4(), ip_source.get_ipv6());
        let ipv4 = ipv4_result.as_ref();
        let ipv6 = ipv6_result.as_ref();
        println!("Found these:");
        match ipv4 {
            Ok(ip) => println!("\tIPv4: {ip}"),
            Err(err) => eprintln!("\tIPv4 failed: {err}"),
        }
        match ipv6 {
            Ok(ip) => println!("\tIPv6: {ip}"),
            Err(err) => eprintln!("\tIPv6 failed: {err}"),
        }

        let ipv4_same = last_ipv4
            .as_ref()
            .map(|p| ipv4.map(|q| p == q).unwrap_or(false))
            .unwrap_or(false);
        let ipv6_same = last_ipv6
            .as_ref()
            .map(|p| ipv6.map(|q| p == q).unwrap_or(false))
            .unwrap_or(false);

        if !ipv4_same || !ipv6_same || conf.always_update {
            let client = api_client(&conf.api_key)?;
            let mut tasks: Vec<JoinHandle<Result<ResponseFeedback, ClientError>>> = Vec::new();
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
                        "A" => match ipv4 {
                            Ok(ref value) => Ok(value),
                            Err(ref err) => Err(ClientError::Ipv4missing(
                                entry.name.clone(),
                                err.to_string(),
                            )),
                        },
                        "AAAA" => match ipv6 {
                            Ok(ref value) => Ok(value),
                            Err(ref err) => Err(ClientError::Ipv6missing(
                                entry.name.clone(),
                                err.to_string(),
                            )),
                        },
                        &_ => Err(ClientError::BadEntry(entry_type.to_string())),
                    }?;
                    let payload = APIPayload {
                        rrset_values: vec![ip.to_string()],
                        rrset_ttl: Config::ttl(entry, conf),
                    };
                    let req = client.put(url).json(&payload);
                    let task_governor = governor.clone();
                    let entry_type = entry_type.to_string();
                    let entry_name = entry.name.to_string();

                    let task: JoinHandle<Result<ResponseFeedback, ClientError>> =
                        tokio::task::spawn(async move {
                            task_governor.until_ready_with_jitter(retry_jitter).await;
                            println!("Updating {} record for {}", entry_type, &fqdn);

                            let resp = req.send().await?;

                            let response_feedback = match resp.status() {
                                StatusCode::CREATED => {
                                    let body: ApiResponse = resp.json().await?;
                                    ResponseFeedback {
                                        entry_name,
                                        entry_type,
                                        response: Ok(body.message),
                                    }
                                }
                                StatusCode::UNAUTHORIZED => ResponseFeedback {
                                    entry_name,
                                    entry_type,
                                    response: Err(ApiError::Unauthorized()),
                                },
                                StatusCode::FORBIDDEN => {
                                    let body: ApiResponse = resp.json().await?;
                                    ResponseFeedback {
                                        entry_name: entry_name.clone(),
                                        entry_type,
                                        response: Err(ApiError::Forbidden {
                                            message: body.message,
                                        }),
                                    }
                                }
                                _ => {
                                    let status = resp.status();
                                    let body: ApiResponse = resp.json().await?;
                                    ResponseFeedback {
                                        entry_name,
                                        entry_type,
                                        response: Err(ApiError::Unknown(status, body.message)),
                                    }
                                }
                            };
                            Ok(response_feedback)
                        });
                    tasks.push(task);
                }
            }

            let results = futures::future::try_join_all(tasks).await?;
            // Only count successfull requests
            println!(
                "Updates done for {} entries",
                results
                    .iter()
                    .filter_map(|item| item.as_ref().ok())
                    .filter(|item| item.response.is_ok())
                    .count()
            );
            for item in &results {
                match item {
                    Ok(value) => println!(
                        "{}",
                        match &value.response {
                            Ok(val) => format!(
                                "Record '{}' ({}): {}",
                                value.entry_name, value.entry_type, val
                            ),
                            Err(err) => format!(
                                "Record '{}' ({}): {}",
                                value.entry_name, value.entry_type, err
                            ),
                        }
                    ),
                    Err(err) => println!("{err}"),
                }
            }
            if results
                .iter()
                // all tasks finished OK, and all responses were OK as well
                .all(|result| result.as_ref().map(|v| v.response.is_ok()).unwrap_or(false))
            {
                // Only then we update the last seen IP, because we want to
                // retry updates in case the last update just happened to fail
                last_ipv4 = ipv4.ok().map(|v| v.to_string());
                last_ipv6 = ipv6.ok().map(|v| v.to_string());
            } else if opts.repeat.is_some() {
                println!("Some operations failed. They will be retried during the next repeat.")
            }
        } else {
            println!("IP address has not changed since last update");
        }

        if let Some(repeat) = opts.repeat {
            // If configured to repeat, do so
            sleep(Duration::from_secs(repeat)).await;
            continue;
        }
        // Otherwise this is one-shot, we should exit now
        break;
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let opts = opts::Opts::parse();
    let conf = config::load_config(&opts)?;

    let ip_source: Box<dyn IPSource> = match conf.ip_source {
        IPSourceName::Ipify => Box::new(IPSourceIpify),
        IPSourceName::Icanhazip => Box::new(IPSourceIcanhazip),
        IPSourceName::SeeIP => Box::new(IPSourceSeeIP),
    };
    config::validate_config(&conf)?;
    run("https://api.gandi.net", &ip_source, &conf, &opts).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{config, ip_source::common::IPSource, opts::Opts, run, ClientError};
    use async_trait::async_trait;
    use httpmock::MockServer;
    use lazy_static::lazy_static;
    use std::{
        env::temp_dir,
        sync::atomic::{AtomicBool, Ordering::SeqCst},
        time::Duration,
    };
    use tokio::{fs, task::LocalSet, time::sleep};

    struct IPSourceMock;

    #[async_trait]
    impl IPSource for IPSourceMock {
        async fn get_ipv4(&self) -> Result<String, ClientError> {
            Ok("192.168.0.0".to_string())
        }
        async fn get_ipv6(&self) -> Result<String, ClientError> {
            Ok("fe80:0000:0208:74ff:feda:625c".to_string())
        }
    }

    #[tokio::test]
    async fn single_shot() {
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
            then.status(201)
                .body("{\"cause\":\"\", \"code\":201, \"message\":\"\", \"object\":\"\"}");
        });

        let opts = Opts {
            config: Some(temp.to_string_lossy().to_string()),
            ..Opts::default()
        };
        let conf = config::load_config(&opts).expect("Failed to load config");
        let ip_source: Box<dyn IPSource> = Box::new(IPSourceMock);
        run(server.base_url().as_str(), &ip_source, &conf, &opts)
            .await
            .expect("Failed when running the update");

        // Assert
        mock.assert();
    }

    #[test]
    fn repeat() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        LocalSet::new().block_on(&runtime, async {
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
                then.status(201)
                    .body("{\"cause\":\"\", \"code\":201, \"message\":\"\", \"object\":\"\"}");
            });

            let server_url = server.base_url();
            let handle = tokio::task::spawn_local(async move {
                let opts = Opts {
                    config: Some(temp.to_string_lossy().to_string()),
                    repeat: Some(1),
                    ..Opts::default()
                };
                let conf = config::load_config(&opts).expect("Failed to load config");
                let ip_source: Box<dyn IPSource> = Box::new(IPSourceMock);
                run(&server_url, &ip_source, &conf, &opts)
                    .await
                    .expect("Failed when running the update");
            });

            sleep(Duration::from_secs(3)).await;
            handle.abort();

            // Only should update once because the IP doesn't change
            mock.assert();
        });
    }

    #[test]
    fn repeat_with_failure() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        LocalSet::new().block_on(&runtime, async {
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
                    .body_contains("192.168.0.0")
                    .matches(|_| {
                        // Don't match during the first call, but do during the second call
                        lazy_static! {
                            static ref FIRST_CALL: AtomicBool = AtomicBool::new(true);
                        }
                        if FIRST_CALL.load(SeqCst) {
                            FIRST_CALL.store(false, SeqCst);
                            return true;
                        }
                        false
                    });
                then.status(500)
                    .body("{\"cause\":\"\", \"code\":500, \"message\":\"Something went wrong\", \"object\":\"\"}");
            });
            let mock_fail = server.mock(|when, then| {
                when.method("PUT")
                    .path(format!(
                        "/v5/livedns/domains/{fqdn}/records/{rname}/{rtype}"
                    ))
                    .body_contains("192.168.0.0");
                then.status(201)
                    .body("{\"cause\":\"\", \"code\":201, \"message\":\"\", \"object\":\"\"}");
            });

            let server_url = server.base_url();
            let handle = tokio::task::spawn_local(async move {
                let opts = Opts {
                    config: Some(temp.to_string_lossy().to_string()),
                    repeat: Some(1),
                    ..Opts::default()
                };
                let conf = config::load_config(&opts).expect("Failed to load config");
                let ip_source: Box<dyn IPSource> = Box::new(IPSourceMock);
                run(&server_url, &ip_source, &conf, &opts)
                    .await
                    .expect("Failed when running the update");
            });

            sleep(Duration::from_secs(4)).await;
            handle.abort();

            // The first call failed
            mock_fail.assert();
            // We then retried since the first call failed. The retry succeeds
            // so we don't retry again.
            mock.assert();
        });
    }

    #[test]
    fn repeat_always_update() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        LocalSet::new().block_on(&runtime, async {
            let mut temp = temp_dir().join("gandi-live-dns-test");
            fs::create_dir_all(&temp)
                .await
                .expect("Failed to create test dir");
            temp.push("test.toml");
            fs::write(
                &temp,
                "fqdn = \"example.com\"\nalways_update = true\napi_key = \"xxx\"\nttl = 300\n[[entry]]\nname =\"@\"\n",
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
                then.status(201).body("{\"cause\":\"\", \"code\":201, \"message\":\"\", \"object\":\"\"}");
            });

            let server_url = server.base_url();
            let handle = tokio::task::spawn_local(async move {
                let opts = Opts {
                    config: Some(temp.to_string_lossy().to_string()),
                    repeat: Some(1),
                    ..Opts::default()
                };
                let conf = config::load_config(&opts).expect("Failed to load config");
                let ip_source: Box<dyn IPSource> = Box::new(IPSourceMock);
                run(&server_url, &ip_source, &conf, &opts)
                    .await
                    .expect("Failed when running the update");
            });

            sleep(Duration::from_secs(3)).await;
            handle.abort();

            // Should update multiple times since always_update
            assert!(mock.hits() > 1);
        });
    }
}
