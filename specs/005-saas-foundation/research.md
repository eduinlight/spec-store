# Research — Spec 005

Spec: `specs/005-saas-foundation/spec.md` · Constitution: v2.0 · Date: 2026-10-22

Versions were read from crates.io, npm and the official release pages on 2026-10-22. Local reference repositories: `../lightchat`, `../lightchat-cd`, `../portfolio`. Database behaviours marked "verified" were checked against a throwaway `postgres:18-alpine` (18.6) container.

## R1 — MCP authorization: what the MCP server must implement

- Question: what does the latest MCP authorization specification require from a resource server?
- Finding: the latest revision is 2026-07-28 (server-side rules unchanged since 2025-11-25). The server must publish Protected Resource Metadata (RFC 9728) listing at least one authorization server. Clients look for the `resource_metadata` URL in the 401 `WWW-Authenticate` header first, then `/.well-known/oauth-protected-resource/<path>`, then the root. Header: `Bearer resource_metadata="…", scope="…"`. Invalid or expired token → 401; insufficient scope → 403 `error="insufficient_scope"`. The server must reject tokens whose audience is not itself (RFC 8707) and must not pass tokens through. Client registration preference: pre-registered, Client ID Metadata Documents (CIMD) when advertised, Dynamic Client Registration (DCR, deprecated but supported). PKCE S256 is mandatory; servers should not advertise `offline_access`.
- Source: https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization ; https://modelcontextprotocol.io/specification/2026-07-28/basic/authorization/authorization-server-discovery ; https://modelcontextprotocol.io/specification/2025-11-25/basic/authorization
- Decision: canonical resource `https://mcp-spec-store.eduindev.com/mcp`. Metadata served at `/.well-known/oauth-protected-resource/mcp` and at the root; every 401 carries the full header (FR-11). Rejected: relying on well-known probing only.

## R2 — Keycloak 26 as MCP authorization server

- Question: does Keycloak 26 provide what MCP clients need?
- Finding: since 26.4.0 Keycloak serves RFC 8414 metadata at `/.well-known/oauth-authorization-server/realms/{realm}` (the path-insertion form MCP clients try). OAuth 2.1, RFC 8414, RFC 7591 DCR and RFC 9207 are supported; RFC 8707 resource indicators (`--features=resource-indicators`) and CIMD (`--features=cimd`) are experimental. Without resource indicators the documented way to bind the audience is a client scope with an Audience mapper whose custom audience is the MCP URL. Anonymous DCR is governed by the Client Registration anonymous policies (Trusted Hosts, Allowed Client Scopes, Allowed Registration Web Origins). The reverse proxy must expose `/.well-known/`. Latest release: 26.8.1.
- Source: https://www.keycloak.org/securing-apps/mcp-authz-server ; https://www.keycloak.org/2025/09/keycloak-2640-released ; https://www.keycloak.org/server/reverseproxy ; https://www.keycloak.org/downloads
- Decision: pin Keycloak 26.8.1. Client scope `spec-store-mcp` with an Audience mapper set to the MCP URL, realm default scope so DCR clients get it; anonymous DCR enabled with a trusted-hosts policy. Rejected: the experimental resource-indicators and CIMD features, which may change incompatibly.

## R3 — rmcp server-side authentication

- Question: does rmcp 3.5.1 validate bearer tokens or serve resource metadata on the server side?
- Finding: the `auth` feature is client-side only (OAuth flow, header parsing, credential store); 3.6.0 adds nothing for servers. The official examples (`complex_auth_streamhttp.rs`, `simple_auth_streamhttp.rs`) write their own axum middleware and metadata route around `nest_service("/mcp", …)`. `StreamableHttpService` stores the request `http::request::Parts` in the request context, so anything an axum layer inserts into the extensions is readable from a tool through `ctx.extensions.get::<Parts>()` — which `src/mcp.rs` already does for the caller.
- Source: `rmcp-3.5.1/src/transport/streamable_http_server/tower.rs` lines 1272–1330 and 2088–2094; https://github.com/modelcontextprotocol/rust-sdk/tree/main/examples/servers/src
- Decision: an axum middleware validates the token on every request and inserts the `Caller`; tools keep reading it from `Parts`. Rejected: third-party wrappers (`rmcp-server-kit`, `tower-mcp`), a framework for a few dozen lines.

## R4 — JWT validation in Rust

