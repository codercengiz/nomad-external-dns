use crate::nomad::NomadDnsTag;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait DnsProviderTrait {
    async fn update_or_create_dns_record<'a>(
        &self,
        nomad_tag: &'a NomadDnsTag,
    ) -> Result<(), anyhow::Error>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    pub id: String,
    pub zone_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub value: String,
    pub ttl: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DnsRecordCreate {
    pub zone_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub value: String,
    pub ttl: Option<i32>,
}
