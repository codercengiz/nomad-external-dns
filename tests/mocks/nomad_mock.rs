use mockito::ServerGuard;

pub async fn mock_get_nomad_service_by_name(server: &mut ServerGuard) -> mockito::Mock {
    println!("Mocking Nomad job by job name");
    server
        .mock("GET", mockito::Matcher::Any)
        .with_status(200)
        .with_header("Content-Type", "application/json")
        .with_body(
            r#"
            {
              "Affinities": null,
              "AllAtOnce": false,
              "Constraints": null,
              "ConsulNamespace": "",
              "ConsulToken": "",
              "CreateIndex": 11,
              "Datacenters": [
                  "dc1"
              ],
              "DispatchIdempotencyToken": "",
              "Dispatched": false,
              "ID": "nomad-external-dns-job",
              "JobModifyIndex": 11,
              "Meta": null,
              "ModifyIndex": 12,
              "Multiregion": null,
              "Name": "nomad-external-dns-job",
              "Namespace": "default",
              "NodePool": "default",
              "NomadTokenID": "",
              "ParameterizedJob": null,
              "ParentID": "",
              "Payload": null,
              "Periodic": null,
              "Priority": 50,
              "Region": "global",
              "Spreads": null,
              "Stable": false,
              "Status": "running",
              "StatusDescription": "",
              "Stop": false,
              "SubmitTime": 1713796433807767378,
              "TaskGroups": [
                  {
                      "Affinities": null,
                      "Constraints": null,
                      "Consul": {
                          "Cluster": "default",
                          "Namespace": "",
                          "Partition": ""
                      },
                      "Count": 1,
                      "EphemeralDisk": {
                          "Migrate": false,
                          "SizeMB": 300,
                          "Sticky": false
                      },
                      "MaxClientDisconnect": null,
                      "Meta": null,
                      "Migrate": {
                          "HealthCheck": "checks",
                          "HealthyDeadline": 300000000000,
                          "MaxParallel": 1,
                          "MinHealthyTime": 10000000000
                      },
                      "Name": "dns-group",
                      "Networks": null,
                      "PreventRescheduleOnLost": false,
                      "ReschedulePolicy": {
                          "Attempts": 0,
                          "Delay": 30000000000,
                          "DelayFunction": "exponential",
                          "Interval": 0,
                          "MaxDelay": 3600000000000,
                          "Unlimited": true
                      },
                      "RestartPolicy": {
                          "Attempts": 2,
                          "Delay": 15000000000,
                          "Interval": 1800000000000,
                          "Mode": "fail",
                          "RenderTemplates": false
                      },
                      "Scaling": null,
                      "Services": null,
                      "ShutdownDelay": null,
                      "Spreads": null,
                      "StopAfterClientDisconnect": null,
                      "Tasks": [
                          {
                              "Actions": null,
                              "Affinities": null,
                              "Artifacts": null,
                              "CSIPluginConfig": null,
                              "Config": {
                                  "network_mode": "host",
                                  "args": [
                                      "--nomad-service-name",
                                      "nomad-external-dns-service",
                                      "hetzner",
                                      "--dns-token",
                                      "test_token",
                                      "--dns-zone-id",
                                      "test_zone_id"
                                  ],
                                  "image": "nomad-external-dns:local"
                              },
                              "Constraints": null,
                              "Consul": null,
                              "DispatchPayload": null,
                              "Driver": "docker",
                              "Env": {
                                  "CONSUL_HTTP_ADDR": "http://localhost:8500",
                                  "HETZNER_DNS_API_URL": "http://127.0.0.1:38125",
                                  "NOMAD_ADDR": "http://localhost:4646"
                              },
                              "Identities": null,
                              "Identity": {
                                  "Audience": [
                                      "nomadproject.io"
                                  ],
                                  "ChangeMode": "",
                                  "ChangeSignal": "",
                                  "Env": false,
                                  "File": false,
                                  "Name": "default",
                                  "ServiceName": "",
                                  "TTL": 0
                              },
                              "KillSignal": "",
                              "KillTimeout": 5000000000,
                              "Kind": "",
                              "Leader": false,
                              "Lifecycle": null,
                              "LogConfig": {
                                  "Disabled": false,
                                  "MaxFileSizeMB": 10,
                                  "MaxFiles": 10
                              },
                              "Meta": null,
                              "Name": "dns-task",
                              "Resources": {
                                  "CPU": 500,
                                  "Cores": 0,
                                  "Devices": null,
                                  "DiskMB": 0,
                                  "IOPS": 0,
                                  "MemoryMB": 256,
                                  "MemoryMaxMB": 0,
                                  "NUMA": null,
                                  "Networks": null
                              },
                              "RestartPolicy": {
                                  "Attempts": 2,
                                  "Delay": 15000000000,
                                  "Interval": 1800000000000,
                                  "Mode": "fail",
                                  "RenderTemplates": false
                              },
                              "ScalingPolicies": null,
                              "Services": [
                                  {
                                      "Address": "",
                                      "AddressMode": "auto",
                                      "CanaryMeta": null,
                                      "CanaryTags": null,
                                      "Checks": [
                                          {
                                              "AddressMode": "",
                                              "Args": null,
                                              "Body": "",
                                              "CheckRestart": null,
                                              "Command": "check_services_health.sh",
                                              "Expose": false,
                                              "FailuresBeforeCritical": 0,
                                              "FailuresBeforeWarning": 0,
                                              "GRPCService": "",
                                              "GRPCUseTLS": false,
                                              "Header": null,
                                              "InitialStatus": "",
                                              "Interval": 30000000000,
                                              "Method": "",
                                              "Name": "service-health-check",
                                              "OnUpdate": "require_healthy",
                                              "Path": "",
                                              "PortLabel": "",
                                              "Protocol": "",
                                              "SuccessBeforePassing": 0,
                                              "TLSServerName": "",
                                              "TLSSkipVerify": false,
                                              "TaskName": "dns-task",
                                              "Timeout": 10000000000,
                                              "Type": "script"
                                          }
                                      ],
                                      "Cluster": "default",
                                      "Connect": null,
                                      "EnableTagOverride": false,
                                      "Identity": null,
                                      "Meta": null,
                                      "Name": "nomad-external-dns-service",
                                      "Namespace": "default",
                                      "OnUpdate": "require_healthy",
                                      "PortLabel": "",
                                      "Provider": "consul",
                                      "TaggedAddresses": null,
                                      "Tags": [
                                          "external-dns.hostname=example.com",
                                          "external-dns.type=A",
                                          "external-dns.value=192.168.1.100",
                                          "external-dns.ttl=300"
                                      ],
                                      "TaskName": "dns-task"
                                  }
                              ],
                              "ShutdownDelay": 0,
                              "Templates": null,
                              "User": "",
                              "Vault": null,
                              "VolumeMounts": null
                          }
                      ],
                      "Update": {
                          "AutoPromote": false,
                          "AutoRevert": false,
                          "Canary": 0,
                          "HealthCheck": "checks",
                          "HealthyDeadline": 300000000000,
                          "MaxParallel": 1,
                          "MinHealthyTime": 10000000000,
                          "ProgressDeadline": 600000000000,
                          "Stagger": 30000000000
                      },
                      "Volumes": null
                  }
              ],
              "Type": "service",
              "Update": {
                  "AutoPromote": false,
                  "AutoRevert": false,
                  "Canary": 0,
                  "HealthCheck": "",
                  "HealthyDeadline": 0,
                  "MaxParallel": 1,
                  "MinHealthyTime": 0,
                  "ProgressDeadline": 0,
                  "Stagger": 30000000000
              },
              "VaultNamespace": "",
              "VaultToken": "",
              "Version": 0
          }
            "#,
        )
        .create_async()
        .await
}
