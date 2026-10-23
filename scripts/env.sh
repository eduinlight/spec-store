#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
if [[ -f .env ]]; then
  missing=()
  while IFS= read -r key; do
    grep -q "^$key=." .env || missing+=("$key")
  done < <(sed -n 's/^\([A-Z_]*\)=.*/\1/p' .env.dist)
  if (( ${#missing[@]} > 0 )); then
    echo "env: .env lacks ${missing[*]}; remove it and run make env to generate a new one" >&2
    exit 1
  fi
  exit 0
fi

draft="$(mktemp)"
trap 'rm -f "$draft"' EXIT
while IFS= read -r line || [[ -n "$line" ]]; do
  case "$line" in
    SESSION_SEALING_KEY='<generated>') echo "SESSION_SEALING_KEY=$(openssl rand -base64 32)" ;;
    *='<generated>') echo "${line%%=*}=$(openssl rand -hex 24)" ;;
    *) echo "$line" ;;
  esac
done < .env.dist > "$draft"
platform_password="$(sed -n 's/^PLATFORM_DB_PASSWORD=//p' "$draft")"
sed "s|<platform-db-password>|$platform_password|" "$draft" > .env
chmod 600 .env
echo "wrote .env with generated credentials"
