use mockito::{Matcher, ServerGuard};

pub async fn mock_create_dns_record(server: &mut ServerGuard) -> mockito::Mock {
    print!("Mocking create DNS record");
    server
        .mock("POST", "/records")
        .match_header("Auth-API-Token", Matcher::Any)
        .match_body(Matcher::Any)
        .with_status(201)
        .with_body(r#"{"id":"new_dns_record_id","message":"DNS record created"}"#)
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
