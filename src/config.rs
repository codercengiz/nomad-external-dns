use std::time::Duration;

use clap::{Parser, Subcommand};

/// Available DNS providers as subcommands, each with its own configuration options
#[derive(Clone, Debug, Subcommand)]
pub enum DnsProvider {
    /// Hetzner DNS provider configuration
    Hetzner(HetznerConfig),
}

/// Define a struct to hold all command-line arguments
#[derive(Clone, Debug, Parser)]
#[command(author, about, version)]
pub struct Config {
    /// Specifies the address of the Consul server.
    #[arg(long, default_value = "localhost:8500")]
    pub consul_address: url::Url,

    /// Optionally sets the datacenter of the Consul server.
    #[arg(long)]
    pub consul_datacenter: Option<String>,

    /// Consul TTL times out after this duration without being renewed
    #[arg(long, default_value = "30s", value_parser = humantime::parse_duration)]
    pub consul_ttl: Duration,

    #[command(subcommand)]
    pub dns_provider: DnsProvider,
}

/// Define a struct to hold all command-line arguments
#[derive(Clone, Debug, Parser)]
pub struct HetznerConfig {
    /// Sets the Hetzner DNS API token
    #[arg(long, env = "DNS_TOKEN")]
    pub dns_token: String,

    /// Sets the Hetzner DNS zone ID
    #[arg(long)]
    pub dns_zone_id: String,

    /// Sets the Hetzner DNS API URL.
    #[arg(
        long,
        env = "HETZNER_DNS_API_URL",
        default_value = "https://dns.hetzner.com/api/v1"
    )]
    pub api_url: url::Url,
}
