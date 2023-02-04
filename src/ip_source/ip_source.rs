use async_trait::async_trait;

use crate::ClientError;

#[async_trait]
pub trait IPSource {
    async fn get_ipv4(&self) -> Result<String, ClientError>;
    async fn get_ipv6(&self) -> Result<String, ClientError>;
}
