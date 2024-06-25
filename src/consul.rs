use anyhow::{bail, Context, Result};
use base64::prelude::{Engine as _, BASE64_STANDARD};
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    future::Future,
    time::{Duration, SystemTime},
};
use tokio::{
    task::JoinHandle,
    time::{interval, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dns_trait::DnsType;

const CONSUL_STORE_KEY: &str = "dns_records_lock";

#[derive(Copy, Clone)]
enum SessionDuration {
    Seconds(u32),
}

impl TryFrom<Duration> for SessionDuration {
    type Error = anyhow::Error;

    fn try_from(value: Duration) -> Result<Self> {
        // Consul only supports durations of up to 86400 seconds.
        let secs = value.as_secs();
        if secs > 86400 {
            bail!("Tried to convert a duration longer than 24 hours into SessionDuration");
        }
        Ok(SessionDuration::Seconds(secs as u32))
    }
}

impl Serialize for SessionDuration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&match self {
            Self::Seconds(s) => format!("{s}s"),
        })
    }
}

#[derive(serde::Serialize)]
struct CreateSessionRequest {
    #[serde(rename = "Name")]
    name: &'static str,
    #[serde(rename = "Behavior")]
    behavior: &'static str,
    /// How long the session will survive without being renewed.
    #[serde(rename = "TTL")]
    ttl: SessionDuration,
    /// How long the locks held by this session should keep being held after the session
    /// has expired.
    #[serde(rename = "LockDelay")]
    lock_delay: &'static str,
}
#[derive(Deserialize, Debug)]
struct CreateSessionResponse {
    #[serde(rename = "ID")]
    id: Uuid,
}

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

#[derive(Debug, Deserialize)]
struct ConsulKVResponse {
    #[serde(rename = "Value")]
    value: Option<String>,
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Session")]
    session: Option<String>,
}

#[derive(Clone)]
pub struct ConsulClient {
    pub http_client: reqwest::Client,
    pub kv_api_base_url: Url,
    pub catalog_api_base_url: Url,
    pub session_api_base_url: Url,
    pub datacenter: Option<String>,
    pub kv_dns_records: HashMap<String, DnsRecord>,
}

impl ConsulClient {
    pub fn new(consul_address: Url, consul_datacenter: Option<String>) -> Result<ConsulClient> {
        let kv_api_base_url = consul_address.join("v1/")?.join("kv/")?;
        let catalog_api_base_url = consul_address.join("v1/")?.join("catalog/")?;
        let session_api_base_url = consul_address.join("v1/")?.join("session/")?;
        let client = reqwest::Client::builder().build()?;
        Ok(ConsulClient {
            http_client: client,
            kv_api_base_url,
            catalog_api_base_url,
            session_api_base_url,
            datacenter: consul_datacenter,
            kv_dns_records: HashMap::new(),
        })
    }

    /// Create a new session in Consul
    /// This session is used to acquire a lock
    pub async fn create_session(
        &self,
        ttl: Duration,
        token: CancellationToken,
    ) -> Result<ConsulSession, anyhow::Error> {
        let session_request = CreateSessionRequest {
            name: "external-dns",
            behavior: "release",
            ttl: ttl.try_into()?,
            lock_delay: "30s",
        };

        let session_url = self.session_api_base_url.join("create")?;

        let mut req = self.http_client.put(session_url).json(&session_request);

        // Set dc if it is provided in the config
        if let Some(dc) = &self.datacenter {
            req = req.query(&[("dc", dc)]);
        }

        let resp = req.send().await?.error_for_status()?;
        let session_response: CreateSessionResponse = resp.json().await?;

        let join_handle = tokio::spawn(
            session_handler(self.clone(), token, session_response.id, ttl)
                .context("failed to create Consul session handler")?,
        );

        Ok(ConsulSession {
            session_id: session_response.id,
            join_handle,
        })
    }

