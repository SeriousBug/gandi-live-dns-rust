use anyhow;
use async_trait::async_trait;

use super::ip_source::IPSource;

pub(crate) struct IPSourceIpify {}

async fn get_ip(api_url: &str) -> anyhow::Result<String> {
  let response = reqwest::get(api_url).await?;
  let text = response.text().await?;
  Ok(text)
}

#[async_trait]
impl IPSource for IPSourceIpify {
  async fn get_ipv4() -> anyhow::Result<String> {
    get_ip("https://api.ipify.org").await
  }
  async fn get_ipv6() -> anyhow::Result<String> {
    get_ip("https://api6.ipify.org").await
  }
}
