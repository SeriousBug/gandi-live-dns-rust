use anyhow;
use async_trait::async_trait;

#[async_trait]
pub trait IPSource {
  async fn get_ipv4() -> anyhow::Result<String>;
  async fn get_ipv6() -> anyhow::Result<String>;
}
