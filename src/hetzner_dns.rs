use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    config::HetznerConfig,
    consul,
    dns_trait::{DnsProviderTrait, DnsRecord},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordsWrapper {
    records: Vec<DnsRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecordResponse {
    record: DnsRecord,
}

pub struct HetznerDns {
    pub config: HetznerConfig,
}

#[async_trait]
impl DnsProviderTrait for HetznerDns {
    /// Create a DNS record based on the Consul service tags
    async fn create_dns_record<'a>(
        &self,
        dns_record: &'a consul::DnsRecord,
    ) -> Result<String, anyhow::Error> {
        let new_record = json!({
            "zone_id": self.config.dns_zone_id,
            "type": dns_record.type_,
            "name": dns_record.hostname,
            "value": dns_record.value,
            "ttl": dns_record.ttl
        });

        let client = Client::new();
        let url = format!("{}/records", &self.config.api_url);
        let res = client
            .post(url)
            .header("Auth-API-Token", &self.config.dns_token)
            .json(&new_record)
            .send()
            .await?
            .error_for_status()?;

        let created_dns = res.json::<RecordResponse>().await?;
        Ok(created_dns.record.id)
    }

    async fn delete_dns_record<'a>(&self, record_id: &'a str) -> Result<(), anyhow::Error> {
        let url = format!("{}/records/{}", &self.config.api_url, record_id);
        let client = Client::new();
        client
            .delete(url)
            .header("Auth-API-Token", &self.config.dns_token)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}