- Question: which crate validates Keycloak tokens, and does local validation meet the 5-minute disable rule (FR-10)?
- Finding: `jsonwebtoken` 11.1.0 has `JwkSet` and `DecodingKey::from_jwk`, needs a crypto backend (`aws_lc_rs` or `rust_crypto`); rmcp already depends on it with `aws_lc_rs`. Its default `Validation::leeway` is 60 s and `validate_aud` defaults to true. `jwt-authorizer` 0.15 requires axum 0.7; `axum-jwt-auth` 0.7 has little adoption. Keycloak refuses to refresh a disabled user (`invalid_grant "User disabled"`), but an issued access token stays valid until `exp`.
- Source: `jsonwebtoken-11.1.0/src/validation.rs` line 126; https://github.com/keycloak/keycloak/blob/main/services/src/main/java/org/keycloak/protocol/oidc/TokenManager.java
- Decision: `jsonwebtoken` 11 with `aws_lc_rs`, JWKS cache keyed by `kid` (refresh on unknown `kid`, rate-limited, and every 10 minutes), validate `iss`, `aud`, `exp`, RS256, leeway 0. With a 5-minute access token lifespan a disabled person is rejected within 5 minutes. Rejected: introspection on every request (a network call per request, Keycloak becomes a hard runtime dependency); `jwt-authorizer` (axum 0.7).

## R5 — Token lifetimes, renewal and revocation

- Question: which Keycloak settings give 5-minute credentials, 30-day idle renewal and per-client revocation (FR-12)?
- Finding: realm Access Token Lifespan defaults to 5 minutes. Normal refresh tokens are bounded by SSO Session Idle/Max (30 min / 10 h). Offline tokens have Offline Session Idle (default 30 days) and are not tied to the SSO session; with Offline Session Max Limited off they expire only by idleness. Keycloak issues an offline token whenever `offline_access` is among the client's scopes, default scopes included. "Revoke Refresh Token" with Max Reuse 0 rotates refresh tokens. `DELETE /admin/realms/{realm}/users/{id}/consents/{clientId}` revokes the consent and the offline token (needs `manage-users`); with no consent and no offline token it returns 404 and revokes nothing. `GET /admin/realms/{realm}/users/{id}/consents` lists them. Disabled users cannot refresh, offline tokens included.
- Source: https://www.keycloak.org/docs/latest/server_admin/index.html#_timeouts ; https://www.keycloak.org/docs-api/latest/rest-api/index.html ; `UserConsentManager.revokeConsentToClient` and `DefaultClientSessionContext.isOfflineTokenRequested` in keycloak/keycloak main
- Decision: Access Token Lifespan 5 min, Revoke Refresh Token on (max reuse 0), Offline Session Idle 30 days, Offline Session Max Limited off, `offline_access` as a default scope for DCR clients. The API lists and revokes a person's MCP clients through a confidential service-account client `spec-store-api-admin` with `view-users`, `manage-users`, `view-clients`. Rejected: raising SSO Session Idle to 30 days (lengthens every browser session); online refresh tokens with consent revocation (revocation can silently do nothing). Unconfirmed: whether Claude Code keeps using a 30-day offline token — checked in the manual validation.

## R6 — CLI device sign-in and credential storage

- Question: how does the CLI sign in (FR-13) and where are credentials kept?
- Finding: Keycloak supports RFC 8628: `POST /realms/{realm}/protocol/openid-connect/auth/device`, token endpoint with `grant_type=urn:ietf:params:oauth:grant-type:device_code`, toggle "OAuth 2.0 Device Authorization Grant" per client. Defaults: code lifespan 600 s (exactly 10 minutes) and polling interval 5 s. `oauth2` 5.0.0 supports device flow but pins reqwest 0.12 while rmcp uses 0.13. `keyring` 4.2.0 (`v1` API) stores secrets in Keychain, Credential Manager or Secret Service; headless Linux may have no secret service.
- Source: https://www.keycloak.org/docs/latest/server_admin/index.html (Device authorization grant); `OAuth2DeviceConfig.java` in keycloak/keycloak main; crates.io `oauth2` 5.0.0, `keyring` 4.2.0
- Decision: public client `spec-store-cli` with only the device grant enabled; the CLI makes the two device-flow POSTs with `reqwest` 0.13 (already in the tree through rmcp) and polls at the returned interval; credentials in `keyring`, falling back to a `0600` file under the user's config directory when no keystore is available. Rejected: `oauth2` (a second reqwest major); file-only storage (weaker on desktops).

