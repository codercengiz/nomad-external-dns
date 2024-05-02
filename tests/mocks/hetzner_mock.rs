use consul_external_dns::dns_trait::DnsRecordCreate;
use mockito::{Matcher, ServerGuard};

pub async fn mock_create_dns_record(
    server: &mut ServerGuard,
    create_dns_record: &DnsRecordCreate,
    create_dns_record_id: &str,
) -> mockito::Mock {
    print!("Mocking create DNS record");

    let matcher_body = Matcher::JsonString(serde_json::to_string(&create_dns_record).unwrap());

    let expected_body = format!(
        r#"{{"record":{{"type":"{}","id":"{}","created":"","modified":"","zone_id":"string","name":"{}","value":"{}","ttl":{}}}}}"#,
        create_dns_record.type_,
        create_dns_record_id,
        create_dns_record.name,
        create_dns_record.value,
        create_dns_record.ttl.unwrap_or(0)
    );

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
