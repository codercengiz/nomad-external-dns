use std::time::{Duration, Instant, SystemTime};
use anyhow::Result;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

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
}

impl ConsulClient {
    pub fn new(consul_address: Url) -> Result<ConsulClient> {
        let kv_api_base_url = consul_address.join("v1/")?.join("kv/")?;
        let client = reqwest::Client::builder().build()?;
        Ok(ConsulClient {
            http_client: client,
            kv_api_base_url,
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
        let lock_url = self.kv_api_base_url.join("consul_lock")?;
        self.http_client
            .delete(lock_url)
            .send()
            .await?
            .error_for_status()?;
        println!("Dropped Consul lock");
        Ok(())
    }
}
