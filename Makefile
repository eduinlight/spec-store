-include .env
export

COMPOSE := docker compose

docker-up: env
	$(COMPOSE) up -d --wait

docker-down:
	$(COMPOSE) down

.env:
	@./scripts/env.sh

env: .env
	@./scripts/env.sh

install: env docker-up
	pnpm install --frozen-lockfile
	@echo
	@echo "sdd-vault services are ready:"
	@echo "    postgres  localhost:$${POSTGRES_PORT:-55432}"
	@echo "    mailpit   http://localhost:$${MAILPIT_UI_PORT:-58025}"

dev: env
	./scripts/dev.sh

uninstall:
	$(COMPOSE) down -v --remove-orphans --rmi local

reinstall: uninstall install

test-services: env
	$(COMPOSE) up -d --wait postgres mailpit

test-sweep:
	$(COMPOSE) exec -T postgres psql -U postgres -d postgres -tAc "SELECT datname FROM pg_database WHERE datname LIKE 'sdd\_vault\_test\_%' OR datname LIKE 'sdd\_vault\_tpl\_%'" | while read -r db; do \
		[ -n "$$db" ] && $(COMPOSE) exec -T postgres psql -U postgres -d postgres -qc "DROP DATABASE IF EXISTS \"$$db\" WITH (FORCE)" </dev/null; \
	done; true

OPENAPI_JSON := apps/app/src/api/openapi.json
OPENAPI_TYPES := apps/app/src/api/schema.d.ts

openapi:
	cargo run -q --locked -p sdd-vault-api -- openapi > $(OPENAPI_JSON)
	pnpm --filter app exec openapi-typescript src/api/openapi.json -o src/api/schema.d.ts --silent

openapi-check:
	@tmp=$$(mktemp -d); \
	cargo run -q --locked -p sdd-vault-api -- openapi > $$tmp/openapi.json; \
	pnpm --filter app exec openapi-typescript $$tmp/openapi.json -o $$tmp/schema.d.ts --silent; \
	if ! diff -q $$tmp/openapi.json $(OPENAPI_JSON) >/dev/null || ! diff -q $$tmp/schema.d.ts $(OPENAPI_TYPES) >/dev/null; then \
		echo "the API contract changed: run make openapi and commit the result" >&2; rm -rf $$tmp; exit 1; \
	fi; \
	rm -rf $$tmp

lint: openapi-check
	cargo clippy --workspace --locked --all-targets -- -D warnings
	pnpm exec biome lint .
	pnpm -r --if-present typecheck

fmt-check:
	cargo fmt --all --check
	pnpm exec biome format .

test: test-services test-sweep
	cargo test --workspace --locked
	$(MAKE) test-perf
	pnpm -r --if-present test

test-perf: test-services
	cargo test --release --locked -p sdd-vault-core --test perf --test perf_import -p sdd-vault-api --test perf_read -p sdd-vault-cli --test perf_import -- --ignored --nocapture

e2e: docker-up
	pnpm exec playwright test
	pnpm --filter landing build
	pnpm exec lhci autorun
	scripts/backup-check.sh

backup-check: docker-up
	scripts/backup-check.sh

.PHONY: env dev e2e backup-check docker-up docker-down install uninstall reinstall test-services test-sweep openapi openapi-check lint fmt-check test test-perf
