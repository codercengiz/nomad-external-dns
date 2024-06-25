use std::{
    process::{Command, Stdio},
    time::Duration,
};

use mockito::{Server, ServerGuard};
use reqwest::Client;
use rstest::fixture;
use tokio::time::sleep;

#[fixture]
pub async fn server() -> ServerGuard {
    Server::new_async().await
}

pub struct EnvironmentManager;

impl EnvironmentManager {
    pub async fn new() -> Self {
        Self::start_consul().await.expect("Failed to start Consul");
        Self::start_nomad().await.expect("Failed to start Nomad");
        EnvironmentManager
    }

    pub async fn start_consul() -> Result<(), String> {
        Command::new("docker")
            .arg("run")
            .arg("--rm")
            .arg("--name")
            .arg("consul-dev")
            .arg("-p")
            .arg("8500:8500")
            .arg("hashicorp/consul")
            .arg("agent")
            .arg("-dev")
            .arg("-client=0.0.0.0")
            .stdout(Stdio::null())
            .spawn()
            .expect("Couldn't run Consul binary");

        let client = Client::new();
        let mut retries = 5;
        while retries > 0 {
            let res = client
                .get("http://127.0.0.1:8500/v1/status/leader")
                .send()
                .await;
            match res {
                Ok(response) if response.status().is_success() => {
                    println!("Consul is up and running!");
                    return Ok(());
                }
                _ => {
                    println!("Waiting for Consul to start...");
                    sleep(Duration::from_secs(1)).await;
                    retries -= 1;
                }
            }
        }
        Err("Consul did not start in time".into())
    }

    pub async fn start_nomad() -> Result<(), String> {
        Command::new("sudo")
            .arg("nomad")
            .arg("agent")
            .arg("-dev")
            .arg("-config=tests/nomad.hcl")
            .stdout(Stdio::null())
            .spawn()
            .expect("Couldn't run Nomad binary");

        let client = Client::new();
        let mut retries = 5;
        while retries > 0 {
            let res = client
                .get("http://127.0.0.1:4646/v1/status/leader")
                .send()
                .await;
            match res {
                Ok(response) if response.status().is_success() => {
                    println!("Nomad is up and running!");
                    return Ok(());
                }
                _ => {
                    println!("Waiting for Nomad to start...");
                    sleep(Duration::from_secs(1)).await;
                    retries -= 1;
                }
            }
        }
        Err("Nomad did not start in time".into())
    }
}

impl Drop for EnvironmentManager {
    fn drop(&mut self) {
        Command::new("sudo")
            .arg("pkill")
            .arg("-f")
            .arg("nomad agent")
            .output()
            .expect("Failed to stop Nomad");

        println!("Stopping Nomad agent...");

        // Stopping and removing the Consul Docker container
        Command::new("docker")
            .arg("kill")
            .arg("consul-dev")
            .output()
            .expect("Failed to stop consul-dev Docker container");

        Command::new("docker")
            .arg("rm")
            .arg("consul-dev")
            .output()
            .expect("Failed to remove consul-dev Docker container");

        println!("Environment cleaned up.");
    }
}
