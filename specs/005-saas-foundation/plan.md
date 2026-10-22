# Technical plan — Spec 005

Spec: `specs/005-saas-foundation/spec.md` · Constitution: v2.0 · Date: 2026-10-22

## 1. Summary

The single crate becomes a Cargo workspace with one library, `packages/core`, that holds every business rule and side effect of specs 001 to 003 plus organizations, invitations and the email outbox, and three thin Rust binaries: `apps/api` (axum JSON API, browser sign-in as a Backend-for-Frontend, outbox sender), `apps/mcp` (rmcp over axum with MCP authorization) and `apps/cli` (device sign-in, commands through the API, import/export on the local disk). Persistence moves to PostgreSQL 18 through sync Diesel, with row locks replacing SQLite's single writer and an advisory lock around migrations. A pnpm workspace adds `apps/landing` (React Router 7 prerendered to static HTML), `apps/app` (React Router 7 data-mode SPA) and `packages/ui` (shadcn components and the design tokens shared with the Keycloak theme). Keycloak 26.8 with a custom theme image signs people in; lightchat's Makefile, compose, image and GitOps conventions deliver everything to staging and production, with daily off-cluster backups.

## 2. Technical context

- Language/version: Rust stable 1.97, edition 2024; TypeScript 6 on Node 24 LTS with pnpm 12.
- Framework/platform: axum 0.8, tokio 1, rmcp 3.5.1 (streamable HTTP server), diesel 2.3 (`postgres`, `r2d2`, `chrono`), diesel_migrations 2.3; React 19, React Router 7.18 (framework mode with prerender for the landing, data mode for the app), Vite 8, Tailwind CSS 4.3, shadcn/ui (CLI 4.22), react-i18next 17 + i18next 26, dayjs 1.11.
- Dependencies (new, justified in §7): Rust `jsonwebtoken` 11 (`aws_lc_rs`), `aws-lc-rs`, `reqwest` 0.13 (`rustls`, `json`), `lettre` 0.11, `utoipa` 6 + `utoipa-axum` 0.3, `keyring` 4 (CLI only), `chrono` (feature of diesel, already in the tree). Removed: `libsqlite3-sys`, the diesel `sqlite` features and `pulldown-cmark` (rendering moves to the app). Kept: `base64`, `sha2`, `rand` for session and invitation secrets, `similar` for diffs. TypeScript: `react-markdown` 10 + `remark-gfm` 4, `i18next-browser-languagedetector` 8, `openapi-fetch` 0.17 + `openapi-typescript` 7, `@biomejs/biome` 2.6, `vitest` 5 + `@testing-library/react` 16 + `jsdom`, `@playwright/test` 1.64, `@lhci/cli` 0.15, `lucide-react`, `clsx`, `tailwind-merge`.
- Storage: PostgreSQL 18; databases `platform` (this system) and `keycloak` (Keycloak's own); one instance per environment.
- Tests: `cargo test` per crate against the compose PostgreSQL (one cloned database per test, R15) and Mailpit; a fake Keycloak (JWKS, token, device and admin endpoints) in the test-support crate; Vitest for the TypeScript apps; Playwright sign-in and Lighthouse CI in `make e2e`.
- Constraints: constitution v2.0; spec NFR-1 to NFR-9; two replicas of api and mcp in production; hostnames of FR-39; secrets never in git (lightchat-cd convention).
- Unknowns: all resolved in `research.md` (R1–R23). Items still unconfirmed upstream and verified in the manual validation: Claude Code's use of a 30-day offline token (R5).

## 3. Constitution check (gate)

| Principle | Complies | Note |
|---|---|---|
| 1. The spec rules | yes | Spec 005 clarified; approval precedes `/sdd-tasks` |
| 2. Stack and limits | yes | Base stack exactly as listed; React Router pinned to 7.18 (R16); every other dependency and every extra deliverable (keycloak image, backup image, test-support crate) justified in §7 |
| 3. Tests as a gate | yes | `make lint`, `make fmt-check`, `make test` cover all crates and both apps (§9) |
| 4. Simplicity | yes | No repository/port layers: core functions take `&mut PgConnection` as today; the only new abstraction is the `Caller` resolver per transport, justified in §7 |
| 5. Markdown is the source of truth | yes | Bodies stored as markdown; tasks and markers still derived on read; API returns raw markdown |
| 6. Nothing is lost | yes | PostgreSQL only state; numbered migrations under an advisory lock; revisions append-only with `FOR UPDATE` optimistic locking; daily backups |
| 7. Access is scoped | see §8 | MCP and CLI calls carry Keycloak access tokens; browser calls carry a session cookie backed by a server-held Keycloak token (deviation) |
| 8. Reference sources | yes | Every decision cites official docs or source in `research.md` |
| 9. Language | yes | All artifacts English; UI strings translated only in apps and theme (NFR-8) |
| 10. Human approval | yes | Commits per task with OK; creating the `spec-store-cd` repository and cluster secrets are manual steps for the user |

## 4. Module / file structure

```
Cargo.toml                      workspace: members packages/core, packages/testkit, apps/api, apps/mcp, apps/cli; [workspace.package] version, edition 2024, rust-version 1.97; [workspace.dependencies]
rustfmt.toml                    tab_spaces = 2 (kept)
Makefile                        single entry point (§6.5)
docker-compose.yml              postgres:18, keycloak (deploy/keycloak image, start-dev --import-realm), mailpit; healthchecks for --wait
.env.dist                       documented variables for every deliverable
.dockerignore                   whitelist of built artifacts
package.json, pnpm-workspace.yaml, biome.json, tsconfig.base.json, vitest.config.ts (`test.projects`)
.github/workflows/              ci.yml, deploy-{api,mcp,landing,app,keycloak,backup}.yml, cd-bump.yml, release.yml
scripts/ensure-tools.sh         tool check copied from lightchat, plus node and pnpm
src/, tests/, migrations/       removed after their content moves (below)

packages/core/                  crate spec-store-core (lib), no HTTP
  migrations/0001_platform/up.sql   PostgreSQL schema (§5); old SQLite migrations removed
  src/lib.rs
  src/model.rs                  StoreError (+ Unavailable), Caller {user_id, handle, is_operator}, Role, OrgRole, DocumentKind, SpecStatus, validate_slug, validate_body (size + NUL)
  src/schema.rs                 diesel schema (regenerated)
  src/db.rs                     r2d2 PgConnection pool, migrate() under pg_advisory_xact_lock
  src/address.rs                ProjectRef parse/format "<org>/<project>"
  src/store/mod.rs              Store, with_conn/with_tx (READ COMMITTED transaction), audit()
  src/store/users.rs            upsert_user_from_identity (FR-8), caller lookup; tokens removed
  src/store/organizations.rs    create, list, members, role change, removal, last-owner rule (FR-1, FR-3)
  src/store/invitations.rs      create/resend/revoke/list/accept/decline, expiry (FR-2)
  src/store/projects.rs         projects inside an organization, access_in(caller, ProjectRef) (FR-4, FR-5)
  src/store/specs.rs, documents.rs, tasks.rs, transfer.rs   moved; FOR UPDATE locking (FR-16); ProjectRef addressing
  src/store/audit.rs            org/global audit pages (FR-17)
  src/store/sessions.rs         browser sessions and login attempts (BFF) with encrypted tokens
  src/store/outbox.rs           email_outbox enqueue/claim (SKIP LOCKED)/complete/fail (FR-2)
  src/markdown/{mod,tasks,markers}.rs   moved unchanged
  src/templates.rs              moved unchanged
  src/tree.rs                   moved unchanged (scan, plan_export, write_export; used by the CLI)
  src/diff.rs                   diff_lines moved from src/ui/diff.rs (HTML rendering removed)
  src/identity.rs               Keycloak JWT verification: JwksCache, verify(token, audience) -> Claims (FR-10, FR-11)
  src/crypto.rs                 AES-256-GCM seal/open for stored tokens (aws-lc-rs)
  src/email.rs                  invitation message (en/es) and lettre SMTP sending
  src/keycloak.rs               HTTP client: code exchange, refresh, logout URL, admin consents list/revoke, set locale
  src/config.rs                 typed settings shared by binaries (database, Keycloak, SMTP, hosts)
  tests/                        store_* tests ported from tests/*.rs + organizations, invitations, outbox, concurrency, migrations

packages/testkit/               crate spec-store-testkit (dev-dependency only)
  src/lib.rs                    TestDb (template clone/drop), FakeKeycloak (axum: JWKS, token, device, admin), mint_token(), Mailpit client

apps/api/                       crate spec-store-api (bin spec-store-api)
  src/main.rs                   config, preflight, migrate, serve, outbox sender, graceful SIGTERM; subcommand `openapi`
  src/router.rs                 /api/v1 routes (utoipa-axum), /auth routes, /healthz, /readyz, host allow-list, body limit
  src/auth.rs                   Caller extractor: session cookie (+ CSRF header) or bearer token (audience spec-store-api)
  src/bff.rs                    /auth/login, /auth/callback, /auth/logout, /auth/backchannel-logout, session refresh
  src/routes/{me,organizations,invitations,members,projects,specs,documents,history,audit,transfer,clients}.rs
  src/error.rs                  StoreError -> HTTP status + {code, message}
  src/outbox.rs                 sender loop
  tests/                        http_* integration tests

apps/mcp/                       crate spec-store-mcp (bin spec-store-mcp)
  src/main.rs                   config, preflight, serve, graceful SIGTERM
  src/router.rs                 /mcp (StreamableHttpService), /.well-known/oauth-protected-resource[/mcp], /healthz, /readyz
  src/auth.rs                   bearer middleware: verify audience = MCP URL, insert Caller; 401 with resource_metadata
  src/tools.rs                  the 16 tools of src/mcp.rs, `project` = "<org>/<project>"
  tests/                        mcp_* integration tests (rmcp client over HTTP)

apps/cli/                       crate spec-store-cli (bin spec-store)
  src/main.rs, src/cli.rs       clap commands (§6.3)
  src/login.rs                  device flow against Keycloak, keyring / 0600 file storage
  src/client.rs                 reqwest client to the API with bearer and transparent refresh
  src/transfer.rs               import: tree::scan -> POST; export: GET -> tree::plan_export/write_export
  tests/                        cli_* tests against the API in-process + FakeKeycloak

apps/landing/                   React Router 7 framework mode, ssr:false, prerender ["/", "/es"]
  app/root.tsx, app/routes/home.tsx, app/i18n/{en,es}.ts, app/lib/i18n.ts, react-router.config.ts, vite.config.ts
apps/app/                       Vite SPA, React Router 7 data mode
  src/main.tsx, src/router.tsx  routes and loaders (401 -> /auth/login?return_to=)
  src/api/schema.d.ts           generated from the API OpenAPI document (committed)
  src/api/client.ts             openapi-fetch, credentials same-origin, X-Spec-Store-Request header
  src/routes/                   onboarding, organization, members, invitations, projects, project, spec, document, history, revision, diff, audit, profile (authorized clients)
  src/components/Markdown.tsx   react-markdown + remark-gfm, no raw HTML
  src/lib/{i18n,dates,language}.ts
  src/i18n/{en,es}.ts
packages/ui/                    shadcn components, src/styles/tokens.css (design tokens), src/styles/globals.css (Tailwind @theme from tokens)

deploy/
  api/Dockerfile, mcp/Dockerfile        debian:trixie-slim + prebuilt binary
  app/Dockerfile, landing/Dockerfile    nginx:1.27-alpine + built assets
  spa.conf                               SPA nginx config (lightchat)
  landing.conf                           static site + language redirect map (R17)
  keycloak/Dockerfile                    keycloak 26.8.1, theme copied, kc.sh build, start --optimized
  keycloak/themes/spec-store/{login,email}/   FreeMarker theme, messages_en/es, CSS from tokens.css
  keycloak/realm/spec-store.json         realm template with placeholders (clients, scopes, lifetimes, i18n, SMTP)
  keycloak/realm/dev-users.json          seeded test person (development only)
  postgres/init.sql                      create databases platform and keycloak with their roles
  backup/Dockerfile, backup/backup.sh, backup/restore-check.sh   postgres:18 + rclone

spec-store-cd (separate repository, created by the user)
  application.yaml, base/ (api, mcp, app, landing, keycloak deployments+services, postgres statefulset+service, PDBs, backup and restore-check cronjobs, keycloak-config job), staging/ and prod/ overlays (namespace, configMapGenerator, ingress, images), README with secret bootstrap
```

## 5. Data model

PostgreSQL, migration `0001_platform` (§4). All keys `bigint GENERATED ALWAYS AS IDENTITY`, times `timestamptz NOT NULL DEFAULT now()`.

- `users`: `identity_subject text UNIQUE NOT NULL` (Keycloak `sub`), `handle text UNIQUE NOT NULL` (preferred_username, FR-8), `email text NOT NULL`, `name text NOT NULL`, `created_at`, `updated_at`. Platform operator is not stored: it comes from the realm role `platform-operator` in the token on every request.
- `organizations`: `slug text UNIQUE` (validate_slug rules, FR-1), `name`, `created_at`.
- `organization_members`: `(organization_id, user_id)` PK, `role text CHECK IN ('owner','member')` (FR-3).
- `invitations`: `organization_id`, `email text` (lowercased), `role` = member, `token_hash text UNIQUE` (SHA-256 of a random secret in the link), `inviter_id`, `state CHECK IN ('pending','accepted','declined','revoked')`, `expires_at` (= created + 7 days), `accepted_by`, `created_at`, `updated_at`; expiry is computed, not stored as a state (FR-2).
- `projects`: `organization_id NOT NULL`, `slug`, `name`, `repository_url`, `UNIQUE (organization_id, slug)` (FR-4).
- `memberships` (project roles): `(project_id, user_id)` PK, `role CHECK IN ('owner','editor','reader')`; core rejects a grant to a non-member of the organization (FR-4).
- `specs`, `documents`, `revisions`: as migrations 0001/0002 today, with `UNIQUE NULLS NOT DISTINCT (project_id, spec_id, kind)` replacing the partial index (R12); `revisions.body text` (NFR-2).
- `audit_log`: `actor_id`, `organization_id NULL`, `project_id NULL`, `action`, `target`, `created_at`; indexes on `(organization_id, id DESC)` and `(project_id, id DESC)` (FR-17).
- `browser_sessions`: `secret_hash text UNIQUE` (SHA-256 of the cookie value), `user_id`, `identity_session_id text` (Keycloak `sid`, for back-channel logout), `refresh_token_sealed bytea`, `id_token_sealed bytea`, `access_expires_at`, `created_at`, `ended_at` (FR-6, FR-7, NFR-4).
- `login_attempts`: `state_hash UNIQUE`, `pkce_verifier_sealed bytea`, `return_to text`, `created_at`; rows older than 10 minutes are refused and swept.
- `email_outbox`: `kind`, `recipient text`, `language text`, `payload jsonb`, `attempts int`, `next_attempt_at`, `sent_at NULL`, `failed_at NULL`, `created_at`; index on `(next_attempt_at) WHERE sent_at IS NULL AND failed_at IS NULL` (FR-2).

Removed tables: `api_tokens`, `sessions` (token-based UI sessions).

Validations reused: slugs (spec 001 FR-16 rules, now also organizations), repository URL, body size 1 MB plus NUL rejection, status transitions, task line format.

## 6. Contracts / interfaces

### 6.1 HTTP API (`apps/api`)

JSON over `https://api-spec-store.eduindev.com` (bearer, CLI) and `https://app-spec-store.eduindev.com/api` (session cookie, app). Errors: `{ "code": "...", "message": "..." }` with 401 `unauthenticated`, 403 `forbidden`, 404 `not_found` (also for other tenants, FR-5), 409 `conflict` (body adds `current_revision`), 422 `validation`, 503 `unavailable`. Every state-changing request authenticated by cookie must send `X-Spec-Store-Request: 1`, else 403.

| Method and path | FR |
|---|---|
| `GET /api/v1/me` → handle, email, name, is_operator, language | FR-8 |
| `PUT /api/v1/me/language {language}` → sets cookie and Keycloak locale | FR-24, FR-9 |
| `GET /api/v1/me/clients` · `DELETE /api/v1/me/clients/{client_id}` | FR-12 |
| `GET /api/v1/orgs` · `POST /api/v1/orgs {slug,name}` · `GET /api/v1/orgs/{org}` | FR-1, FR-21 |
| `GET/POST /api/v1/orgs/{org}/invitations` · `POST …/{id}/resend` · `DELETE …/{id}` | FR-2, FR-14, FR-36 |
| `GET /api/v1/invitations` (mine) · `POST /api/v1/invitations/{token}/accept` · `…/decline` | FR-2, FR-37 |
| `GET /api/v1/orgs/{org}/members` · `PUT …/{handle} {role}` · `DELETE …/{handle}` | FR-3 |
| `GET/POST /api/v1/orgs/{org}/projects` | FR-4 |
| `GET /api/v1/projects/{org}/{project}` · `PUT/DELETE …/repository-url` · `GET/PUT/DELETE …/members[/{handle}]` | FR-4, FR-14, FR-22 |
| `GET …/specs` · `…/specs/{n}` · `…/specs/{n}/documents/{kind}` · `…/constitution` · `…/revisions` · `…/revisions/{id}` · `…/revisions/{id}/diff` · `…/tasks` · `…/markers` · `…/status` | FR-19, FR-20, FR-22 |
| `GET /api/v1/orgs/{org}/audit?project=&before=` · `GET /api/v1/audit?org=&before=` (operator) | FR-17, FR-22 |
| `POST …/import {tree, constitution_version, dry_run}` → report · `GET …/export?spec=` → documents | FR-15 |
| `GET /auth/login?return_to=` · `GET /auth/callback` · `POST /auth/logout` · `POST /auth/backchannel-logout` | FR-6, FR-7 |
| `GET /healthz` (200 always) · `GET /readyz` (PostgreSQL ping and Keycloak JWKS reachable, else 503) | FR-33 |

The API exposes no endpoint that writes specs, documents, constitutions or statuses to cookie-authenticated callers (FR-38); spec writes stay MCP-only as in spec 001, and import is accepted only with a bearer token (CLI).

### 6.2 MCP (`apps/mcp`)

`POST https://mcp-spec-store.eduindev.com/mcp`. Without or with an invalid token: 401, `WWW-Authenticate: Bearer resource_metadata="https://mcp-spec-store.eduindev.com/.well-known/oauth-protected-resource/mcp", scope="openid spec-store-mcp"` (FR-11). Metadata: `{resource, authorization_servers:[realm issuer], scopes_supported, bearer_methods_supported:["header"]}`. Tools unchanged in name, input and output (spec 001), except `project` is `"<org>/<project>"` and errors keep spec 001 wording (FR-19, FR-20).

### 6.3 CLI (`apps/cli`, binary `spec-store`)

Global `--api <url>` (env `SPEC_STORE_API`, default production). Exit 0 on success, 1 on failure with the message on stderr (spec 003 NFR-2).

`login` · `logout` · `whoami` (FR-13) · `org create <slug> <name>` · `org list` · `org invite <org> <email>` · `org invitations <org>` · `org resend <org> <id>` · `org revoke-invite <org> <id>` · `org member set <org> <handle> <owner|member>` · `org member remove <org> <handle>` · `project create <org/project> <name>` · `project list [<org>]` · `project member set <org/project> <handle> <role>` · `project member remove …` · `project set-repo <org/project> <url>` · `project clear-repo <org/project>` · `audit list <org> [--project <slug>] [--limit N]` (FR-14) · `import <org/project> <dir> [--constitution-version V] [--dry-run]` · `export <org/project> <dir> [--spec N] [--force]` (FR-15).

### 6.4 App screens (`apps/app`)

Onboarding (no organization: create form + pending invitations, FR-37), invitation acceptance `/invitations/:token` (FR-2), organization switcher and projects (FR-21), members and invitations management, project creation, project roles and repository URL (FR-36), project, spec, document, history, revision, diff, audit (FR-22), profile with authorized clients and revoke (FR-12), language switcher (FR-24). No screen writes specs or documents (FR-38).

### 6.5 Makefile targets

`install` (compose up `--wait`, pnpm install), `uninstall` (compose down `-v`, FR-26), `dev` (api, mcp, app, landing in parallel), `lint` (clippy `-D warnings`, `biome lint`, `tsc`, OpenAPI drift check), `fmt-check` (`cargo fmt --check`, `biome format`), `test` (compose postgres+mailpit up, stale test databases swept, `cargo test --workspace --locked`, `pnpm -r test`) (FR-27), `e2e` (Playwright sign-in against compose Keycloak, Lighthouse CI on the landing build), `build-<deliverable>`, `openapi`.

## 7. Technical decisions

| Decision | Discarded alternative | Why |
|---|---|---|
| Workspace: `packages/core` + `apps/{api,mcp,cli}` + `packages/testkit` | A `server-common` crate between core and the binaries | api and mcp share only identity verification and config, which belong to core; one fewer crate |
| Sync Diesel + r2d2, store calls in `spawn_blocking` (R10) | diesel-async | Keeps every existing store function; no requirement needs async SQL |
| `FOR UPDATE` row locks for document writes and spec numbers (R12) | SERIALIZABLE + retry | Deterministic, one-line changes to existing functions |
| Migrations under `pg_advisory_xact_lock` in each binary at start (R11) | Separate migration Job | Same "deploying the image is deploying the migrations" rule as lightchat; no ordering problem |
| Fresh migration `0001_platform`; SQLite migrations and upgrade code removed | Port 0001/0002 + carry-over to PostgreSQL | No store is in use (Clarification 2026-10-22) |
| Application-level tenant scoping through `access_in` (R14) | PostgreSQL RLS | One place for authorization; isolation proven by tests |
| `jsonwebtoken` 11 with local JWKS validation, leeway 0, 5-minute tokens (R4) | Token introspection | No Keycloak round trip per request; still meets FR-10 |
| Platform operator from the realm role in the token | `is_operator` column | No second source of truth; changing it in Keycloak takes effect within one token lifetime |
| Browser sign-in through the API as BFF, same-origin `/api` and `/auth` on the app host (R9) | Public SPA client with tokens in memory | NFR-4 and RFC 10017 |
| `Caller` resolved per transport (cookie or bearer in api, bearer in mcp) feeding the same core functions | Separate core entry points per transport | FR-20: identical rules and errors whatever the interface |
| Refresh/ID tokens sealed with AES-256-GCM via `aws-lc-rs` (already in the tree) | `aes-gcm` crate; unencrypted columns | No new crypto crate; a database dump never contains usable tokens |
| Login state in `login_attempts` table | Signed state cookie | Works across instances without a shared signing key rotation story |
| MCP audience via client scope + Audience mapper, anonymous DCR with trusted hosts (R2) | Experimental resource-indicators / CIMD features | Supported features only |
| Revoke MCP clients through Keycloak Admin API with a service account, offline tokens as the renewal mechanism (R5) | Our own grant table | Keycloak already owns the grant; revocation stops renewal at the source |
| CLI device flow with `reqwest` directly (R6) | `oauth2` crate | `oauth2` pins reqwest 0.12; the flow is two POSTs |
| `keyring` with 0600 file fallback (R6) | File only | FR-13 "only in the user's account": OS keystore where available |
| Email outbox table + in-process sender with `SKIP LOCKED`, `lettre` (R23) | Send inside the request; external queue | FR-2 retries for one hour and never fails the invitation; no new service |
| utoipa + utoipa-axum OpenAPI, generated TS types committed with drift check (R20) | ts-rs; hand-written types | Route-level contract, drift fails `make lint` |
| Markdown rendered in the app with react-markdown + remark-gfm (R19) | Rust HTML from the API | API stays raw markdown; no `dangerouslySetInnerHTML` |
| Landing prerendered to static HTML on nginx (R17) | Node SSR server | FR-25 and NFR-7 with no server runtime |
| React Router 7.18 (R16) | React Router 8.5 | Constitution v2.0 P2 and the request name 7 |
| `packages/ui` with `tokens.css` shared by Tailwind `@theme`, both apps and the Keycloak theme CSS (R7, R16) | Tokens copied per app | One source for FR-9 branding |
| Language cookie `spec_store_lang` on `eduindev.com`, Keycloak `ui_locales` + profile `locale` sync (R7, R18) | localStorage | Shared between landing, app and API; Keycloak profile locale otherwise overrides `ui_locales` |
| Keycloak FreeMarker theme on `keycloak.v2` in a custom image (R7) | Keycloakify | No Node + Maven stage for a few pages |
| keycloak-config-cli Job in the cluster, `--import-realm` in development (R8) | `--import-realm` everywhere | Import skips existing realms; config-cli applies changes |
| One PostgreSQL StatefulSet per environment with two databases (R22) | CloudNativePG; separate instances | Lightchat pattern; backups via CronJob meet NFR-9 |
| Backup and restore-check CronJobs with `pg_dump` + `rclone`, pruning in the job (R22) | Barman / S3 lifecycle | Simple, testable, works with Garage |
| One `deploy-<x>.yml` per deliverable with `on.push.paths`, `release.yml` rebuilds all on tags, `cd-bump.yml` (R21) | Single workflow with paths-filter | Lightchat pattern; public URLs are compiled into bundles so images are rebuilt per environment |
| Secrets created by hand per namespace, documented in the CD README (R22) | Sealed Secrets / SOPS | Lightchat convention, no new controller |
| Test databases cloned from a migrated template (R15), FakeKeycloak in `packages/testkit` | testcontainers; real Keycloak in unit tests | Parallel and fast; Keycloak covered by `make e2e` and manual validation |
| Mailpit in compose for development and tests | Stub sender trait | Real SMTP path tested without an abstraction layer |
| CLI distributed with `cargo install --git … spec-store-cli` documented in the README | Release binaries per platform | Out of scope of the spec; no requirement on distribution |

## 8. Deviations from the constitution

| Deviation | Why it is necessary | Why the simple alternative does not work |
|---|---|---|
| P7: browser requests carry a session cookie, not a Keycloak credential. The cookie maps to a server-side session holding a Keycloak refresh token; the API refreshes the access token when it expires (≤ 5 minutes) and ends the session when Keycloak refuses, so every browser call is still backed by a credential at most 5 minutes old | NFR-4 forbids tokens readable by page scripts; RFC 10017 recommends this pattern | Sending the Keycloak access token from the browser requires holding it in page memory, which NFR-4 forbids |
| P2: Mailpit (development and tests only), an external SMTP provider and the Garage object store are used besides PostgreSQL and Keycloak | FR-2 needs outbound mail and FR-34 storage outside the cluster | They are external providers or local tools, not services the platform runs; running mail or object storage in the cluster would add services instead |
| P7: `/healthz`, `/readyz`, the resource metadata, `/auth/*`, the landing and static assets answer without authentication | FR-11, FR-25, FR-33 require it | These endpoints exist to be reached before authentication |

## 9. Test strategy

- **Core integration** (`packages/core/tests`, PostgreSQL clone per test): every ported spec 001/003 store test (`store_*`, `tree`), plus `organizations.rs`, `invitations.rs`, `tenancy.rs`, `audit.rs`, `outbox.rs`, `concurrency.rs` (two connections racing on a document write, on spec creation and on a first constitution write), `migrations.rs` (two threads migrating an empty database at once, both succeed, one row per version), `body.rs` (CRLF, 1 MB, NUL).
- **Unit** (`#[cfg(test)]` in core): `identity` (valid, expired with leeway 0, wrong audience, unknown `kid` refresh), `crypto` round trip, `address` parsing, `email` templates in both languages, `markdown`, `diff`.
- **API integration** (`apps/api/tests`, in-process router + FakeKeycloak + Mailpit): `http_auth.rs` (bearer, cookie, CSRF header, refresh, disabled user refused on refresh, back-channel logout), `http_bff.rs` (login redirect with `return_to` and `ui_locales`, callback, logout URL), `http_orgs.rs`, `http_invitations.rs` (email in Mailpit within the sender interval, resend, expiry, wrong email), `http_projects.rs`, `http_read.rs` (spec 002 data parity), `http_audit.rs`, `http_transfer.rs`, `http_clients.rs` (list/revoke via admin API), `http_readonly.rs` (no document writes through cookie auth), `http_health.rs` (readyz 503 with the database stopped or JWKS unreachable), `http_hosts.rs`, `http_parity.rs` (same error for the same operation through API and MCP).
- **MCP integration** (`apps/mcp/tests`, rmcp HTTP client): `mcp_auth.rs` (401 header and metadata, wrong audience, expired), `mcp_tools.rs` (the spec 001 MCP suite ported from `tests/mcp_stdio.rs` to HTTP with `org/project`), `mcp_tenancy.rs`, `mcp_replicas.rs` (two server instances on one database, concurrent writes with the same expected revision: one conflict).
- **CLI integration** (`apps/cli/tests`): `cli_login.rs` (device flow with FakeKeycloak, timeout after the code lifespan, credentials file mode 0600 in fallback, logout removes them), `cli_commands.rs`, `cli_transfer.rs` (spec 003 suite ported, round trip byte-identical).
- **Performance** (built with `--release` inside `make test`): `perf_read.rs` (NFR-1 against the API), `perf_import.rs` (NFR-3 through the CLI).
- **Frontend** (Vitest): i18n key parity for app and landing (FR-24), `dates.test.ts` (FR-23 boundary at 7 days, both languages, time zone), `Markdown.test.tsx` (ported render cases: raw HTML literal, unsafe schemes dropped), route loader tests with mocked fetch for 401 redirect (FR-6), onboarding (FR-37), management forms (FR-36), absence of document-write UI (FR-38), landing prerender output contains the text and sign-up/sign-in links (FR-25).
- **E2E** (`make e2e`, compose Keycloak): Playwright sign-up → verify email via Mailpit → app → sign out → protected page asks again (FR-6, FR-7, FR-9 Spanish screenshot); Lighthouse CI ≥ 0.9 performance and accessibility on the landing (NFR-7).
- **Pipeline and cluster checks** (recorded manual checks in `validation.md`): FR-28 failing run on a broken commit, FR-29/FR-30 timings, FR-31 cross-environment token refused, FR-32 revert, FR-34 backup object present, FR-35 first restore-check run, NFR-5 pod deletion under load, NFR-6 HTTP refused.

## 10. FR coverage

| FR | Module/file | Planned test |
|---|---|---|
| FR-1 | core `store/organizations.rs`, api `routes/organizations.rs` | `core/tests/organizations.rs::create_and_duplicate_slug`, `api/tests/http_orgs.rs` |
| FR-2 | core `store/invitations.rs`, `store/outbox.rs`, `email.rs`; api `outbox.rs` | `core/tests/invitations.rs` (expiry, revoked, used, wrong email), `api/tests/http_invitations.rs::email_delivered_within_5_minutes` |
| FR-3 | core `store/organizations.rs` | `core/tests/organizations.rs::role_change_removal_last_owner` |
| FR-4 | core `store/projects.rs`, `address.rs` | `core/tests/tenancy.rs::same_slug_two_orgs`, `::project_role_requires_org_member` |
| FR-5 | core `store/projects.rs::access_in` | `core/tests/tenancy.rs`, `api/tests/http_projects.rs::other_tenant_not_found`, `mcp/tests/mcp_tenancy.rs`, operator read/write cases |
| FR-6 | api `bff.rs`; app `router.tsx` | `api/tests/http_bff.rs::login_returns_to_requested_screen`, `app router.test.tsx`, e2e |
| FR-7 | api `bff.rs` | `api/tests/http_bff.rs::logout_ends_session_and_redirects_to_end_session`, e2e |
| FR-8 | core `store/users.rs` | `core/tests/users.rs::profile_created_then_updated_from_identity` |
| FR-9 | `deploy/keycloak/themes/spec-store`, core `keycloak.rs` (locale sync) | e2e Spanish sign-in screenshot; manual check of the five pages in both languages |
| FR-10 | core `identity.rs`, api `bff.rs` refresh | `identity` unit (leeway 0), `api/tests/http_auth.rs::disabled_user_refused_on_refresh` |
| FR-11 | mcp `auth.rs`, `router.rs` | `mcp/tests/mcp_auth.rs::unauthenticated_gets_resource_metadata` |
| FR-12 | realm lifetimes; api `routes/clients.rs`; core `keycloak.rs` | `api/tests/http_clients.rs::list_and_revoke`, `mcp/tests/mcp_auth.rs::expired_token_rejected`, manual Claude Code check |
| FR-13 | cli `login.rs` | `cli/tests/cli_login.rs` (success, timeout, storage, logout) |
| FR-14 | cli `cli.rs`, `client.rs` | `cli/tests/cli_commands.rs` (one case per command) |
| FR-15 | cli `transfer.rs`, core `tree.rs`, `store/transfer.rs` | `cli/tests/cli_transfer.rs` (spec 003 FR-1–FR-21 ported) |
| FR-16 | core `store/documents.rs` | `core/tests/concurrency.rs`, `mcp/tests/mcp_replicas.rs` |
| FR-17 | core `store/audit.rs` | `core/tests/audit.rs` (entry per action, org owner vs operator visibility) |
| FR-18 | core `db.rs` | `core/tests/migrations.rs::concurrent_start_applies_once` |
| FR-19 | mcp `tools.rs` | `mcp/tests/mcp_tools.rs` (spec 001 suite) |
| FR-20 | api/mcp `error.rs` over core | `api/tests/http_parity.rs` |
| FR-21 | app `routes/organization.tsx`, api `routes/organizations.rs` | `app organization.test.tsx`, `api/tests/http_orgs.rs::list_mine` |
| FR-22 | app `routes/*`, api `routes/{projects,specs,documents,history,audit}.rs` | `api/tests/http_read.rs`, `api/tests/http_audit.rs`, `app` route tests |
| FR-23 | app `lib/dates.ts` | `dates.test.ts` |
| FR-24 | app/landing `lib/i18n.ts`, `i18n/*` | `i18n.parity.test.ts` (both apps), `language.test.ts` (cookie, fallback) |
| FR-25 | landing `routes/home.tsx`, `react-router.config.ts` | `landing prerender.test.ts` (built HTML contains headline and links) |
| FR-26 | `docker-compose.yml`, `Makefile`, `deploy/keycloak/realm/dev-users.json` | manual: `make install` then `make uninstall` on a clean clone; e2e uses the seeded person |
| FR-27 | `Makefile` | manual: each target exits non-zero on an injected failure per package |
| FR-28 | `.github/workflows/ci.yml` | manual pipeline check recorded in validation |
| FR-29 | `deploy-*.yml`, `cd-bump.yml` | manual: merge → staging version changes within 30 min |
| FR-30 | `release.yml` | manual: tag → production version within 30 min |
| FR-31 | CD overlays, realm per namespace | manual: staging token refused by production API and MCP |
| FR-32 | CD repository | manual: revert bump → previous version within 10 min |
| FR-33 | api/mcp `router.rs` | `api/tests/http_health.rs`, `mcp/tests/mcp_health.rs` |
| FR-34 | `deploy/backup/backup.sh`, CD CronJob | `backup.sh` run in `make e2e` against compose PostgreSQL with an rclone local-directory remote; manual production object check |
| FR-35 | `deploy/backup/restore-check.sh`, CD CronJob | same script in `make e2e` (success and tampered-count failure); manual first production run |
| FR-36 | app management routes, api routes | `app management.test.tsx`, `api/tests/http_orgs.rs`, `http_invitations.rs`, `http_projects.rs` |
| FR-37 | app `routes/onboarding.tsx` | `app onboarding.test.tsx` |
| FR-38 | api `auth.rs` (cookie callers cannot reach write routes), app | `api/tests/http_readonly.rs`, `app readonly.test.tsx` |
| FR-39 | CD ingress, `config.rs` hosts | manual: each hostname answers per environment; `api/tests/http_hosts.rs` for configured host checking |

FRs without coverage: 0.

## 11. Risks

- React Router 8 is the current major and 7.x may stop receiving fixes: pin 7.18.x now; an upgrade needs a constitution amendment and a small spec.
- Claude Code may not use offline tokens or may ignore the 30-day idle window (R5, unconfirmed): checked in manual validation; if it fails, FR-12 is revisited with `/sdd-change` before release.
- Keycloak MCP support for recent MCP revisions is labelled experimental: pin 26.8.1, cover the flow in e2e, track release notes before upgrades.
- Moving the whole codebase at once can break existing behaviour silently: the spec 001–003 suites are ported first and must be green on PostgreSQL before any new feature task.
- Garage `Expiration` support unconfirmed: pruning is done by the backup job, not by bucket rules.
- A single-node k3s with local-path volumes means a node loss takes the database down: NFR-9 bounds the loss; restore procedure documented and checked weekly.
- Runner glibc newer than `debian:trixie-slim` would break prebuilt binaries: keep the lightchat runner, check in the first deploy.
- Creating the `spec-store-cd` repository, the GHCR packages, `CD_TOKEN`, the cluster secrets, DNS-free hostnames under the existing wildcard and the Garage bucket are manual steps outside the repository: listed as the first tasks with "Done when" checks.
