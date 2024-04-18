mod mocks;

#[cfg(test)]
mod tests {
    use mockito::Server;
    use nomad_external_dns::config::HetznerConfig;
    use nomad_external_dns::dns_trait::DnsProviderTrait;
    use nomad_external_dns::hetzner_dns::HetznerDns;
    use nomad_external_dns::nomad::NomadDnsTag;

    use crate::mocks::hetzner_mock;
    #[tokio::test]
    async fn test_create_non_existing_dns_record() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let create_mock = hetzner_mock::mock_create_dns_record(&mut server).await;
        hetzner_mock::mock_get_dns_records(&mut server, "fake_zone_id", "fake_token").await;

        let config = HetznerConfig {
            dns_token: "fake_token".to_string(),
            dns_zone_id: "fake_zone_id".to_string(),
            api_url: url,
        };
        let hetzner_dns = HetznerDns { config };
        let nomad_tag = NomadDnsTag {
            hostname: "new.example.com".to_string(),
            type_: "A".to_string(),
            value: "192.168.0.1".to_string(),
            ttl: Some(300),
        };

        let result = hetzner_dns.update_or_create_dns_record(&nomad_tag).await;
        create_mock.assert();
        assert!(result.is_ok());
    }
}
