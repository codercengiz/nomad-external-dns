use std::time::Duration;

use clap::Parser;

use reqwest::Url;
use tokio::time::sleep;

use crate::{
    config::{Config, DnsProvider},
    consul::ConsulClient,
    dns_trait::DnsProviderTrait,
};

mod config;
mod consul;
mod dns_trait;
mod hetzner_dns;
mod nomad;

#[tokio::main]
async fn main() {
    let config = Config::parse();

    // Initialize Consul Client
    let consul_client = loop {
        match ConsulClient::new(
            Url::parse(&config.consul_address).expect("Invalid URL"),
            config.consul_datacenter.clone(),
        ) {
            Ok(client) => break client,
            Err(e) => {
                eprintln!("Failed to create Consul client: {}. Retrying in 100ms", e);
                sleep(Duration::from_millis(100)).await;
            }
        }
    };

    // Acquire Lock
    if let Err(e) = consul_client.acquire_lock().await {
        eprintln!("Failed to acquire Consul lock: {}", e);
        return;
    }

    let nomad_tag = match nomad::fetch_and_parse_service_tags(&config).await {
        Ok(tag) => tag,
        Err(e) => {
            eprintln!("Failed to fetch Nomad DNS tags: {}", e);
            return;
        }
    };

    let dns_provider: Box<dyn DnsProviderTrait> = match config.dns_provider {
        DnsProvider::Hetzner(config) => Box::new(hetzner_dns::HetznerDns { config }),
    };

    let result = dns_provider.update_or_create_dns_record(&nomad_tag).await;

    // Release Lock
    if consul_client.drop_lock().await.is_err() {
        eprintln!("Failed to release Consul lock");
    }

    match result {
        Ok(_) => println!("DNS record updated or created successfully"),
        Err(e) => eprintln!("Failed to update or create DNS record: {}", e),
    }
}
