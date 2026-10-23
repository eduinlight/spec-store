#!/usr/bin/env bash
set -euo pipefail

KEYCLOAK_URL="${KEYCLOAK_URL:-http://localhost:${KEYCLOAK_PORT:-58080}}"
REALM="${REALM:-spec-store}"
MCP_RESOURCE="${MCP_RESOURCE:-http://localhost:54100/mcp}"
ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:?KEYCLOAK_ADMIN_PASSWORD must be set; it is in .env}"
PERSON="${CHECK_PERSON:-dev}"

json() {
  python3 -c "import sys, json; data = json.load(sys.stdin); print($1)"
}

fail() {
  echo "check-realm: $1" >&2
  exit 1
}

discovery="$(curl -sf "$KEYCLOAK_URL/realms/$REALM/.well-known/openid-configuration")" || fail "realm $REALM is not served"
registration="$(echo "$discovery" | json "data.get('registration_endpoint', '')")"
device="$(echo "$discovery" | json "data.get('device_authorization_endpoint', '')")"
[[ -n "$registration" ]] || fail "no registration_endpoint"
[[ -n "$device" ]] || fail "no device_authorization_endpoint"
echo "check-realm: registration endpoint $registration"
echo "check-realm: device endpoint $device"

curl -sf "$KEYCLOAK_URL/.well-known/oauth-authorization-server/realms/$REALM" >/dev/null || fail "no RFC 8414 metadata at the MCP discovery path"
echo "check-realm: authorization server metadata served at the MCP discovery path"

client_id="$(curl -sf -X POST "$registration" -H 'content-type: application/json' -d '{
  "client_name": "check-realm",
  "redirect_uris": ["http://localhost:33418/callback"],
  "token_endpoint_auth_method": "none",
  "grant_types": ["authorization_code", "refresh_token"],
  "response_types": ["code"]
}' | json "data['client_id']")" || fail "anonymous dynamic client registration refused"
echo "check-realm: registered client $client_id"

admin_token="$(curl -sf -d client_id=admin-cli -d "username=$ADMIN_USER" -d "password=$ADMIN_PASSWORD" -d grant_type=password \
  "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" | json "data['access_token']")"
admin() {
  curl -sf -H "Authorization: Bearer $admin_token" "$KEYCLOAK_URL/admin/realms/$REALM$1"
}
client_uuid="$(admin "/clients?clientId=$client_id" | json "data[0]['id']")"
user_id="$(admin "/users?username=$PERSON&exact=true" | json "data[0]['id']")"
audience="$(admin "/clients/$client_uuid/evaluate-scopes/generate-example-access-token?scope=openid&userId=$user_id" | json "data['aud'] if isinstance(data['aud'], list) else [data['aud']]")"
admin "/clients/$client_uuid" >/dev/null
curl -sf -X DELETE -H "Authorization: Bearer $admin_token" "$KEYCLOAK_URL/admin/realms/$REALM/clients/$client_uuid" >/dev/null || true
[[ "$audience" == *"$MCP_RESOURCE"* ]] || fail "access token audience $audience does not contain $MCP_RESOURCE"
echo "check-realm: dynamically registered clients receive tokens for $MCP_RESOURCE"
echo "check-realm: ok"
