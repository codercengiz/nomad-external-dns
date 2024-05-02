use std::time::Duration;

use clap::Parser;

use consul_external_dns::hetzner_dns;
use reqwest::Url;
use tokio::time::sleep;

use consul_external_dns::config::{Config, DnsProvider};
use consul_external_dns::consul::ConsulClient;
use consul_external_dns::dns_trait::DnsProviderTrait;

#[tokio::main]
async fn main() {
    println!("Starting up Consul External DNS");

    let config = match Config::try_parse() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to parse configuration: {}", e);
            return;
        }
    };
    println!("Configuration parsed successfully");

    let dns_provider: Box<dyn DnsProviderTrait> = match config.dns_provider {
        DnsProvider::Hetzner(config) => Box::new(hetzner_dns::HetznerDns { config }),
    };

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
    println!("Consul client created successfully");

    let mut consul_index: Option<u64> = None;
    loop {
        // Acquire Lock
        if let Err(e) = consul_client.acquire_lock().await {
            eprintln!("Failed to acquire Consul lock: {}", e);
            return;
        }
        println!("Consul lock acquired successfully");

        // Fetch current DNS records from Consul
        let dns_state = match consul_client.fetch_all_dns_records().await {
            Ok(records) => records,
            Err(e) => {
                eprintln!("Failed to fetch Consul DNS records: {}", e);

                return;
            }
        };

        // Fetch DNS tags from the services in Consul
        println!("Fetching DNS tags from the services in Consul");
        let fetched_dns_records = match consul_client.fetch_service_tags(&mut consul_index).await {
            Ok(tags) => tags,
            Err(e) => {
                eprintln!("Failed to fetch Consul DNS tags: {}", e);
                return;
            }
        };
        println!(
            "Consul DNS tags fetched successfully. Length: {}",
            fetched_dns_records.len()
        );

        let mut all_success = true;

        for fetched_dns_record in &fetched_dns_records {
            if !dns_state.values().any(|r| r == fetched_dns_record) {
                // Create the record on the DNS provider
                let record = match dns_provider
                    .update_or_create_dns_record(fetched_dns_record)
                    .await
                {
                    Ok(record) => record,
                    Err(e) => {
                        eprintln!("Failed to update or create DNS record: {}", e);
                        all_success = false;
                        continue;
                    }
                };

                // Store the record in Consul
                match consul_client
                    .store_dns_record(record.id, fetched_dns_record)
                    .await
                {
                    Ok(_) => println!("DNS record stored in Consul"),
                    Err(e) => eprintln!("Failed to store DNS record in Consul: {}", e),
                }
            }
        }

        // Delete DNS records from Consul state that are not in the fetched_dns_records
        for (record_id, record) in dns_state.iter() {
            if !fetched_dns_records
                .iter()
                .any(|fetched_record| fetched_record == record)
            {
                // Delete the record from the DNS provider
                match dns_provider.delete_dns_record(record_id).await {
                    Ok(_) => (),
                    Err(e) => {
                        eprintln!("Failed to delete DNS record: {}", e);
                        all_success = false;
                        continue;
                    }
                };

                // Delete the record from Consul state
                match consul_client.delete_dns_record(record_id).await {
                    Ok(_) => println!("DNS record deleted from Consul"),
                    Err(e) => eprintln!("Failed to delete DNS record from Consul: {}", e),
                }
            }
        }

        // Release Lock
        if consul_client.drop_lock().await.is_err() {
            eprintln!("Failed to release Consul lock");
        }

        if all_success {
            println!("Successfully updated or created all DNS records.");
        } else {
            eprintln!("Some DNS updates or creations failed.");
        }
    }
}
