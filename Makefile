-include .env

export DATABASE_URL

COMPOSE := docker compose

docker-up:
	$(COMPOSE) up -d --wait

docker-down:
	$(COMPOSE) down

install: docker-up
	pnpm install --frozen-lockfile
	@echo
	@echo "spec-store services are ready:"
	@echo "    postgres  localhost:$${POSTGRES_PORT:-55432}"
	@echo "    mailpit   http://localhost:$${MAILPIT_UI_PORT:-58025}"

uninstall:
	$(COMPOSE) down -v --remove-orphans

reinstall: uninstall install

test-services:
	$(COMPOSE) up -d --wait postgres mailpit

test-sweep:
	$(COMPOSE) exec -T postgres psql -U postgres -d postgres -tAc "SELECT datname FROM pg_database WHERE datname LIKE 'spec\_store\_test\_%' OR datname LIKE 'spec\_store\_tpl\_%'" | while read -r db; do \
		[ -n "$$db" ] && $(COMPOSE) exec -T postgres psql -U postgres -d postgres -qc "DROP DATABASE IF EXISTS \"$$db\" WITH (FORCE)" </dev/null; \
	done; true

lint:
	cargo clippy --workspace --locked --all-targets -- -D warnings
	pnpm exec biome lint .
	pnpm -r --if-present typecheck

fmt-check:
	cargo fmt --all --check
	pnpm exec biome format .

test: test-services test-sweep
	cargo test --workspace --locked
	pnpm -r --if-present test

.PHONY: docker-up docker-down install uninstall reinstall test-services test-sweep lint fmt-check test
