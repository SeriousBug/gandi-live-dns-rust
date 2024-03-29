use async_trait::async_trait;

use crate::ClientError;

use super::common::IPSource;

pub(crate) struct IPSourceIcanhazip;

async fn get_ip(api_url: &str) -> Result<String, ClientError> {
    let response = reqwest::get(api_url).await?;
    let text = response.text().await?;
    Ok(text)
}

#[async_trait]
impl IPSource for IPSourceIcanhazip {
    async fn get_ipv4(&self) -> Result<String, ClientError> {
        Ok(get_ip("https://ipv4.icanhazip.com")
            .await?
            // icanazip puts a newline at the end
            .trim()
            .to_string())
    }
    async fn get_ipv6(&self) -> Result<String, ClientError> {
        Ok(get_ip("https://ipv6.icanhazip.com")
            .await?
            // icanazip puts a newline at the end
            .trim()
            .to_string())
    }
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    use crate::ip_source::common::IPSource;

    use super::IPSourceIcanhazip;

    #[tokio::test]
    #[ignore]
    async fn ipv4_test() {
        let ipv4 = IPSourceIcanhazip
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
        let ipv6 = IPSourceIcanhazip
            .get_ipv6()
            .await
            .expect("Failed to get the IP address");
        assert!(Regex::new(r"^([0-9a-fA-F]*:){7}[0-9a-fA-F]*$")
            .unwrap()
            .is_match(ipv6.as_str()))
    }
}
