# sdd-vault

Hosted store for Spec Driven Development documents (constitution, spec, plan, tasks, research,
validation), shared by teams and their AI agents. Organizations own projects; people sign in with
Keycloak; agents connect over MCP by approving the client in the browser; a CLI manages
organizations, projects, import and export from the terminal. Every document write is an
append-only revision with an audit entry, stored in PostgreSQL.

## Architecture

```
 browser ── app / landing (nginx) ──┐
                                    ├── api (axum) ──┐
 sdd-vault CLI ── bearer token ────┘                ├── packages/core ── PostgreSQL
 AI agent ── MCP over HTTP ───────────── mcp (rmcp) ─┘
                 └──────── Keycloak (sign-in, MCP and device authorization) ────────┘
```

- `packages/core` holds every rule. The API, the MCP server and the CLI's server side call it, so an
  operation behaves the same way and fails with the same error everywhere.
- The browser app never holds a token. The API signs the person in with Keycloak, keeps the tokens
  sealed in PostgreSQL and gives the browser an HTTP-only session cookie.
- The CLI signs in with a device code and keeps its tokens in the system keychain.
- Agents find the authorization server through the MCP server's protected-resource metadata, and the
  person approves them in the browser.
- Invitation emails go through an outbox table in PostgreSQL, so a write and its email commit
  together.

## Using sdd-vault

### CLI

```sh
cargo install --locked --git https://github.com/eduinlight/sdd-vault sdd-vault-cli
sddvault login                      # opens the browser with a device code
sddvault org create acme "Acme"
sddvault project create acme/demo "Demo"
sddvault import acme/demo .         # docs/constitution.md and specs/NNN-name/*.md
sddvault export acme/demo ./out
```

`--api`, `--issuer` and `--client-id` (or `SDD_VAULT_API`, `SDD_VAULT_ISSUER`,
`SDD_VAULT_CLIENT_ID`) point the CLI at another environment. They default to production.

### MCP

Add the server to an MCP client as a Streamable HTTP server:

```sh
claude mcp add --transport http sdd-vault https://mcp-sddvault.eduindev.com/mcp
```

On the first call, the server answers with where to authorize. The client opens Keycloak in the
browser, the person signs in and approves the client, and the agent then acts with that person's
access. Revoke an approved agent from the profile page in the app.

## Repository layout

| Path | What it is |
|---|---|
| `packages/core` | Rust library with every business rule: organizations, projects, specs, revisions, audit, invitations, sessions, Keycloak token checks, email |
| `packages/testkit` | Test support: per-test PostgreSQL databases, a fake Keycloak, Mailpit helpers |
| `packages/ui` | Shared design tokens, Tailwind theme, shadcn components and language helpers |
| `apps/api` | JSON API (axum): browser sign-in, organizations, projects, read views, import and export, invitation emails |
| `apps/mcp` | MCP server (rmcp over Streamable HTTP) with MCP authorization |
| `apps/cli` | `sdd-vault` command-line tool with device sign-in |
| `apps/app` | Browser app (React 19, React Router 7, Vite) |
| `apps/landing` | Public landing page (React Router 7, prerendered) |
| `deploy/` | Dockerfiles, nginx configs, Keycloak theme and realm, PostgreSQL init and backup jobs |
| `specs/` | The SDD documents of this repository |

## Development

Prerequisites: Docker, Rust 1.97 with clippy and rustfmt, Node 24, pnpm 10.33 and `libpq`.

```sh
make install     # writes .env with generated credentials, starts PostgreSQL, Keycloak and Mailpit, installs packages
make dev         # runs the API, the MCP server, the app and the landing page
make uninstall   # stops everything and removes containers, volumes and the local Keycloak image
```

`make dev` prints every address. `.env` is never committed: `make env` writes it from `.env.dist`
and generates every password, client secret and key in it. With the default ports:

| Service | Address |
|---|---|
| Landing | http://localhost:5174 |
| App | http://localhost:5173 (sign in as `dev` with `SDD_VAULT_DEV_PASSWORD`; `ops` with `SDD_VAULT_OPS_PASSWORD` is a platform operator) |
| API | http://localhost:54000 |
| MCP | http://localhost:54100/mcp |
| Keycloak admin | http://localhost:58080/admin (`KEYCLOAK_ADMIN_USER` with `KEYCLOAK_ADMIN_PASSWORD`) |
| Mailpit inbox | http://localhost:58025 |

The app dev server proxies `/api` and `/auth` to the API, so the browser sees a single origin, as
it does in production. Keycloak imports the realm from `deploy/keycloak/realm/sdd-vault.json`
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
