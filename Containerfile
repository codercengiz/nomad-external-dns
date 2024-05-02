FROM docker.io/alpine
COPY --chmod=755 consul_external_dns /app/
ENTRYPOINT ["/app/consul_external_dns"]
