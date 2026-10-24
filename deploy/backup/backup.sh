#!/usr/bin/env bash
set -euo pipefail

: "${PGHOST:?PGHOST must be set}"
: "${PGUSER:?PGUSER must be set}"
: "${PGPASSWORD:?PGPASSWORD must be set}"
: "${BACKUP_REMOTE:?BACKUP_REMOTE must be set, for example garage:sdd-vault-prod-backups}"
PLATFORM_DATABASE="${PLATFORM_DATABASE:-platform}"
OTHER_DATABASES="${OTHER_DATABASES:-keycloak}"
RETENTION_DAYS="${RETENTION_DAYS:-30}"
export PGPORT="${PGPORT:-5432}"

stamp="$(date -u +%Y%m%dT%H%M%SZ)"
work="$(mktemp -d)"
mkdir "$work/$stamp"
out="$work/$stamp"

coproc SNAPSHOT { psql -d "$PLATFORM_DATABASE" -At -q -v ON_ERROR_STOP=1; }
snapshot_in="${SNAPSHOT[1]}"
snapshot_out="${SNAPSHOT[0]}"
trap 'exec {snapshot_in}>&- 2>/dev/null || true; rm -rf "$work"' EXIT

echo "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY; SELECT pg_export_snapshot();" >&"$snapshot_in"
read -r snapshot <&"$snapshot_out"
echo "SELECT json_build_object(
  'database', current_database(),
  'taken_at', '$stamp',
  'counts', json_build_object(
    'organizations', (SELECT count(*) FROM organizations),
    'projects', (SELECT count(*) FROM projects),
    'documents', (SELECT count(*) FROM documents),
    'revisions', (SELECT count(*) FROM revisions),
    'audit_log', (SELECT count(*) FROM audit_log)));" >&"$snapshot_in"
read -r meta <&"$snapshot_out"
pg_dump --format=custom --snapshot="$snapshot" --dbname="$PLATFORM_DATABASE" --file="$out/$PLATFORM_DATABASE.dump"
echo "COMMIT;" >&"$snapshot_in"
exec {snapshot_in}>&-
wait "$SNAPSHOT_PID"

printf '%s\n' "$meta" > "$out/meta.json"
pg_dumpall --globals-only --no-role-passwords > "$out/globals.sql"
for database in $OTHER_DATABASES; do
  pg_dump --format=custom --dbname="$database" --file="$out/$database.dump"
done

rclone copy "$out" "$BACKUP_REMOTE/$stamp"
rclone delete --min-age "${RETENTION_DAYS}d" "$BACKUP_REMOTE"
rclone rmdirs --leave-root "$BACKUP_REMOTE"
echo "backup $stamp uploaded to $BACKUP_REMOTE: $PLATFORM_DATABASE $OTHER_DATABASES"
