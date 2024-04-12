use reqwest::{header, Client, Error};
use serde::{Deserialize, Serialize};

const HETZNER_API_URL: &str = "https://dns.hetzner.com/api/v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    pub id: String,
    pub zone_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub value: String,
    pub ttl: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DnsRecordCreate {
    pub zone_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub value: String,
    pub ttl: Option<i32>,
}

pub async fn list_dns_records(api_token: &str, zone_id: &str) -> Result<Vec<DnsRecord>, Error> {
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
        Err(err) => Err(err.into()),
    }
}

pub async fn update_dns_record(api_token: &str, record: &DnsRecord) -> Result<(), Error> {
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

pub async fn create_dns_record(
    api_token: &str,
    record_create: &DnsRecordCreate,
) -> Result<(), Error> {
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
