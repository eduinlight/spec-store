FROM debian:trixie-slim
ARG BINARY=target/release/spec-store-api
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates libpq5 \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --uid 10001 --no-create-home spec-store
COPY ${BINARY} /app/spec-store-api
USER 10001
ENV API_BIND=0.0.0.0:4000
EXPOSE 4000
ENTRYPOINT ["/app/spec-store-api"]
