use clap::{Arg, Command};

/// Define a struct to hold all command-line arguments

#[derive(Debug)]
pub struct Config {
    pub hetzner_dns_token: String,
    pub hetzner_dns_zone_id: String,
    pub nomad_service_name: String,
    pub nomad_hostname: String,
    pub nomad_port: String,
}

/// Function to parse the command-line arguments and return a Config instance
pub fn parse_args() -> Config {
    let matches = Command::new("Nomad External DNS Tool For Hetzner")
        .version("0.1.0")
        .about("Updates Hetzner DNS records based on Nomad job tags")
        .arg(
            Arg::new("hetzner-dns-token")
                .long("hetzner-dns-token")
                .value_name("TOKEN")
                .help("Sets the Hetzner DNS API token")
                .required(true),
        )
        .arg(
            Arg::new("hetzner-dns-zone-id")
                .long("hetzner-dns-zone-id")
                .value_name("ZONE_ID")
                .help("Sets the Hetzner DNS zone ID")
                .required(true),
        )
        .arg(
            Arg::new("nomad-service-name")
                .long("nomad-service-name")
                .value_name("NOMAD_SERVICE_NAME")
                .help("Sets the Nomad service name")
                .required(true),
        )
        .arg(
            Arg::new("nomad-hostname")
                .long("nomad-hostname")
                .value_name("NOMAD_HOSTNAME")
                .help("Sets the Nomad server hostname")
                .default_value("localhost")
                .required(false),
        )
        .arg(
            Arg::new("nomad-port")
                .long("nomad-port")
                .value_name("NOMAD_PORT")
                .help("Sets the port number of the Nomad server")
                .default_value("4646")
                .required(false),
        )
        .get_matches();

    Config {
        hetzner_dns_token: matches
            .get_one::<String>("hetzner-dns-token")
            .expect("required")
            .to_owned(),
        hetzner_dns_zone_id: matches
            .get_one::<String>("hetzner-dns-zone-id")
            .expect("required")
            .to_owned(),
        nomad_service_name: matches
            .get_one::<String>("nomad-service-name")
            .expect("required")
            .to_owned(),
        nomad_hostname: matches
            .get_one::<String>("nomad-hostname")
            .unwrap()
            .to_owned(),
        nomad_port: matches.get_one::<String>("nomad-port").unwrap().to_owned(),
    }
}
