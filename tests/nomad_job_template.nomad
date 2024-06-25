job "example-{{JOBNAME}}" {
  datacenters = ["dc1"]
  type = "service"

  group "example-{{JOBNAME}}-group" {
    count = 1

    task "server" {
      driver = "docker"

      config {
        image = "hashicorp/http-echo"
        args = [
          "-listen", ":5678",
          "-text", "Hello from Nomad!"
        ]
      }

      service {
        name = "http-echo-{{JOBNAME}}"
        tags = [ 
          {{TAGS}}
          "external-dns.enable={{EXTERNAL.DNS.ENABLE}}"
          ]
        port = "http"

        check {
          name     = "alive"
          type     = "tcp"
          port     = "http"
          interval = "10s"
          timeout  = "2s"
        }
      }

      resources {
        cpu    = 500 # 500 MHz
        memory = 256 # 256MB

        network {
          mbits = 10
          port "http" {
            static = 5678
          }
        }
      }
    }
  }
}
