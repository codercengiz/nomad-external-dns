use clap::{Parser, Subcommand};

/// Available DNS providers as subcommands, each with its own configuration options
#[derive(Debug, Subcommand)]
pub enum DnsProvider {
    /// Hetzner DNS provider configuration
    Hetzner(HetznerConfig),
}

/// Define a struct to hold all command-line arguments
#[derive(Debug, Parser)]
#[command(author, about, version)]
pub struct Config {
    /// Specifies the address of the Consul server. Defaults to "http://127.0.0.1:8500".
    #[arg(long, default_value = "http://127.0.0.1:8500")]
    pub consul_address: String,

    /// Optionally sets the datacenter of the Consul server.
    #[arg(long)]
    pub consul_datacenter: Option<String>,

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

    /// Sets the Hetzner DNS API URL. Defaults to "https://dns.hetzner.com/api/v1".
    #[arg(
        long,
        env = "HETZNER_DNS_API_URL",
        default_value = "https://dns.hetzner.com/api/v1"
    )]
    pub api_url: String,
}
