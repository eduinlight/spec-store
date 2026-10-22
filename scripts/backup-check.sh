#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."
network="$(docker inspect -f '{{range $name, $_ := .NetworkSettings.Networks}}{{$name}}{{end}}' "$(docker compose ps -q postgres)")"
store="$(mktemp -d)"
chmod 777 "$store"
pg() {
  docker compose exec -T postgres "$@"
}
cleanup() {
  pg dropdb -U postgres --if-exists backup_check >/dev/null 2>&1 || true
  rm -rf "$store"
}
trap cleanup EXIT

pg dropdb -U postgres --if-exists backup_check 2>/dev/null
pg createdb -U postgres backup_check
pg pg_dump -U postgres --schema-only --no-owner platform | pg psql -q -U postgres -d backup_check >/dev/null
pg psql -q -U postgres -d backup_check -v ON_ERROR_STOP=1 >/dev/null <<'SQL'
INSERT INTO users (identity_subject, handle, email, name) VALUES ('backup-check', 'backup-check', 'backup@example.com', 'Backup Check');
INSERT INTO organizations (slug, name) SELECT 'org-' || n, 'Org ' || n FROM generate_series(1, 3) n;
INSERT INTO projects (organization_id, slug, name) SELECT o.id, 'project-' || n, 'Project ' || n FROM organizations o, generate_series(1, 4) n;
INSERT INTO documents (project_id, kind) SELECT id, 'constitution' FROM projects;
INSERT INTO revisions (document_id, body, author_id, message) SELECT d.id, '# Constitution ' || n, u.id, 'revision ' || n FROM documents d, users u, generate_series(1, 5) n;
SQL

docker build -q -t spec-store-backup:check -f deploy/backup/Dockerfile . >/dev/null
run() {
  docker run --rm --network "$network" \
    -e PGHOST=postgres -e PGUSER=postgres -e PGPASSWORD \
    -e PLATFORM_DATABASE=backup_check -e OTHER_DATABASES=keycloak \
    -e BACKUP_REMOTE=/backups -v "$store:/backups" \
    --entrypoint "$1" spec-store-backup:check
}

run /usr/local/bin/backup.sh
run /usr/local/bin/restore-check.sh

latest="$(ls "$store" | sort | tail -n 1)"
sed -i.orig 's/"revisions" : [0-9]*/"revisions" : 61/' "$store/$latest/meta.json"
rm "$store/$latest/meta.json.orig"
grep -q '"revisions" : 61' "$store/$latest/meta.json"
if run /usr/local/bin/restore-check.sh; then
  echo "backup-check: the tampered count was not detected" >&2
  exit 1
fi
echo "backup-check: the tampered count was detected"
