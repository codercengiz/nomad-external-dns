use consul_external_dns::dns_trait::DnsRecord;
use mockito::{Matcher, ServerGuard};

pub async fn mock_create_dns_record(
    server: &mut ServerGuard,
    create_dns_record: &DnsRecord,
) -> mockito::Mock {
    print!("Mocking create DNS record");

    let matcher_body = Matcher::JsonString(get_matcher_body(create_dns_record));

    let expected_body = get_expected_body(create_dns_record);

    server
        .mock("POST", "/records")
        .match_header("Auth-API-Token", Matcher::Any)
        .match_body(matcher_body)
        .with_status(201)
        .with_body(expected_body)
        .create_async()
        .await
}

/// Mocks the GET request to retrieve DNS records for a specific zone in Hetzner's API.
pub async fn mock_get_dns_records(
    server: &mut ServerGuard,
    zone_id: &str,
    api_token: &str,
) -> mockito::Mock {
    server
        .mock("GET", mockito::Matcher::Any)
        .match_query(Matcher::UrlEncoded("zone_id".into(), zone_id.into()))
        .match_header("Auth-API-Token", api_token)
        .with_status(200)
        .with_body(
            r#"{"records": [{"type": "A","id": "string","zone_id": "string","name": "string","value": "string"}]}"#,
        )
        .create_async()
        .await
}

fn get_expected_body(dns_record: &DnsRecord) -> String {
    match dns_record.ttl {
        Some(ttl) => {
            format!(
                r#"{{"record":{{"type":"{}","id":"{}","created":"","modified":"","zone_id":"string","name":"{}","value":"{}","ttl":{}}}}}"#,
                dns_record.type_, dns_record.id, dns_record.name, dns_record.value, ttl
            )
        }
        None => {
            format!(
                r#"{{"record":{{"type":"{}","id":"{}","created":"","modified":"","zone_id":"string","name":"{}","value":"{}", "ttl":0}}}}"#,
                dns_record.type_, dns_record.id, dns_record.name, dns_record.value
            )
        }
    }
}

fn get_matcher_body(dns_record: &DnsRecord) -> String {
    match dns_record.ttl {
        Some(ttl) => {
            format!(
                r#"{{"type":"{}","zone_id":"{}","name":"{}","value":"{}","ttl":{}}}"#,
                dns_record.type_, dns_record.zone_id, dns_record.name, dns_record.value, ttl
            )
        }
        None => {
            format!(
                r#"{{"type":"{}","zone_id":"{}","name":"{}","value":"{}","ttl":null}}"#,
                dns_record.type_, dns_record.zone_id, dns_record.name, dns_record.value
            )
        }
    }
}
