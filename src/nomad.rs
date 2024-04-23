use std::collections::HashMap;

use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize, Deserialize, Debug)]
struct NomadService {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Tags")]
    tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Job {
    #[serde(rename = "TaskGroups")]
    task_groups: Vec<TaskGroup>,
}

#[derive(Serialize, Deserialize, Debug)]
struct TaskGroup {
    #[serde(rename = "Tasks")]
    tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    #[serde(rename = "Services")]
    services: Vec<NomadService>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NomadDnsTag {
    pub hostname: String,
    pub type_: String,
    pub ttl: Option<i32>,
    pub value: String,
}

/// Fetch and parse the DNS tags from the Nomad service by service name
pub async fn fetch_and_parse_service_tags(config: &Config) -> Result<NomadDnsTag, anyhow::Error> {
    println!("Fetching Nomad service tags");
    let client = Client::new();
    let response = client
        .get(format!(
            "{}:{}/v1/job/{}",
            config.nomad_hostname, config.nomad_port, config.nomad_job_name
        ))
        .send()
        .await?
        .error_for_status()?;

    let service_detail = match response.json::<Job>().await {
        Ok(job) => job,
        Err(e) => {
            eprintln!("Error parsing JSON: {:?}", e);
            return Err(anyhow::anyhow!("Error parsing JSON: {}", e));
        }
    };
    

    let dns_tags = service_detail.task_groups
        .into_iter()
        .flat_map(|task_group| task_group.tasks)
        .flat_map(|task| task.services)
        .filter_map(|service| parse_dns_tags(service.tags))
        .next()
        .ok_or_else(|| anyhow::anyhow!("No valid DNS tags found"))?;

    Ok(dns_tags)
}

fn parse_dns_tags(tags: Vec<String>) -> Option<NomadDnsTag> {
    let mut tag_map: HashMap<String, String> = HashMap::new();
    for tag in tags.iter().filter(|t| t.starts_with("external-dns.")) {
        let parts: Vec<&str> = tag.split('=').collect();
        if parts.len() == 2 {
            tag_map.insert(parts[0].to_string(), parts[1].to_string());
        }
    }
    Some(NomadDnsTag {
        hostname: tag_map.get("external-dns.hostname").cloned()?,
        type_: tag_map.get("external-dns.type").cloned()?,
        ttl: tag_map.get("external-dns.ttl").and_then(|t| t.parse().ok()),
        value: tag_map.get("external-dns.value").cloned()?,
    })
}
