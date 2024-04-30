use anyhow::Result;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant, SystemTime},
};
use tokio::time::sleep;

use crate::dns_trait::DnsType;

/// A DNS record based on the tags of a service in Consul
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    pub hostname: String,
    pub type_: DnsType,
    pub ttl: Option<i32>,
    pub value: String,
}

#[derive(Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
struct ConsulLock {
    pub locked_at: SystemTime,
}

impl ConsulLock {
    fn new() -> Self {
        Self {
            locked_at: SystemTime::now(),
        }
    }
}

pub struct ConsulClient {
    pub http_client: reqwest::Client,
    pub kv_api_base_url: Url,
    pub datacenter: Option<String>,
}

impl ConsulClient {
    pub fn new(consul_address: Url, consul_datacenter: Option<String>) -> Result<ConsulClient> {
        let kv_api_base_url = consul_address.join("v1/")?.join("kv/")?;
        let client = reqwest::Client::builder().build()?;
        Ok(ConsulClient {
            http_client: client,
            kv_api_base_url,
            datacenter: consul_datacenter,
        })
    }

    /// Acquire a lock
    ///
    /// Times out after a while and returns an error if it does.
    pub async fn acquire_lock(&self) -> Result<()> {
        let consul_lock = ConsulLock::new();
        let wait_time = Instant::now();
        let timeout = Duration::from_secs(10);

        loop {
            if wait_time.elapsed() > timeout {
                println!("Timed out trying to acquire lock");
                println!("Assuming poisoned lock, deleting last lock");
                self.drop_lock().await?;
            }

            let mut lock_url = self.kv_api_base_url.join("consul_lock")?;

            // Append 'cas=0' to ensure the lock is acquired only if the key does not already exist.
            lock_url.query_pairs_mut().append_pair("cas", "0");

            // Set dc if it is provided in the config
            if let Some(dc) = &self.datacenter {
                lock_url.query_pairs_mut().append_pair("dc", dc);
            }

            let resp = self
                .http_client
                .put(lock_url)
                .json(&consul_lock)
                .send()
                .await?
                .error_for_status()?;
            let body = resp.text().await?;

            // If the lock is acquired, the response body will be "true"
            if body.starts_with("true") {
                println!("Acquired Consul lock");
                return Ok(());
            }
            sleep(Duration::from_millis(100)).await;
        }
    }

    /// Drop a lock
    pub async fn drop_lock(&self) -> Result<()> {
        let mut lock_url = self.kv_api_base_url.join("consul_lock")?;
        if let Some(dc) = &self.datacenter {
            lock_url.query_pairs_mut().append_pair("dc", dc);
        }
        self.http_client
            .delete(lock_url)
            .send()
            .await?
            .error_for_status()?;
        println!("Dropped Consul lock");
        Ok(())
    }

    /// Retrieves a list of all registered services and parses their tags into DnsTag
    pub async fn fetch_service_tags(&self) -> Result<Vec<DnsRecord>, anyhow::Error> {
        let mut services_url = self.kv_api_base_url.join("catalog/")?.join("services")?;

        // Set dc if it is provided in the config
        if let Some(dc) = &self.datacenter {
            services_url.query_pairs_mut().append_pair("dc", dc);
        }

        // Add a filter to only match "normal" Consul services
        services_url.query_pairs_mut().append_pair(
            "filter",
            r#"ServiceKind == "" and ServiceTags contains "external-dns.enable=true""#,
        );

        let response = self
            .http_client
            .get(services_url)
            .send()
            .await?
            .error_for_status()?
            .json::<HashMap<String, Vec<String>>>()
            .await?;

        let dns_tags = response
            .into_iter()
            .flat_map(|(_service_name, tags)| parse_dns_tags(tags))
            .collect();

        Ok(dns_tags)
    }
}

fn parse_dns_tags(tags: Vec<String>) -> Vec<DnsRecord> {
    const PREFIX: &'static str = "external-dns.";
    // Parse service tags of the format `external-dns.<field>=<value>`.
    let mut dns_tags: HashMap<String, HashMap<String, String>> = HashMap::new();
    for tag in tags.into_iter() {
        let Some(rest) = tag.strip_prefix(PREFIX) else {
            continue;
        };
        let Some((identifier, rest)) = rest.split_once('.') else {
            continue;
        };
        let Some((field, value)) = rest.split_once('=') else {
            continue;
        };
        dns_tags
            .entry(identifier.to_string())
            .or_default()
            .insert(field.to_string(), value.to_string());
    }

    let mut records = Vec::new();
    for (identifier, mut tags) in dns_tags {
        let Some(hostname) = tags.remove("hostname") else {
            println!("Missing hostname for identifier: {}", identifier);
            continue;
        };
        let type_: DnsType = match tags.remove("type").map(|t| t.parse()) {
            None => {
                println!("Missing type for identifier: {}", identifier);
                continue;
            }
            Some(Ok(t)) => t,
            Some(Err(e)) => {
                eprintln!("Failed to parse type for identifier {}: {}", identifier, e);
                continue;
            }
        };

        let ttl = match tags.remove("ttl").map(|t| t.parse()) {
            None => None,
            Some(Ok(ttl)) => Some(ttl),
            Some(Err(e)) => {
                eprintln!("Failed to parse TTL for identifier {}: {}", identifier, e);
                continue;
            }
        };
        let Some(value) = tags.remove("value") else {
            println!("Missing value for identifier: {}", identifier);
            continue;
        };

        records.push(DnsRecord {
            hostname,
            type_,
            ttl,
            value,
        });
    }

    records
}