## R7 — Keycloak theme

- Question: how to brand the Keycloak pages in English and Spanish with the product's tokens (FR-9)?
- Finding: `keycloak.v2` (PatternFly 5) is the default login theme since 26.0. A FreeMarker theme with `parent=keycloak.v2` overrides styles, templates and `messages/messages_<locale>.properties`; an `email` theme type is separate. Themes ship as a directory under `/opt/keycloak/themes` or a JAR in `providers/`. Keycloakify 11 builds React themes (with a shadcn/Tailwind starter) but adds a Node + Maven build. Locale resolution order: login-page selector, user profile locale, `ui_locales`, cookie, `Accept-Language`, realm default — so a stored profile locale beats `ui_locales`. Realm internationalization must be enabled with `en` and `es`.
- Source: https://www.keycloak.org/ui-customization/themes ; https://www.keycloak.org/2024/10/keycloak-2600-released.html ; https://docs.keycloakify.dev/deploying-your-theme ; https://www.keycloak.org/docs/latest/server_admin/index.html (Localization)
- Decision: FreeMarker `spec-store` theme (types `login` and `email`), `parent=keycloak.v2`, CSS that maps `packages/ui/src/styles/tokens.css` onto the PatternFly variables, logo and product name in `messages_en`/`messages_es`, copied into the custom image. The API passes `ui_locales` from the language cookie, and writes the person's chosen language to their Keycloak `locale` attribute on change so the profile locale never contradicts it. Rejected: Keycloakify (extra Node + Maven stage and a React bundle per page for a handful of pages).

## R8 — Keycloak realm as code and running it

- Question: how is the realm configured in development and in the cluster, and how does Keycloak run behind the proxy?
- Finding: `--import-realm` imports `/opt/keycloak/data/import` on start and skips a realm that already exists; files accept `${ENV}` placeholders. keycloak-config-cli 6.5.1 applies realm files idempotently. Behind a TLS-terminating proxy Keycloak needs `KC_HTTP_ENABLED=true`, `KC_PROXY_HEADERS=xforwarded`, `KC_HOSTNAME=https://…`; health on management port 9000 (`/health/live`, `/health/ready`, `/health/started`) with `KC_HEALTH_ENABLED=true`. Clustering uses `jdbc-ping` by default and user sessions are persisted in the database, so two replicas need no extra discovery. The official image is built with `kc.sh build` and started with `start --optimized`.
- Source: https://www.keycloak.org/server/importExport ; https://github.com/adorsys/keycloak-config-cli ; https://www.keycloak.org/server/reverseproxy ; https://www.keycloak.org/server/hostname ; https://www.keycloak.org/server/containers ; https://www.keycloak.org/server/caching ; https://www.keycloak.org/observability/health
- Decision: one realm template `deploy/keycloak/realm/spec-store.json` with placeholders. Development: `start-dev --import-realm` plus a seeded test person in a dev-only file. Cluster: keycloak-config-cli as an ArgoCD PostSync Job with the same file. Plain Deployment (1 replica on staging, 2 on production). Rejected: `--import-realm` in the cluster (later changes silently skipped); the Keycloak Operator (CRDs and a controller for one realm).

## R9 — Browser authentication pattern

- Question: how does the app sign in without tokens readable by page scripts (NFR-4)?
- Finding: RFC 10017 (OAuth 2.0 for Browser-Based Applications, August 2026) strongly recommends a Backend-for-Frontend: a confidential client on the server holding the tokens, the browser holding only a session cookie that is `Secure`, `HttpOnly`, preferably `SameSite=Strict`, host-only (`__Host-` prefix), plus CSRF defences beyond SameSite (strict CORS or a required custom header). RP-initiated logout uses `end_session_endpoint` with `client_id`, `id_token_hint`, `post_logout_redirect_uri`, `ui_locales`; back-channel logout posts a signed logout token to the client.
- Source: https://datatracker.ietf.org/doc/rfc10017/ ; https://www.keycloak.org/docs-api/26.7.2/javadocs/org/keycloak/protocol/oidc/endpoints/LogoutEndpoint.html ; https://www.keycloak.org/docs/latest/server_admin/index.html (OIDC logout)
- Decision: the API is the confidential client `spec-store-app` (authorization code + PKCE). The Ingress routes `app-spec-store.eduindev.com/api` and `/auth` to the API so the app and its API share one origin: session cookie `__Host-spec_store_session` (`Secure`, `HttpOnly`, `SameSite=Strict`, `Path=/`), every state-changing request must carry `X-Spec-Store-Request: 1`. Refresh and ID tokens are stored server side encrypted with AES-256-GCM (`aws-lc-rs`, already in the tree through jsonwebtoken). Back-channel logout endpoint ends the matching sessions. Rejected: public client with keycloak-js / oidc-client-ts (tokens readable by scripts); cross-origin API calls with CORS (more configuration than a path rule).

