use async_trait::async_trait;

use crate::ClientError;

use super::ip_source::IPSource;

pub(crate) struct IPSourceSeeIP;

async fn get_ip(api_url: &str) -> Result<String, ClientError> {
    let response = reqwest::get(api_url).await?;
    let text = response.text().await?;
    Ok(text)
}

#[async_trait]
impl IPSource for IPSourceSeeIP {
    async fn get_ipv4(&self) -> Result<String, ClientError> {
        get_ip("https://ip4.seeip.org").await
    }
    async fn get_ipv6(&self) -> Result<String, ClientError> {
        get_ip("https://ip6.seeip.org").await
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use super::IPSource;
    use super::IPSourceSeeIP;

    #[tokio::test]
    #[ignore]
    async fn ipv4_test() {
        let ipv4 = IPSourceSeeIP
            .get_ipv4()
            .await
            .expect("Failed to get the IP address");
        assert!(Regex::new(r"^\d+[.]\d+[.]\d+[.]\d+$")
            .unwrap()
            .is_match(ipv4.as_str()))
    }

    #[tokio::test]
    #[ignore]
    async fn ipv6_test() {
        let ipv6 = IPSourceSeeIP
            .get_ipv6()
            .await
            .expect("Failed to get the IP address");
        assert!(Regex::new(r"^([0-9a-fA-F]*:){7}[0-9a-fA-F]*$")
            .unwrap()
            .is_match(ipv6.as_str()))
    }
}
