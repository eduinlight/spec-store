# spec-store

Hosted store for Spec Driven Development documents (constitution, spec, plan, tasks, research,
validation), shared by teams and their AI agents. Organizations own projects; people sign in with
Keycloak; agents connect over MCP by approving the client in the browser; a CLI manages
organizations, projects, import and export from the terminal. Every document write is an
append-only revision with an audit entry, stored in PostgreSQL.

## Repository layout

| Path | What it is |
|---|---|
| `packages/core` | Rust library with every business rule: organizations, projects, specs, revisions, audit, invitations, sessions, Keycloak token checks, email |
| `packages/testkit` | Test support: per-test PostgreSQL databases, a fake Keycloak, Mailpit helpers |
| `packages/ui` | Shared design tokens, Tailwind theme, shadcn components and language helpers |
| `apps/api` | JSON API (axum): browser sign-in, organizations, projects, read views, import and export, invitation emails |
| `apps/mcp` | MCP server (rmcp over Streamable HTTP) with MCP authorization |
| `apps/cli` | `spec-store` command-line tool with device sign-in |
| `apps/app` | Browser app (React 19, React Router 7, Vite) |
| `apps/landing` | Public landing page (React Router 7, prerendered) |
| `deploy/` | Dockerfiles, nginx configs, Keycloak theme and realm, PostgreSQL init and backup jobs |
| `specs/` | The SDD documents of this repository |

## Development

Prerequisites: Docker, Rust 1.97 with clippy and rustfmt, Node 24, pnpm 10.33 and `libpq`.

```sh
make install     # copies .env.dist to .env, starts PostgreSQL, Keycloak and Mailpit, installs packages
make dev         # runs the API, the MCP server, the app and the landing page
make uninstall   # stops everything and removes containers, volumes and the local Keycloak image
```

`make dev` prints every address. With the defaults from `.env.dist`:

| Service | Address |
|---|---|
| Landing | http://localhost:5174 |
| App | http://localhost:5173 (sign in as `dev` / `${SPEC_STORE_DEV_PASSWORD}`; `ops` / `${SPEC_STORE_OPS_PASSWORD}` is a platform operator) |
| API | http://localhost:54000 |
| MCP | http://localhost:54100/mcp |
| Keycloak admin | http://localhost:58080/admin (`KEYCLOAK_ADMIN_USER` / `KEYCLOAK_ADMIN_PASSWORD`) |
| Mailpit inbox | http://localhost:58025 |

The app dev server proxies `/api` and `/auth` to the API, so the browser sees a single origin, as
it does in production. Keycloak imports the realm from `deploy/keycloak/realm/spec-store.json`
and the test people from `deploy/keycloak/dev/` on its first start; run `make uninstall` and
`make install` to start over.

## Checks

```sh
make lint        # clippy, Biome, TypeScript and the OpenAPI drift check
make fmt-check   # rustfmt and Biome formatting
make test        # every Rust and TypeScript test (starts PostgreSQL and Mailpit first)
make e2e         # browser tests against the local stack and Lighthouse on the landing page
make openapi     # regenerates apps/app/src/api/openapi.json and schema.d.ts after API changes
```