## R10 — PostgreSQL access from Diesel

- Question: sync Diesel with r2d2 or `diesel-async`?
- Finding: diesel 2.3.14, diesel-async 0.9.2. Sync Diesel blocks the calling thread; Tokio recommends `spawn_blocking` for blocking work. diesel-async would make every store function, test helper and the CLI async, and migrations still need sync `diesel_migrations`.
- Source: https://docs.diesel.rs/2.3.x/diesel/r2d2/index.html ; https://github.com/weiznich/diesel_async ; https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html
- Decision: keep sync Diesel with `r2d2::Pool<ConnectionManager<PgConnection>>`; `&mut SqliteConnection` becomes `&mut PgConnection`; async handlers call the store through `tokio::task::spawn_blocking`. Pool size 10 per instance (4 instances × 10 + Keycloak stays below the default `max_connections` 100). Rejected: diesel-async (rewrites every signature for no requirement); direct calls without `spawn_blocking` (worker starvation under load).

## R11 — Concurrent migrations

- Question: is it safe for two instances to migrate at the same time (FR-18)?
- Finding: `diesel_migrations` 2.3.2 takes no lock; each migration runs in its own transaction. Verified: two concurrent `CREATE TABLE IF NOT EXISTS __diesel_schema_migrations` fail with a duplicate key on `pg_type_typname_nsp_index`. `pending_migrations` ignores applied versions it does not know, so an old binary keeps working after a newer migration. `pg_advisory_xact_lock` is released at commit or rollback; the session-level lock survives a rollback and stays on a pooled connection. Verified: with the transaction-level lock the second session waits, then sees the recorded version and applies nothing.
- Source: https://github.com/diesel-rs/diesel/blob/v2.3.14/diesel_migrations/src/migration_harness.rs ; https://www.postgresql.org/docs/18/explicit-locking.html#ADVISORY-LOCKS
- Decision: `db::migrate` runs inside one transaction that first takes `pg_advisory_xact_lock(<constant>)`, then applies the pending migrations; migrations are written expand/contract so old instances keep serving during a rolling deploy. Rejected: a session-level lock (leaks on pooled connections); a separate migration Job (an extra deliverable and ordering problem).

## R12 — Optimistic locking and spec numbers on PostgreSQL

- Question: today `immediate_transaction` serializes all writers. What keeps FR-16 and spec numbering correct under READ COMMITTED on several instances?
- Finding: `check_expected_revision` then `append_revision` (`src/store/documents.rs`) and `max(number)+1` in `create_spec` (`src/store/specs.rs`) race on PostgreSQL. Under READ COMMITTED a row lock makes the second transaction wait and then read the committed value. Verified: two concurrent compare-and-set updates give `UPDATE 1` and `UPDATE 0`; two concurrent `max+1` inserts give one unique violation. PostgreSQL treats NULLs as distinct, so `UNIQUE (project_id, spec_id, kind)` does not deduplicate constitutions; `UNIQUE NULLS NOT DISTINCT` (PG ≥ 15) does.
- Source: https://www.postgresql.org/docs/18/transaction-iso.html#XACT-READ-COMMITTED ; https://www.postgresql.org/docs/18/sql-createtable.html
- Decision: document writes read `documents.current_revision_id` with `FOR UPDATE` inside the write transaction; first writes use `INSERT … ON CONFLICT DO NOTHING` on `UNIQUE NULLS NOT DISTINCT (project_id, spec_id, kind)` then lock the row; spec creation locks the project row `FOR UPDATE` before `max(number)+1`, with the unique constraint as a backstop. Rejected: SERIALIZABLE with a retry wrapper around every write; unique-violation retry loops.

## R13 — Text bodies, timestamps and identifiers

