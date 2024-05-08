use std::time::Duration;

use clap::Parser;

use consul_external_dns::hetzner_dns;
use tokio::time::{interval, sleep, MissedTickBehavior};

use consul_external_dns::config::{Config, DnsProvider};
use consul_external_dns::consul::ConsulClient;
use consul_external_dns::dns_trait::DnsProviderTrait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let cancel = CancellationToken::new();
    println!("Starting up Consul External DNS");

    println!("=> Parsing configuration");
    let config = Config::try_parse().expect("===> failed to parse configuration");
    println!("===> parsed successfully");

    let dns_provider: Box<dyn DnsProviderTrait> = match config.clone().dns_provider {
        DnsProvider::Hetzner(config) => Box::new(hetzner_dns::HetznerDns { config }),
    };

    // Initialize Consul Client
    println!("=> Creating Consul client");
    let mut consul_client = create_consul_client(&config).await;
    println!("===> created Consul client successfully");

    // Create Consul session
    println!("=> Creating Consul session");
    let session_id = consul_client
        .create_session()
        .await
        .expect("===> failed to create consul session");
    println!("===> created Consul session successfully");

    // Spawn a task to renew the session periodically
    {
        tokio::spawn(renew_session_periodically(
            consul_client.clone(),
            session_id,
            cancel.clone(),
        ));
    }

    // Acquire Lock
    println!("=> Acquiring Consul lock");
    if let Err(e) = consul_client.acquire_lock(session_id).await {
        eprintln!("===> failed to acquire Consul lock: {}", e);
        return;
    }
    println!("===> acquired Consul lock successfully");

    // Read dns records from consul store and save the state the consul_client.kv_dns_records
    let dns_records_state = match consul_client.fetch_all_dns_records().await {
        Ok(records) => records,
        Err(e) => {
            eprintln!("===> failed to fetch Consul DNS records: {}", e);
            return;
        }
    };
    consul_client.kv_dns_records = dns_records_state;

    process_dns_records(consul_client, dns_provider, cancel).await;
}

async fn create_consul_client(config: &Config) -> ConsulClient {
    loop {
        match ConsulClient::new(
            config.consul_address.clone(),
            config.consul_datacenter.clone(),
        ) {
            Ok(client) => return client,
            Err(e) => {
                eprintln!("===> failed to create Consul client: {}", e);
                sleep(Duration::from_millis(100)).await;
            }
        }
    }
}

async fn renew_session_periodically(
    consul_client: ConsulClient,
    session_id: Uuid,
    cancel_token: CancellationToken,
) {
    let mut interval = interval(Duration::from_secs(15));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        println!("=> Renewing session {session_id}");
        match consul_client.renew_session(session_id).await {
            Ok(_) => println!("===> renewed session"),
            Err(err) => {
                println!("===> renewing session failed, shutting down: {err}");
                cancel_token.cancel();
                break;
            }
        }
    }
}

async fn process_dns_records(
    mut consul_client: ConsulClient,
    dns_provider: Box<dyn DnsProviderTrait>,
    cancel_token: CancellationToken,
) {
    let mut consul_dns_index: Option<String> = None;
    loop {
        // Fetch current DNS records from Consul
        println!("=> Fetching DNS records from Consul store");
        let dns_state = match consul_client.fetch_all_dns_records().await {
            Ok(records) => records,
            Err(e) => {
                eprintln!("===> failed to fetch Consul DNS records: {}", e);
                return;
            }
        };
        println!(
            "===> fetched Consul DNS records successfully with total records: {}",
            dns_state.len()
        );

        // Fetch DNS tags from the services in Consul
        // This is the long polling request that will block until there are changes
        // in the Consul Services. The timeout is set to 100 seconds.
        println!("=> Fetching DNS tags from Consul Services");
        let fetched_dns_records = match consul_client
            .fetch_service_tags(&mut consul_dns_index)
            .await
        {
            Ok(tags) => tags,
            Err(e) => {
                eprintln!("===> failed to fetch Consul DNS tags: {}", e);
                return;
            }
        };
        println!(
            "===> fetched Consul DNS tags successfully, total tags: {}",
            fetched_dns_records.len()
        );

        println!("The services in Consul have changed now; DNS records in the DNS provider need to be updated.");
        let mut all_success = true;

        println!("=> Creating DNS records in the DNS provider");
        for fetched_dns_record in &fetched_dns_records {
            if !dns_state.values().any(|r| r == fetched_dns_record) {
                // Create the record on the DNS provider
                let record_id = match dns_provider.create_dns_record(fetched_dns_record).await {
                    Ok(record_id) => record_id,
                    Err(e) => {
                        eprintln!("===> failed to create DNS record: {}", e);
                        all_success = false;
                        continue;
                    }
                };
                println!("===> DNS record created in DNS provider");

                // Store the record in Consul
                match consul_client
                    .store_dns_record(record_id, fetched_dns_record.clone())
                    .await
                {
                    Ok(_) => println!("===> DNS record stored in Consul"),
                    Err(e) => eprintln!("===> failed to store DNS record in Consul: {}", e),
                }
            }
        }

        // Delete DNS records from Consul state that are not in the fetched_dns_records
        println!("=> Deleting DNS records from the DNS provider");
        for (record_id, record) in dns_state.iter() {
            if !fetched_dns_records
                .iter()
                .any(|fetched_record| fetched_record == record)
            {
                // Delete the record from the DNS provider
                if let Err(e) = dns_provider.delete_dns_record(record_id).await {
                    eprintln!("===> failed to delete DNS record: {}", e);
                    all_success = false;
                    continue;
                };
                println!("===> DNS record deleted from DNS provider");

                // Delete the record from Consul state
                match consul_client.delete_dns_record(record_id).await {
                    Ok(_) => println!("===> DNS record deleted from Consul"),
                    Err(e) => eprintln!("===> failed to delete DNS record from Consul: {}", e),
                }
            }
        }

        println!("=> Storing all DNS records in Consul KV store");
        if all_success {
            match consul_client.store_all_dns_records().await {
                Ok(()) => println!("===> stored all DNS records in Consul"),
                Err(e) => {
                    eprintln!("===> failed to store all DNS records in Consul: {}", e);
                    all_success = false;
                }
            }

            if all_success {
                println!("All DNS updates or creations succeeded.");
            } else {
                eprintln!("Some DNS updates or creations failed.");
            }

            tokio::select! {
                _ = cancel_token.cancelled() => {
                    println!("Exiting Consul External DNS, because the cancel token was triggered.");
                    break;
                }
                _ = sleep(Duration::from_secs(1)) => {},
            };
        }
    }
}
