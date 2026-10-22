-include .env

export DATABASE_URL

COMPOSE := docker compose

docker-up:
	$(COMPOSE) up -d --wait

docker-down:
	$(COMPOSE) down

install: docker-up
	@echo
	@echo "spec-store services are ready:"
	@echo "    postgres  localhost:$${POSTGRES_PORT:-55432}"
	@echo "    mailpit   http://localhost:$${MAILPIT_UI_PORT:-58025}"

uninstall:
	$(COMPOSE) down -v --remove-orphans

reinstall: uninstall install

.PHONY: docker-up docker-down install uninstall reinstall