- Question: can PostgreSQL keep bodies byte for byte, and which types replace the SQLite text timestamps?
- Finding: verified: `\r\n` round-trips in `text`; a NUL byte fails with SQLSTATE 22021. Diesel's `chrono` feature maps `DateTime<Utc>` to `timestamptz`; chrono 0.4 is already in the tree through rmcp. Identity columns map to `i64`; PG 18 has native `uuidv7()` if opaque ids are needed later.
- Source: https://www.postgresql.org/docs/18/datatype-character.html ; https://docs.diesel.rs/2.3.x/diesel/sql_types/struct.Timestamptz.html ; https://www.postgresql.org/docs/18/ddl-identity-columns.html
- Decision: `text` bodies; NUL rejected by body validation with a `Validation` error (NFR-2); `timestamptz DEFAULT now()`, read through a SQL function `iso_utc(timestamptz)` (migration 0001) that returns RFC 3339 with milliseconds and `Z` as today, so every `created_at` keeps its `String` type in Rust (implemented in T09–T11; diesel `chrono` stays available for time arithmetic); `bigint GENERATED ALWAYS AS IDENTITY` keys, slugs as the public address. Rejected: `bytea` bodies; text timestamps; UUID primary keys.

## R14 — Tenant isolation

- Question: row-level security or application-level scoping?
- Finding: RLS needs a per-request `SET LOCAL` on every pooled connection, a transaction for every read and `FORCE ROW LEVEL SECURITY` to apply to the table owner; authorization would live in two places.
- Source: https://www.postgresql.org/docs/18/ddl-rowsecurity.html
- Decision: application-level scoping through the single access resolver (`access_in`, extended to organization + project), with tenant-isolation tests per interface (FR-5). Rejected: RLS now (possible later as defence in depth).

## R15 — Integration tests against PostgreSQL

- Question: how do tests get isolated databases while staying parallel?
- Finding: `begin_test_transaction` needs a pool of one connection and cannot test concurrency; per-test schemas complicate `search_path` and the migration table; testcontainers starts a container per binary. `CREATE DATABASE … TEMPLATE` copies a migrated database in milliseconds. lightchat uses one database per test with a stale-database sweep (`apps/api/tests/mongo.rs`, Makefile `test-integration-sweep`).
- Source: https://www.postgresql.org/docs/18/manage-ag-templatedbs.html ; `../lightchat/Makefile`
- Decision: a test-support crate creates and migrates `spec_store_tpl` once per test binary (under an advisory lock), each test clones it into `spec_store_test_<pid>_<n>` and drops it at the end; `make test` starts the compose PostgreSQL and Mailpit first and sweeps stale test databases. Missing `DATABASE_URL` fails the test, it does not skip. Migration-race tests use an empty database. Rejected: `#[ignore]` gating (the gate of constitution P3 must run everything).

## R16 — Frontend versions and layout

- Question: which versions and workspace layout?
- Finding: react-router 8.5.0 is the current major; the 7.x line is still patched (7.18.4). react 19.3.1, vite 8.3.5, tailwindcss 4.3.3, shadcn CLI 4.22.1, pnpm 12.15.0, i18next 26.5.5, react-i18next 17.1.2, dayjs 1.11.23, @biomejs/biome 2.6.2, vitest 5.0.5. shadcn supports a pnpm monorepo with a shared `packages/ui` (`@workspace/ui`), Tailwind 4 with `@source` entries for workspace packages. The portfolio project uses React Router 7 + Biome 2 + Tailwind 4 + i18next + Vitest, Node 22.12 (too old for current shadcn templates, which require ≥ 22.22).
- Source: https://ui.shadcn.com/docs/monorepo ; https://tailwindcss.com/docs/detecting-classes-in-source-files ; https://reactrouter.com/upgrading/v7 ; `../portfolio/package.json`, `../portfolio/biome.json`
- Decision: React Router 7.18.x as constitution v2.0 P2 and the request require; Node 24 LTS; pnpm workspace `apps/landing`, `apps/app`, `packages/ui`; one root `biome.json` with 2-space indentation. Rejected: React Router 8 (needs a constitution amendment; tracked as a risk); Turborepo (two apps do not need it).

## R17 — Landing rendering

