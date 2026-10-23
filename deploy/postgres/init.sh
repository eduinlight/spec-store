#!/usr/bin/env bash
set -euo pipefail

: "${PLATFORM_DB_PASSWORD:?PLATFORM_DB_PASSWORD must be set}"
: "${KEYCLOAK_DB_PASSWORD:?KEYCLOAK_DB_PASSWORD must be set}"

psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname postgres \
  -v platform_password="$PLATFORM_DB_PASSWORD" \
  -v keycloak_password="$KEYCLOAK_DB_PASSWORD" <<'SQL'
CREATE ROLE platform LOGIN CREATEDB PASSWORD :'platform_password';
CREATE DATABASE platform OWNER platform;
CREATE ROLE keycloak LOGIN PASSWORD :'keycloak_password';
CREATE DATABASE keycloak OWNER keycloak;
SQL
