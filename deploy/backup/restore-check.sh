#!/usr/bin/env bash
set -euo pipefail

: "${BACKUP_REMOTE:?BACKUP_REMOTE must be set}"
PLATFORM_DATABASE="${PLATFORM_DATABASE:-platform}"
OTHER_DATABASES="${OTHER_DATABASES:-keycloak}"

work="$(mktemp -d)"
data="$work/data"
cleanup() {
  pg_ctl -D "$data" -m immediate stop >/dev/null 2>&1 || true
  rm -rf "$work"
}
trap cleanup EXIT

latest="$(rclone lsf --dirs-only "$BACKUP_REMOTE" | sort | tail -n 1)"
latest="${latest%/}"
if [[ -z "$latest" ]]; then
  echo "restore-check: no backup found in $BACKUP_REMOTE" >&2
  exit 1
fi
rclone copy "$BACKUP_REMOTE/$latest" "$work/backup"
if [[ ! -s "$work/backup/meta.json" ]]; then
  echo "restore-check: backup $latest has no meta.json" >&2
  exit 1
fi

initdb --username=postgres --auth=trust -D "$data" >/dev/null
pg_ctl -D "$data" -o "-p 55999 -k $work -c listen_addresses=''" -w start >/dev/null
export PGHOST="$work" PGPORT=55999 PGUSER=postgres
unset PGPASSWORD

psql -q -d postgres -f "$work/backup/globals.sql" >/dev/null 2>&1 || true
for database in $PLATFORM_DATABASE $OTHER_DATABASES; do
  createdb "$database"
  pg_restore --exit-on-error --no-owner --dbname="$database" "$work/backup/$database.dump"
done

status=0
checked=0
while IFS='|' read -r table recorded; do
  checked=$((checked + 1))
  restored="$(psql -d "$PLATFORM_DATABASE" -At -c "SELECT count(*) FROM \"$table\"")"
  if [[ "$restored" != "$recorded" ]]; then
    echo "restore-check: $table has $restored rows after restore, meta.json recorded $recorded" >&2
    status=1
  else
    echo "restore-check: $table $restored rows"
  fi
done < <(echo "SELECT key || '|' || value FROM json_each_text((:'meta')::json -> 'counts') ORDER BY key;" \
  | psql -d postgres -At -v ON_ERROR_STOP=1 -v meta="$(cat "$work/backup/meta.json")")

if (( checked == 0 )); then
  echo "restore-check: meta.json of backup $latest has no counts" >&2
  status=1
fi
if (( status == 0 )); then
  echo "restore-check: backup $latest restored and verified"
fi
exit "$status"
