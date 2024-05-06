use std::fmt::Display;

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

// implement Display for DnsType
impl Display for DnsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DnsType::A => write!(f, "A"),
            DnsType::AAAA => write!(f, "AAAA"),
            DnsType::CNAME => write!(f, "CNAME"),
        }
    }
}

#[async_trait]
pub trait DnsProviderTrait {
    async fn create_dns_record<'a>(
        &self,
        dns_record: &'a consul::DnsRecord,
    ) -> Result<String, anyhow::Error>;

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
