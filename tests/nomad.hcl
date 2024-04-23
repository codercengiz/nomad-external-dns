
consul {
  address = "localhost:8500"
}

plugin "docker" {
  config {
    endpoint = "unix:///var/run/docker.sock"

    volumes {
      enabled      = true
      selinuxlabel = "z"
    }

    allow_privileged = false
    allow_caps       = ["chown", "net_raw"]

    gc {
      image       = true
      image_delay = "3m"
      container   = true
    }
  }
}

client {
  enabled = true
  options {
    "driver.whitelist" = "docker"
    "docker.privileged" = "true"
    "docker.allow_pull_on_create" = "false" 
  }
}