    /// Renew the Consul session
    pub async fn renew_session(&self, session_id: Uuid) -> Result<(), anyhow::Error> {
        let session_url = self
            .session_api_base_url
            .join(&format!("renew/{}", session_id))?;

        let mut req = self.http_client.put(session_url);

        // Set dc if it is provided in the config
        if let Some(dc) = &self.datacenter {
            req = req.query(&[("dc", dc)]);
        }

        req.send().await?.error_for_status()?;
        println!("Renewed Consul session: {}", session_id);
        Ok(())
    }

    /// Acquire a lock
    pub async fn acquire_lock(&self, session_id: Uuid) -> Result<()> {
        let mut interval = interval(Duration::from_secs(10));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            let lock_url = self.kv_api_base_url.join(CONSUL_STORE_KEY)?;

            let mut req = self.http_client.put(lock_url);
            req = req.query(&[("acquire", &session_id.to_string())]);

            // Set dc if it is provided in the config
            if let Some(dc) = &self.datacenter {
                req = req.query(&[("dc", dc)]);
            }

            let resp = req.send().await?;
            let body = resp.text().await?;

            // If the lock is acquired, the response body will be "true"
            if body.starts_with("true") {
                println!("=====> Acquired Consul lock");
                return Ok(());
            }

            println!("=====> Failed to acquire Consul lock");
            // We limit re-checks to at most every 10 seconds, so we don't spam the server in case we
            // didn't acquire the lock even though it claims it to be free.
            interval.tick().await;
            // Wait for lock to be free
            self.wait_for_lock().await?;
        }
    }

    async fn wait_for_lock(&self) -> Result<(), anyhow::Error> {
        let mut consul_index: Option<String> = None;
        loop {
            let lock_url = self.kv_api_base_url.join(CONSUL_STORE_KEY)?;

            let mut req = self.http_client.get(lock_url);

            // Set dc if it is provided in the config
            if let Some(dc) = &self.datacenter {
                req = req.query(&[("dc", dc)]);
            }

            if let Some(index) = consul_index.take() {
                req = req.query(&[("index", &index)]);
            }
            let response = req.send().await?;

            consul_index = response
                .headers()
                .get("X-Consul-Index")
                .and_then(|value| value.to_str().ok())
                .map(|value| value.to_string());

            let kvs = response.json::<Vec<ConsulKVResponse>>().await?;

            for kv in kvs {
                if kv.key == CONSUL_STORE_KEY && kv.session.is_none() {
                    println!("=====> lock is free, returning");
                    return Ok(());
                }
            }
        }
    }

    /// Retrieves a list of all registered services and parses their tags into DnsTag
    pub async fn fetch_service_tags(
        &self,
        consul_index: &mut Option<String>,
    ) -> Result<Vec<DnsRecord>, anyhow::Error> {
        let services_url = self.catalog_api_base_url.join("services")?;

        let mut req = self.http_client.get(services_url);

        // Set dc if it is provided in the config
        if let Some(dc) = &self.datacenter {
            req = req.query(&[("dc", dc)]);
        }

        if let Some(index) = consul_index {
            req = req.query(&[("index", &index.to_string())]);
        }

        // Add a filter to only match "normal" Consul services
        req = req.query(&[(
            "filter",
            r#"ServiceKind == "" and ServiceTags contains "external-dns.enable=true""#,
        )]);

        let response = req.send().await?.error_for_status()?;

        if let Some(index_header) = response.headers().get("X-Consul-Index") {
            if let Ok(index_str) = index_header.to_str() {
                let _ = consul_index.insert(index_str.to_string());
            } else {
                eprintln!("Failed to convert HeaderValue to string");
            }
        }

        let records = response.json::<HashMap<String, Vec<String>>>().await?;

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
        &mut self,
        provider_record_id: String,
        dns_record: DnsRecord,
    ) -> Result<(), anyhow::Error> {
        match self
            .kv_dns_records
            .insert(provider_record_id, dns_record.clone())
        {
            Some(_) => Err(anyhow::anyhow!("Unexpected record update")),
            None => Ok(()),
        }
    }

    /// Deletes a single DNS record from Consul.
    /// This function fetches the current DNS records, removes the specified record, and then updates
    /// the store in Consul.
    pub async fn delete_dns_record(&mut self, record_id: &str) -> Result<(), anyhow::Error> {
        if self.kv_dns_records.remove(record_id).is_some() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Record not found"))
        }
    }

    // Store all DNS records under a single key as a HashMap
    pub async fn store_all_dns_records(&self) -> Result<()> {
        let url = self.kv_api_base_url.join(CONSUL_STORE_KEY)?;

        let mut req = self.http_client.put(url).json(&self.kv_dns_records);

        // Set dc if it is provided in the config
        if let Some(dc) = &self.datacenter {
            req = req.query(&[("dc", dc)]);
        }

        req.send().await?.error_for_status()?;
        Ok(())
    }

    /// Fetches all DNS records from Consul.
    /// This function retrieves the state of all DNS records stored under a specific Consul key.
    pub async fn fetch_all_dns_records(&self) -> Result<HashMap<String, DnsRecord>, anyhow::Error> {
        let url = self.kv_api_base_url.join(CONSUL_STORE_KEY)?;

        let mut req = self.http_client.get(url);

        // Set dc if it is provided in the config
        if let Some(dc) = &self.datacenter {
            req = req.query(&[("dc", dc)]);
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            if resp.status() == StatusCode::NOT_FOUND {
                return Ok(HashMap::new());
            }
            return Err(anyhow::anyhow!(resp.status()));
        }

        let body = resp.bytes().await?;

        let kv_response: Vec<ConsulKVResponse> = serde_json::from_slice(&body)
            .map_err(|e| anyhow::anyhow!("Failed to decode KV response: {}", e))?;

        let mut records: HashMap<String, DnsRecord> = HashMap::new();
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

        let type_string = tags.remove("type");
        let type_: DnsType = match type_string.as_ref().map(|t| t.parse()) {
            None => {
                println!("Missing type for identifier: {}", identifier);
                continue;
            }
            Some(Ok(t)) => t,
            Some(Err(e)) => {
                eprintln!(
                    "Unsupported record type {} specified for identifier {}: {}",
                    type_string.unwrap_or_default(),
                    identifier,
                    e
                );
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

pub struct ConsulSession {
    pub session_id: Uuid,
    pub join_handle: JoinHandle<()>,
}

fn session_handler(
    client: ConsulClient,
    token: CancellationToken,
    id: Uuid,
    ttl: Duration,
) -> Result<impl Future<Output = ()> + Send> {
    let id = id.to_string();

    let renewal_url = client.session_api_base_url.join(&format!("renew/{}", id))?;
    let destroy_url = client
        .session_api_base_url
        .join(&format!("destroy/{}", id))?;

    Ok(async move {
        // Renew the session at 2 times the TTL.
        let mut interval = interval(ttl / 2);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            // Wait for either cancellation or an interval tick to have passed.
            tokio::select! {
                _ = token.cancelled() => {
                    println!("Consul session handler was cancelled");
                    break;
                },
                _ = interval.tick() => {},
            };

            println!("Renewing Consul session");

            let mut req = client.http_client.put(renewal_url.clone());

            // Set dc if it is provided in the config
            if let Some(dc) = &client.datacenter {
                req = req.query(&[("dc", dc)]);
            }

            let res = req.send().await.and_then(|res| res.error_for_status());
            if let Err(err) = res {
                eprintln!("Renewing Consul session failed, aborting: {err}");
                token.cancel();
                return;
            }
        }

        println!("Destroying Consul session");
        let mut req = client.http_client.put(destroy_url);

        // Set dc if it is provided in the config
        if let Some(dc) = &client.datacenter {
            req = req.query(&[("dc", dc)]);
        }
        let res = req.send().await.and_then(|res| res.error_for_status());
        if let Err(err) = res {
            eprintln!("Destraying Consul session failed: {err}");
        }
    })
}
