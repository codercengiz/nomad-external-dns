use anyhow::Error;
use dns::{DnsRecord, DnsRecordCreate};

use crate::nomad::NomadDnsTag;

mod config;
mod dns;
mod nomad;

#[tokio::main]
async fn main() {
    let config = config::parse_args();
    print!("Config: {:?}", config);

    let nomad_tag = match nomad::fetch_and_parse_service_tags(&config).await {
        Ok(tag) => tag,
        Err(e) => {
            eprintln!("Failed to fetch Nomad DNS tags: {}", e);
            return;
        }
    };

    let existing_records =
        match dns::list_dns_records(&config.hetzner_dns_token, &config.hetzner_dns_zone_id).await {
            Ok(records) => records,
            Err(e) => {
                eprintln!("Failed to list DNS records: {}", e);
                return;
            }
        };

    let update_or_create_result =
        update_or_create_dns_record(&config, &nomad_tag, &existing_records).await;

    match update_or_create_result {
        Ok(_) => println!("DNS record updated or created successfully"),
        Err(e) => eprintln!("Failed to update or create DNS record: {}", e),
    }
}

async fn update_or_create_dns_record(
    config: &config::Config,
    nomad_tag: &NomadDnsTag,
    existing_records: &[DnsRecord],
) -> Result<(), Error> {
    let matched_record = existing_records
        .iter()
        .find(|record| record.name == nomad_tag.hostname && record.type_ == nomad_tag.type_);

    match matched_record {
        Some(record) => {
            if record.value != nomad_tag.value || record.ttl != nomad_tag.ttl {
                // Update the existing record
                let updated_record = DnsRecord {
                    id: record.id.clone(),
                    zone_id: record.zone_id.clone(),
                    type_: nomad_tag.type_.clone(),
                    name: nomad_tag.hostname.clone(),
                    value: nomad_tag.value.clone(),
                    ttl: nomad_tag.ttl.clone(),
                };
                dns::update_dns_record(&config.hetzner_dns_token, &updated_record).await?;
                Ok(())
            } else {
                Ok(())
            }
        }
        None => {
            // Create a new DNS record
            let new_record = DnsRecordCreate {
                zone_id: config.hetzner_dns_zone_id.clone(),
                type_: nomad_tag.type_.clone(),
                name: nomad_tag.hostname.clone(),
                value: nomad_tag.value.clone(),
                ttl: nomad_tag.ttl.clone(),
            };
            dns::create_dns_record(&config.hetzner_dns_token, &new_record).await?;
            Ok(())
        }
    }
}
