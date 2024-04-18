use mockito::ServerGuard;

pub async fn mock_get_nomad_service_by_name(server: &mut ServerGuard) -> mockito::Mock {
    server
        .mock("GET", mockito::Matcher::Any)
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            r#"
            [
              {
                "Address": "127.0.0.1",
                "AllocID": "177160af-26f6-619f-9c9f-5e46d1104395",
                "CreateIndex": 14,
                "Datacenter": "dc1",
                "ID": "_nomad-task-177160af-26f6-619f-9c9f-5e46d1104395-redis-example-cache-redis-db",
                "JobID": "example",
                "ModifyIndex": 24,
                "Namespace": "default",
                "NodeID": "7406e90b-de16-d118-80fe-60d0f2730cb3",
                "Port": 29702,
                "ServiceName": "example-cache-redis",
                "Tags": [
                  "external-dns.hostname=redis.example.com",
                  "external-dns.ttl=300",
                  "external-dns.type=A",
                  "external-dns.value=10.10.10.10"
                ]
              }
            ]
            "#
        )
        .create_async()
        .await
}