- Question: SSR server or prerendered HTML for FR-25 and NFR-7?
- Finding: React Router framework mode with `ssr: false` and `prerender: ["/", "/es"]` writes static HTML per route at build time; loaders run at build time.
- Source: https://reactrouter.com/how-to/pre-rendering
- Decision: prerender `/` (English) and `/es` (Spanish) and serve them with nginx; an nginx `map` on the language cookie then `Accept-Language` redirects first visits from `/` to `/es`. Rejected: `@react-router/serve` on Node (a server runtime for two static pages).

## R18 — App routing, data, languages and dates

- Question: how does the app route, fetch, translate and format dates (FR-23, FR-24)?
- Finding: React Router data mode (`createBrowserRouter` with loaders and actions) revalidates after actions. i18next-browser-languagedetector reads and writes a cookie with a `cookieDomain` option; i18next `CustomTypeOptions` makes unknown keys a type error; the portfolio types `es` as `Translation` so a missing key fails `tsc`. dayjs `relativeTime` + `localizedFormat` + `locale/es`; ISO timestamps with `Z` are shown in the browser time zone without the timezone plugin.
- Source: https://reactrouter.com/start/data/routing ; https://github.com/i18next/i18next-browser-languageDetector ; https://www.i18next.com/overview/typescript ; https://day.js.org/docs/en/plugin/relative-time ; `../portfolio/app/lib/i18n.tsx`
- Decision: data mode, loaders call the API, 401 redirects to `/auth/login`. Language cookie `spec_store_lang` on `Domain=eduindev.com` (`SameSite=Lax`, `Secure`, one year), detection order cookie then navigator, fallback `en`; `es` typed as the `en` shape plus a Vitest key-parity test. Dates: `fromNow()` under 7 days, `format('LLL')` beyond, locale per call. Rejected: TanStack Query/Router now; localStorage (per origin, cannot be shared with the landing).

## R19 — Markdown in the app

- Question: render markdown in the browser or keep the Rust renderer?
- Finding: react-markdown 10.1.0 turns raw HTML nodes into text (shown literally) and its default `urlTransform` allows only relative URLs and `http`, `https`, `irc`, `ircs`, `mailto`, `xmpp`; remark-gfm 4.0.1 adds tables and task lists. This matches spec 002 FR-11 and FR-12 as implemented in `src/ui/render.rs`.
- Source: https://github.com/remarkjs/react-markdown (README "Security", `lib/index.js`)
- Decision: render in the app with react-markdown + remark-gfm, no rehype-raw; the API returns raw markdown (which agents need anyway); the cases of `src/ui/render.rs` tests are ported to Vitest. Rejected: server-rendered HTML injected with `dangerouslySetInnerHTML`.

## R20 — API contract between Rust and TypeScript

- Question: how do the app's types follow the API?
- Finding: utoipa 6.0.0 + utoipa-axum 0.3.0 (axum ^0.8.4) generate an OpenAPI document from the router; openapi-typescript 7.13.0 generates types and openapi-fetch 0.17.0 is a typed fetch client that accepts `credentials`. ts-rs 12 types data shapes only.
- Source: https://docs.rs/utoipa-axum/latest/utoipa_axum/ ; https://openapi-ts.dev/openapi-fetch/
- Decision: utoipa + utoipa-axum; `spec-store-api openapi` prints the document; `apps/app/src/api/schema.d.ts` is generated and committed; `make lint` fails when it differs from a fresh generation. Rejected: ts-rs (no route contract); hand-written types (drift).

## R21 — Repository, images and pipelines (lightchat conventions)

- Question: which lightchat conventions are copied?
- Finding: `Makefile` as single entry point (`-include .env`, `lint`, `fmt-check`, `test`, `docker-up`/`docker-down`, `install`/`uninstall`); `docker-compose.yml` with healthchecks and `--wait` for dev services only; whitelist `.dockerignore`; Rust binaries built on the self-hosted runner and copied into `debian:trixie-slim`; SPA in `nginx:1.27-alpine` with `deploy/spa.conf`; one `deploy-<app>.yml` per deliverable with `on.push.paths`, images `ghcr.io/eduinlight-org/<name>:<sha>` and `:latest`; reusable `cd-bump.yml` (`kustomize edit set image`, rebase-and-retry, secret `CD_TOKEN`); `release-apps.yml` checks the tag equals `[workspace.package].version` and rebuilds every image because public URLs are compiled into the bundles; `ci.yml` on push to main and tags only. lightchat runs migrations in-process before binding, `/healthz` static and `/readyz` pinging dependencies, graceful shutdown on SIGTERM.
- Source: `../lightchat/Makefile`, `../lightchat/docker-compose.yml`, `../lightchat/.dockerignore`, `../lightchat/deploy/`, `../lightchat/.github/workflows/{ci,deploy-api,deploy-web,cd-bump,release-apps}.yml`, `../lightchat/apps/api/src/main.rs`, `../lightchat/apps/api/src/interface/http/health.rs`
- Decision: copy these conventions; the cd-bump follows the YAML that runs (`CD_TOKEN`, rebase-and-retry) rather than the lightchat-cd README, which disagrees with it.

