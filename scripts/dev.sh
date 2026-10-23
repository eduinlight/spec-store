#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
./scripts/env.sh >/dev/null
set -a
. ./.env
set +a

pids=()
cleanup() {
  kill "${pids[@]}" 2>/dev/null || true
  wait 2>/dev/null || true
}
trap cleanup EXIT INT TERM

cargo build -q --locked -p spec-store-api -p spec-store-mcp
./target/debug/spec-store-api &
pids+=($!)
./target/debug/spec-store-mcp &
pids+=($!)
API_URL="$API_URL" pnpm --filter app dev --strictPort &
pids+=($!)
VITE_APP_URL="$APP_URL" pnpm --filter landing dev --strictPort &
pids+=($!)

echo
echo "spec-store is running:"
echo "    landing  $LANDING_URL"
echo "    app      $APP_URL      (sign in as dev with SPEC_STORE_DEV_PASSWORD from .env)"
echo "    api      $API_URL"
echo "    mcp      $MCP_RESOURCE"
echo "    keycloak http://localhost:${KEYCLOAK_PORT:-58080}/admin ($KEYCLOAK_ADMIN_USER with KEYCLOAK_ADMIN_PASSWORD from .env)"
echo "    mailpit  http://localhost:${MAILPIT_UI_PORT:-58025}"
echo
wait
