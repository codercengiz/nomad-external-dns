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
    /// Specifies the service name in Nomad.
    #[arg(long)]
    pub nomad_service_name: String,

    /// Specifies the Nomad server hostname. Defaults to "localhost".
    #[arg(long, default_value = "localhost")]
    pub nomad_hostname: String,

    /// Specifies the port number for the Nomad server. Defaults to 4646.
    #[arg(long, default_value = "4646")]
    pub nomad_port: String,

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
}