## R22 — Cluster layout, secrets and backups

- Question: how are PostgreSQL, Keycloak and the backups run in the cluster (FR-31, FR-33 to FR-35, NFR-5, NFR-9)?
- Finding: lightchat-cd: `application.yaml` (AppProject + one Application per overlay, automated prune/self-heal, retry), `base/` + `prod/` + `staging/`, `configMapGenerator` for non-secret env, `images:` pins, Traefik Ingress without TLS (TLS ends on the VPS), StatefulSet with local-path PVC and `ignoreDifferences` on `volumeClaimTemplates`, secrets created by hand with `kubectl create secret` and never in git. No preStop, no PDB in lightchat. `postgres:18` moved `PGDATA` to `/var/lib/postgresql/18/docker` and declares `VOLUME /var/lib/postgresql`. CloudNativePG 1.30 backups need the Barman Cloud plugin and cert-manager (three cluster-wide installs). Garage implements only part of S3 lifecycle; whether `Expiration.Days` is honoured is unconfirmed. `pg_dump` takes a consistent snapshot; CronJobs report failed Jobs, which ArgoCD shows as Degraded. PDB `policy/v1` with `unhealthyPodEvictionPolicy` is stable since Kubernetes 1.31.
- Source: `../lightchat-cd/README.md`, `../lightchat-cd/application.yaml`, `../lightchat-cd/base/`, `../lightchat-cd/prod/`; https://github.com/docker-library/docs/blob/master/postgres/content.md ; https://cloudnative-pg.io/plugin-barman-cloud/docs/installation/ ; https://garagehq.deuxfleurs.fr/documentation/reference-manual/s3-compatibility/ ; https://www.postgresql.org/docs/18/app-pgdump.html ; https://kubernetes.io/docs/concepts/workloads/controllers/cron-jobs/ ; https://kubernetes.io/docs/tasks/run-application/configure-pdb/
- Decision: one `postgres:18` StatefulSet per environment (volume at `/var/lib/postgresql`) with databases `platform` and `keycloak` and a role each, created by an init script; secrets by hand as in lightchat. Backup CronJob daily at 03:00 UTC: `pg_dump -Fc` per database plus `pg_dumpall --globals-only` and a `meta.json` with counts taken in the same snapshot, uploaded with `rclone` to `spec-store-prod-backups` on Garage, pruning objects older than 30 days in the job. Restore-check CronJob weekly: restore the latest dump into a throwaway PostgreSQL sidecar, compare counts, exit 1 on mismatch. PDB `minAvailable: 1` for every two-replica Deployment in production, `preStop` sleep 5 s and `terminationGracePeriodSeconds: 30`. Rejected: CloudNativePG (three cluster-wide installs, still a custom restore check); S3 lifecycle rules (unconfirmed in Garage); Sealed Secrets (no local precedent).

## R23 — Outbound email

- Question: how does the platform send invitation emails with retries for one hour (FR-2)?
- Finding: `lettre` 0.11 is the maintained Rust SMTP client (async with Tokio, rustls). With several API instances, `SELECT … FOR UPDATE SKIP LOCKED` lets each instance claim different rows of a queue table. Mailpit provides an SMTP sink with an HTTP API for local development and tests.
- Source: https://docs.rs/lettre/latest/lettre/ ; https://www.postgresql.org/docs/18/sql-select.html#SQL-FOR-UPDATE-SHARE ; https://mailpit.axllent.org/docs/api-v1/
- Decision: invitations write an `email_outbox` row in the same transaction; an in-process sender in each API instance claims due rows with `SKIP LOCKED` every 10 s, sends with `lettre`, retries with backoff until one hour after creation, then marks the row failed and logs it without the address in clear. Mailpit in docker compose for development and tests. Rejected: sending inside the request (a mail outage would fail the invitation); an external queue (a new service).
