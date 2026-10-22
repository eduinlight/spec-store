FROM debian:trixie-slim
ARG BINARY=target/release/spec-store-mcp
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libpq5 \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 --no-create-home spec-store
COPY ${BINARY} /app/spec-store-mcp
USER 10001
ENV MCP_BIND=0.0.0.0:4100
EXPOSE 4100
ENTRYPOINT ["/app/spec-store-mcp"]
