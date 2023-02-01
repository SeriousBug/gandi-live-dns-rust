use async_trait::async_trait;

#[async_trait]
pub trait IPSource {
    async fn get_ipv4(&self) -> anyhow::Result<String>;
    async fn get_ipv6(&self) -> anyhow::Result<String>;
}
