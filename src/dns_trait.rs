use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::consul;

// convert dnstag type to an enum
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash)]
pub enum DnsType {
    A,
    AAAA,
    CNAME,
}

// implement FromStr for DnsType
impl std::str::FromStr for DnsType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" => Ok(DnsType::A),
            "AAAA" => Ok(DnsType::AAAA),
            "CNAME" => Ok(DnsType::CNAME),
            _ => Err(format!("Invalid DNS type: {}", s)),
        }
    }
}

#[async_trait]
pub trait DnsProviderTrait {
    async fn update_or_create_dns_record<'a>(
        &self,
        dns_record: &'a consul::DnsRecord,
    ) -> Result<DnsRecord, anyhow::Error>;

    async fn delete_dns_record<'a>(&self, record_id: &'a str) -> Result<(), anyhow::Error>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    pub id: String,
    pub zone_id: String,
    #[serde(rename = "type")]
    pub type_: DnsType,
    pub name: String,
    pub value: String,
    pub ttl: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DnsRecordCreate {
    pub zone_id: String,
    #[serde(rename = "type")]
    pub type_: DnsType,
    pub name: String,
    pub value: String,
    pub ttl: Option<i32>,
}
