mod fixtures;
mod mocks;

#[cfg(test)]
mod tests {

    use std::fs;
    use std::process::Command;

    use consul_external_dns::config::HetznerConfig;
    use consul_external_dns::config::{Config, DnsProvider};
    use consul_external_dns::consul::{ConsulClient, DnsRecord};
    use consul_external_dns::dns_trait::{self, DnsProviderTrait, DnsType};
    use consul_external_dns::hetzner_dns::HetznerDns;
    use fake::Fake;
    use mockito::Server;
    use reqwest::Url;

    use crate::fixtures::{self, EnvironmentManager};
    use crate::mocks::{consul_mock, hetzner_mock};

    // It uses the mockito library to mock the Hetzner service response and checks if the DNS record was created.
    #[tokio::test]
    async fn test_create_non_existing_dns_record() {
        let mut server = fixtures::server().await;

        let config = HetznerConfig {
            dns_token: "fake_token".to_string(),
            dns_zone_id: "fake_zone_id".to_string(),
            api_url: url::Url::parse(&server.url()).expect("Invalid URL"),
        };
        let hetzner_dns = HetznerDns { config };
        let consul_dns_record = DnsRecord {
            hostname: "new.example.com".to_string(),
            type_: DnsType::A,
            value: "192.168.0.1".to_string(),
            ttl: Some(300),
        };

        let expected_dns_record = dns_trait::DnsRecord {
            id: "fake_record_id".to_string(),
            zone_id: "fake_zone_id".to_string(),
            type_: DnsType::A,
            name: "new.example.com".to_string(),
            value: "192.168.0.1".to_string(),
            ttl: Some(300),
        };

        // convert the expected_dns_record_create to a JSON string
        let create_mock =
            hetzner_mock::mock_create_dns_record(&mut server, &expected_dns_record).await;
        hetzner_mock::mock_get_dns_records(&mut server, "fake_zone_id", "fake_token").await;

        let result = hetzner_dns.create_dns_record(&consul_dns_record).await;
        create_mock.assert();
        assert!(result.is_ok());
    }

    // It uses the mockito library to mock the Consul service response and checks if the tags are fetched correctly.
    #[tokio::test]
    async fn test_get_dns_tags() {
        let mut server = fixtures::server().await;
        let get_mock_consul = consul_mock::mock_get_consul_services(&mut server).await;

        let parsed_url = match Url::parse(&server.url()) {
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
            dns_provider: DnsProvider::Hetzner(HetznerConfig {
                dns_token: "fake".to_string(),
                dns_zone_id: "fake".to_string(),
                api_url: url::Url::parse(&server.url()).expect("Invalid URL"),
            }),
            consul_address: url::Url::parse(format!("http://{}:{}", hostname, port).as_str())
                .expect("Invalid URL"),
            consul_datacenter: None,
        };

        let consul_client =
            ConsulClient::new(config.consul_address, config.consul_datacenter.clone())
                .expect("Failed to create Consul client");

        let mut consul_index: Option<String> = None;

        let dns_tags = match consul_client.fetch_service_tags(&mut consul_index).await {
            Ok(tags) => tags,
            Err(e) => {
                eprintln!("Failed to fetch Consul DNS tags: {}", e);
                return;
            }
        };

        get_mock_consul.assert();
        assert_eq!(dns_tags.len(), 4);

        let has_correct_value = dns_tags.iter().any(|tag| tag.value == "192.168.1.102");
        assert!(
            has_correct_value,
            "No tag has the expected value of '192.168.1.102'"
        );
    }

    // It will start Consul and Nomad in dev mode, run the Nomad job, and check if the DNS record was created.
    // This is an end-to-end test that checks if the application works as expected.
    #[tokio::test]
    async fn test_end_to_end() {
        let _env_manager = EnvironmentManager::new().await;

        let mut mock_hetzner_server = Server::new_async().await;

        let random_dns_record_id_1: String = (10..20).fake();
        let dns_record_create_1 = dns_trait::DnsRecord {
            id: random_dns_record_id_1,
            zone_id: "test_zone_id".to_string(),
            type_: DnsType::A,
            name: "abc-def-xyz".to_string(),
            value: "1.1.1.101".to_string(),
            ttl: None,
        };

        let random_dns_record_id_2: String = (10..20).fake();
        let dns_record_create_2 = dns_trait::DnsRecord {
            id: random_dns_record_id_2,
            zone_id: "test_zone_id".to_string(),
            type_: DnsType::AAAA,
            name: "def-xyz".to_string(),
            value: "1.1.1.102".to_string(),
            ttl: Some(300),
        };

        let job_name: String = (5..10).fake();

        let create_mock_1 =
            hetzner_mock::mock_create_dns_record(&mut mock_hetzner_server, &dns_record_create_1)
                .await;
        let create_mock_2 =
            hetzner_mock::mock_create_dns_record(&mut mock_hetzner_server, &dns_record_create_2)
                .await;
        hetzner_mock::mock_get_dns_records(&mut mock_hetzner_server, "test_zone_id", "test_token")
            .await;

        // Sleep for 5 seconds to allow the Consul and Nomad servers to start
        std::thread::sleep(std::time::Duration::from_secs(5));

        Command::new("cargo")
            .arg("run")
            .arg("--")
            .arg("--consul-address=http://127.0.0.1:8500")
            .arg("hetzner")
            .arg("--dns-token=test_token")
            .arg("--dns-zone-id=test_zone_id")
            .arg(format!("--api-url={}", mock_hetzner_server.url().clone()))
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .expect("Failed to run the application");

        let job_file_path = create_nomad_job_file_from_template(
            &[dns_record_create_1, dns_record_create_2],
            &job_name,
            true,
        )
        .expect("Failed to create Nomad job file");

        // Run the Nomad job
        let _nomad_output = Command::new("nomad")
            .arg("job")
            .arg("run")
            .arg(job_file_path)
            .stdout(std::process::Stdio::null())
            .spawn()
            .expect("Failed to run Nomad job");

        // Wait for a fixed time to allow the app to finish the saving of the DNS record
        std::thread::sleep(std::time::Duration::from_secs(5));

        // After the job is run, the DNS create API should be called with the new DNS record
        create_mock_1.assert();
        create_mock_2.assert();
    }

    fn create_nomad_job_file_from_template(
        dns_records: &[dns_trait::DnsRecord],
        job_name: &str,
        enable: bool,
    ) -> Result<String, std::io::Error> {
        let template_content = fs::read_to_string("tests/nomad_job_template.nomad")?;

        let tags: Vec<String> = dns_records.iter().map(|record| {
            let ttl_tag = record.ttl.map_or(String::new(), |ttl| format!("\"external-dns.{}.ttl={ttl}\",", record.name));
            format!(
                "\"external-dns.{}.hostname={}\",\n\"external-dns.{}.type={}\",\n\"external-dns.{}.value={}\",\n{}",
                record.name, record.name,
                record.name, record.type_,
                record.name, record.value,
                ttl_tag
            )
        }).collect();

        let tags_joined = tags.join("\n");

        let job_content = template_content
            .replace("{{JOBNAME}}", job_name)
            .replace("{{TAGS}}", &tags_joined)
            .replace("{{EXTERNAL.DNS.ENABLE}}", &enable.to_string());

        let job_file_path = format!("tests/temp_nomad_job_{}.nomad", job_name);
        fs::write(&job_file_path, job_content)?;

        Ok(job_file_path)
    }
}
