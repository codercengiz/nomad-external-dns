use mockito::ServerGuard;

pub async fn mock_get_consul_services(server: &mut ServerGuard) -> mockito::Mock {
    println!("Mocking consul services");
    server
        .mock("GET", mockito::Matcher::Any)
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            r#"
            {
                "consul": [],
                "redis": [],
                "app-rev-mr1": [
                    "external-dns.id1.hostname=example.com",
                    "external-dns.id1.type=A",
                    "external-dns.id1.value=192.168.1.100",
                    "external-dns.id1.ttl=300",
                    "external-dns.id1.enable=true"
                ],
                "app-rev-mr2": [
                    "external-dns.id2.hostname=example.com",
                    "external-dns.id2.type=A",
                    "external-dns.id2.value=192.168.1.101",
                    "external-dns.id2.ttl=300",
                    "external-dns.id2.enable=true"
                ],
                "app-rev-mr3": [
                    "external-dns.id3.hostname=example.com",
                    "external-dns.id3.type=A",
                    "external-dns.id3.value=192.168.1.102",
                    "external-dns.id3.ttl=300",
                    "external-dns.id3.enable=false"
                ],
                "app-rev-mr4": [
                    "external-dns.id4.hostname=example.com",
                    "external-dns.id4.type=A",
                    "external-dns.id4.value=192.168.1.103",
                    "external-dns.id4.ttl=300",
                    "external-dns.id4.enable=true"
                ]
            }"#,
        )
        .create_async()
        .await
}
