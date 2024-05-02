use anyhow::Result;
use base64::prelude::{Engine as _, BASE64_STANDARD};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::HashMap,
    time::{Duration, Instant, SystemTime},
};
use tokio::time::sleep;

use crate::dns_trait::DnsType;

/// A DNS record based on the tags of a service in Consul
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash)]
pub struct DnsRecord {
    pub hostname: String,
    #[serde(rename = "type")]
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

#[derive(Debug, Deserialize)]
struct ConsulKVResponse {
    #[serde(rename = "Value")]
    value: Option<String>,
}

pub struct ConsulClient {
    pub http_client: reqwest::Client,
    pub kv_api_base_url: Url,
    pub catalog_api_base_url: Url,
    pub datacenter: Option<String>,
}

impl ConsulClient {
    pub fn new(consul_address: Url, consul_datacenter: Option<String>) -> Result<ConsulClient> {
        let kv_api_base_url = consul_address.join("v1/")?.join("kv/")?;
        let catalog_api_base_url = consul_address.join("v1/")?.join("catalog/")?;
        let client = reqwest::Client::builder().build()?;
        Ok(ConsulClient {
            http_client: client,
            kv_api_base_url,
            catalog_api_base_url,
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
    pub async fn fetch_service_tags(
        &self,
        consul_index: &mut Option<u64>,
    ) -> Result<Vec<DnsRecord>, anyhow::Error> {
        let mut services_url = self.catalog_api_base_url.join("services")?;

        // Set dc if it is provided in the config
        if let Some(dc) = &self.datacenter {
            services_url.query_pairs_mut().append_pair("dc", dc);
        }

        if let Some(index) = consul_index {
            services_url
                .query_pairs_mut()
                .append_pair("index", &index.to_string());
            services_url.query_pairs_mut().append_pair("wait", "100s");
        }

        // Add a filter to only match "normal" Consul services
        services_url.query_pairs_mut().append_pair(
            "filter",
            r#"ServiceKind == "" and ServiceTags contains "external-dns.enable=true""#,
        );

        let response = self.http_client.get(services_url).send().await?;

        if response.status().is_success() {
            *consul_index = response
                .headers()
                .get("X-Consul-Index")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse().ok());
        }

        let records = response
            .error_for_status()?
            .json::<HashMap<String, Vec<String>>>()
            .await?;

        let dns_tags = records
            .into_iter()
            .flat_map(|(_service_name, tags)| parse_dns_tags(tags))
            .collect();

        Ok(dns_tags)
    }

    /// Stores a single DNS record in Consul.
    /// This function fetches the current state of DNS records, updates it with the new record,
    /// and then re-stores the updated state back into Consul.
    pub async fn store_dns_record(
        &self,
        provider_record_id: String,
        dns_record: &DnsRecord,
    ) -> Result<()> {
        let mut records = self.fetch_all_dns_records().await?;
        records.insert(provider_record_id, dns_record.clone());
        self.store_all_dns_records(&records).await
    }

    /// Deletes a single DNS record from Consul.
    /// This function fetches the current DNS records, removes the specified record, and then updates
    /// the store in Consul.
    pub async fn delete_dns_record(&self, record_id: &str) -> Result<(), anyhow::Error> {
        let mut records = self.fetch_all_dns_records().await?;
        if records.remove(record_id).is_some() {
            self.store_all_dns_records(&records).await
        } else {
            Err(anyhow::anyhow!("Record not found"))
        }
    }

    // Store all DNS records under a single key as a HashMap
    async fn store_all_dns_records(&self, records: &HashMap<String, DnsRecord>) -> Result<()> {
        let url = self.kv_api_base_url.join("dns_records")?;
        let json_data = json!(records);
        self.http_client
            .put(url)
            .json(&json_data)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// Fetches all DNS records from Consul.
    /// This function retrieves the state of all DNS records stored under a specific Consul key.
    pub async fn fetch_all_dns_records(&self) -> Result<HashMap<String, DnsRecord>> {
        let url = self.kv_api_base_url.join("dns_records")?;
        let resp = self.http_client.get(url).send().await?;

        let mut records: HashMap<String, DnsRecord> = HashMap::new();
        if !resp.status().is_success() {
            eprintln!("Failed to fetch DNS records: {}", resp.status());
            if resp.status() == StatusCode::NOT_FOUND {
                return Ok(records);
            }
            return Err(anyhow::anyhow!(
                "Failed to fetch DNS records: {}",
                resp.status()
            ));
        }

        let body = resp.bytes().await?;

        let kv_response: Vec<ConsulKVResponse> = serde_json::from_slice(&body)
            .map_err(|e| anyhow::anyhow!("Failed to decode KV response: {}", e))?;

        for entry in kv_response {
            if let Some(encoded_value) = entry.value {
                let decoded_bytes = &BASE64_STANDARD
                    .decode(encoded_value)
                    .expect("Can't decode base64");

                // Deserialize the JSON string to a HashMap<String, DnsRecord>
                let record_map: HashMap<String, DnsRecord> = serde_json::from_slice(decoded_bytes)
                    .map_err(|e| anyhow::anyhow!("Failed to deserialize DnsRecord: {}", e))?;

                records.extend(record_map);
            }
        }

        Ok(records)
    }
}

fn parse_dns_tags(tags: Vec<String>) -> Vec<DnsRecord> {
    const PREFIX: &str = "external-dns.";
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
