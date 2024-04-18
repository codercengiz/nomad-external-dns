mod mocks;

#[cfg(test)]
mod tests {

    use mockito::Server;
    use nomad_external_dns::config::{Config, DnsProvider};
    use nomad_external_dns::dns_trait::DnsProviderTrait;
    use nomad_external_dns::hetzner_dns::HetznerDns;
    use nomad_external_dns::nomad::NomadDnsTag;
    use nomad_external_dns::{config::HetznerConfig, nomad};
    use reqwest::Url;

    use crate::mocks::{hetzner_mock, nomad_mock};
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

    #[tokio::test]
    async fn test_get_nomad_tags() {
        let mut server = Server::new_async().await;
        let url = server.url();
        let get_mock = nomad_mock::mock_get_nomad_service_by_name(&mut server).await;

        let parsed_url = match Url::parse(&url) {
            Ok(url) => url,
            Err(e) => {
                eprintln!("Failed to parse URL: {}", e);
                return;
            }
        };

        let hostname = match parsed_url.host_str() {
            Some(host) => host.to_string(),
            None => {
                eprintln!("URL does not contain a hostname.");
                return;
            }
        };

        let port = match parsed_url.port_or_known_default() {
            Some(port) => port.to_string(),
            None => {
                eprintln!("URL does not contain a port and no known default.");
                return;
            }
        };

        let config = Config {
            nomad_hostname: "http://".to_owned() + &hostname,
            nomad_port: port,
            nomad_service_name: "fakeservice".to_string(),
            dns_provider: DnsProvider::Hetzner(HetznerConfig {
                dns_token: "fake".to_string(),
                dns_zone_id: "fake".to_string(),
                api_url: "fake".to_string(),
            }),
            consul_address: "fake".to_string(),
            consul_datacenter: None,
        };

        let nomad_tag = match nomad::fetch_and_parse_service_tags(&config).await {
            Ok(tag) => tag,
            Err(e) => {
                eprintln!("Failed to fetch Nomad DNS tags: {}", e);
                return;
            }
        };

        get_mock.assert();
        assert_eq!(nomad_tag.hostname, "redis.example.com");
    }
}
