FROM docker.io/alpine
COPY --chmod=755 nomad_external_dns /app/
ENTRYPOINT ["/app/nomad_external_dns"]
