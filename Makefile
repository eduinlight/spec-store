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

lint:
	cargo clippy --workspace --locked --all-targets -- -D warnings
	pnpm exec biome lint .
	pnpm -r --if-present typecheck

fmt-check:
	cargo fmt --all --check
	pnpm exec biome format .

test: test-services
	cargo test --workspace --locked
	pnpm -r --if-present test

.PHONY: docker-up docker-down install uninstall reinstall test-services lint fmt-check test
