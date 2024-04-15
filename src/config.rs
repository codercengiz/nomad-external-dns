use clap::Parser;

/// Define a struct to hold all command-line arguments
#[derive(Debug, Parser)]
#[command(author, about, version)]
pub struct Config {
    /// Sets the Hetzner DNS API token
    #[arg(long, env = "HETZNER_DNS_TOKEN")]
    pub hetzner_dns_token: String,

    /// Sets the Hetzner DNS zone ID
    #[arg(long)]
    pub hetzner_dns_zone_id: String,

    /// Sets the Nomad service name
    #[arg(long)]
    pub nomad_service_name: String,

    /// Sets the Nomad server hostname
    #[arg(long, default_value = "localhost")]
    pub nomad_hostname: String,

    /// Sets the port number of the Nomad server
    #[arg(long, default_value = "4646")]
    pub nomad_port: String,

    /// Sets the address of the Consul server
    #[arg(long, default_value = "http://127.0.0.1:8500")]
    pub consul_address: String,

    /// Sets datacenter of the Consul server, optional
    #[arg(long)]
    pub consul_datacenter: Option<String>,
}
