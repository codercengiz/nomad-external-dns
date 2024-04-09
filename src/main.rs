mod config;
mod dns;
mod nomad;

#[tokio::main]
async fn main() {
    let config = config::parse_args();
    print!("Config: {:?}", config);
}
