use async_trait::async_trait;
use reqwest::{header, Client, Error};

use crate::{
    config::HetznerConfig,
    dns_trait::{DnsProviderTrait, DnsRecord, DnsRecordCreate},
    nomad::NomadDnsTag,
};

const HETZNER_API_URL: &str = "https://dns.hetzner.com/api/v1";

pub struct HetznerDns {
    pub config: HetznerConfig,
}

#[async_trait]
impl DnsProviderTrait for HetznerDns {
    /// Update or create a DNS record based on the NomadDnsTag
    /// If the record already exists, it will be updated if the value or ttl is different
    /// If the record does not exist, it will be created
    async fn update_or_create_dns_record<'a>(
        &self,
        nomad_tag: &'a NomadDnsTag,
    ) -> Result<(), anyhow::Error> {
        let api_token = &self.config.dns_token;
        let zone_id = &self.config.dns_zone_id;

        let existing_records = match list_dns_records(api_token, zone_id).await {
            Ok(records) => records,
            Err(e) => {
                eprintln!("Failed to list DNS records: {}", e);
                return Err(e.into());
            }
        };

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
                        ttl: nomad_tag.ttl,
                    };
                    update_dns_record(api_token, &updated_record).await?;
                    Ok(())
                } else {
                    Ok(())
                }
            }
            None => {
                // Create a new DNS record
                let new_record = DnsRecordCreate {
                    zone_id: zone_id.clone(),
                    type_: nomad_tag.type_.clone(),
                    name: nomad_tag.hostname.clone(),
                    value: nomad_tag.value.clone(),
                    ttl: nomad_tag.ttl,
                };
                create_dns_record(api_token, &new_record).await?;
                Ok(())
            }
        }
    }
}

async fn list_dns_records(api_token: &str, zone_id: &str) -> Result<Vec<DnsRecord>, Error> {
    let client = Client::new();
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Auth-API-Token",
        header::HeaderValue::from_str(api_token).unwrap(),
    );

    let url = format!("{}/records?zone_id={}", HETZNER_API_URL, zone_id);
    let response = client.get(url).headers(headers).send().await?;

    match response.error_for_status() {
        Ok(res) => {
            let records = res.json::<Vec<DnsRecord>>().await?;
            Ok(records)
        }
        Err(err) => Err(err),
    }
}

async fn update_dns_record(api_token: &str, record: &DnsRecord) -> Result<(), Error> {
    let client = Client::new();
    let url = format!("{}/records/{}", HETZNER_API_URL, record.id);
    let res = client
        .put(url)
        .header("Auth-API-Token", api_token)
        .json(record)
        .send()
        .await?;

    res.error_for_status()?;
    Ok(())
}

async fn create_dns_record(api_token: &str, record_create: &DnsRecordCreate) -> Result<(), Error> {
    let client = Client::new();
    let url = format!("{}/records", HETZNER_API_URL);
    let res = client
        .post(url)
        .header("Auth-API-Token", api_token)
        .json(record_create)
        .send()
        .await?;

    res.error_for_status()?;
    Ok(())
}
