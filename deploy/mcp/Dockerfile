FROM debian:trixie-slim
ARG BINARY=target/release/sdd-vault-mcp
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libpq5 \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 --no-create-home sdd-vault
COPY ${BINARY} /app/sdd-vault-mcp
USER 10001
ENV MCP_BIND=0.0.0.0:4100
EXPOSE 4100
ENTRYPOINT ["/app/sdd-vault-mcp"]
