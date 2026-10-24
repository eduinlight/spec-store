FROM debian:trixie-slim
ARG BINARY=target/release/sdd-vault-api
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libpq5 \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 --no-create-home sdd-vault
COPY ${BINARY} /app/sdd-vault-api
USER 10001
ENV API_BIND=0.0.0.0:4000
EXPOSE 4000
ENTRYPOINT ["/app/sdd-vault-api"]
