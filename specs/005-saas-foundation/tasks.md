# Tasks — Spec 005

Plan: `specs/005-saas-foundation/plan.md` · Date: 2026-10-22

Format: `- [ ] Tnn [P] (FR-x, FR-y) Description with path — Done when: verifiable criterion`
`[P]` = can run in parallel with other `[P]` tasks of the same phase (no shared files).
Each task < 30 minutes. Tests before implementation.

Story order: US-1 to US-6 are all P1. They run in dependency order: US-5 (the MCP agent workflow on the new store), then US-3 (agent authorization), US-1 (organizations), US-4 (CLI), US-2 (human sign-in), US-6 (browser app). Then the P2 stories US-7, US-8, US-9.

Switch group (as in spec 004): T08–T17 change the persistence layer from SQLite to PostgreSQL, so `packages/core` does not compile between them. The full gate (`make lint`, `make fmt-check`, `make test`) runs at T17, and T08–T17 are committed as one commit listing every task. They are marked `[x]` only when T17 passes (constitution principle 3). During the group, `apps/mcp` and `apps/cli` hold moved files but are not workspace members until their phase.

## Phase 1 — Setup
- [x] T01 (FR: —) Convert the repository to a Cargo workspace: `git mv src tests migrations` into `packages/core/` (crate `spec-store-core`, lib plus the existing `spec-store` bin for now), root `Cargo.toml` with `[workspace]`, `[workspace.package]` (version, edition 2024, rust-version 1.97) and `[workspace.dependencies]` — Done when: `cargo test --workspace` is green with the same number of tests as on `main`
- [x] T02 (FR: —) Add `docker-compose.yml` with `postgres:18` (volume at `/var/lib/postgresql`, `deploy/postgres/init.sql` creating databases `platform` and `keycloak` with their roles) and `mailpit`, both with healthchecks; add `.env.dist`; add Makefile targets `docker-up`, `docker-down`, `install`, `uninstall` — Done when: `make install` returns with both services healthy and `make uninstall` leaves no compose volume
- [x] T03 (FR: —) Add the pnpm workspace: root `package.json` (pnpm 12, Node 24 engines), `pnpm-workspace.yaml` (`apps/*`, `packages/*`), `biome.json` (spaces, width 2, Tailwind directives), `tsconfig.base.json` (strict) — Done when: `pnpm install && pnpm exec biome ci .` exit 0
- [x] T04 (FR: —) Create `packages/ui` with shadcn monorepo `components.json`, `src/styles/tokens.css` (colour, radius and font tokens), `src/styles/globals.css` (Tailwind 4 `@theme inline` from the tokens, `@source` for `apps/**`), `src/lib/utils.ts`, and the `button` component — Done when: `pnpm --filter @workspace/ui typecheck` exits 0
- [x] T05 (FR-27) Add Makefile targets `lint` (clippy `-D warnings`, `biome lint`, `pnpm -r typecheck`), `fmt-check` (`cargo fmt --check`, `biome format`) and `test` (compose postgres and mailpit up, `cargo test --workspace --locked`, `pnpm -r test`), plus `scripts/ensure-tools.sh` from lightchat with node and pnpm hints — Done when: `make lint && make fmt-check && make test` exit 0

## Phase 2 — Foundation (blocks the stories)
- [x] T06 (FR: —) Create `packages/testkit` (crate `spec-store-testkit`): `TestDb` builds the template database `spec_store_tpl` once per test binary under an advisory lock, clones `spec_store_test_<pid>_<n>` per test, drops it on `Drop`, and panics when `DATABASE_URL` is missing; add a Makefile step to the `test` target that sweeps stale `spec_store_test_*` databases — Done when: testkit's own test creates, uses and drops a clone, and the sweep removes a leftover database
- [x] T07 (FR: —) Move the old binary-side code out of `packages/core`: `git mv src/mcp.rs apps/mcp/src/tools.rs`, `tests/mcp_stdio.rs apps/mcp/tests/mcp_tools.rs`, `tests/cli_transfer.rs apps/cli/tests/cli_transfer.rs`. Delete `src/ui/`, `src/server.rs`, `src/cli.rs`, `src/main.rs`, `src/auth.rs`, `src/store/sessions.rs`, the `ui_*`, `http_auth`, `cli`, `store_sessions`, `perf_ui`, `perf_open`, `db_pool` and `db_upgrade` tests and the core bin — Done when: `cargo test -p spec-store-core` is green on SQLite and `packages/core` has no `axum`, `rmcp` or `clap` dependency
- [x] T08 (FR-16, FR-18) Rewrite `packages/core/tests/common/mod.rs` on `TestDb` and `PgConnection`. Rewrite `tests/db.rs` (pool, migration history). Write new tests: `tests/migrations.rs` (`concurrent_start_applies_once`: two threads migrate an empty database), `tests/body.rs` (CRLF round trip, 1 MB limit, NUL rejected with a validation error) and `tests/concurrency.rs` (racing document writes with the same expected revision, racing spec creation, racing first constitution write) — Done when: tests written (red: the switch has not happened yet)
- [x] T09 (FR-1, FR-4, FR-17) Replace the SQLite migrations with `packages/core/migrations/0001_platform/up.sql` (every table in plan §5, `UNIQUE NULLS NOT DISTINCT` on documents), regenerate `src/schema.rs`, and rewrite `tests/schema.rs` to compare declared columns with `information_schema.columns` — Done when: no file under `packages/core/migrations` mentions SQLite and `schema.rs` lists every table in plan §5
- [x] T10 (FR-18) Write `src/db.rs`: r2d2 pool of `PgConnection` (size 10) and `migrate`, which takes `pg_advisory_xact_lock` inside one transaction and then runs the pending migrations; remove the carry-over code. Update `src/model.rs`: `Caller { user_id, handle, is_operator }`, `OrgRole`, `StoreError::Unavailable`, and `validate_body` (size and NUL). Switch the diesel features to `postgres`, `r2d2`, `chrono` and drop `libsqlite3-sys` — Done when: `src/db.rs` and `src/model.rs` contain no SQLite type
- [x] T11 (FR-17) Port `src/store/mod.rs`: `with_conn`/`with_tx` on `PgConnection` (READ COMMITTED transaction), `audit` with `organization_id`, `timestamptz` read as RFC 3339 with milliseconds and `Z` — Done when: the file contains no SQLite type or `strftime`
- [x] T12 (FR-4) Add `src/address.rs` (`ProjectRef` parses and formats `<org>/<project>`, with slug validation) and a minimal `src/store/organizations.rs` (`create_organization` making the actor owner, `org_role`) for fixtures and project creation — Done when: both files contain no SQLite type and `ProjectRef` exposes `parse` and `Display`
- [x] T13 (FR-4, FR-5, FR-8) Port `src/store/users.rs`: remove tokens, add `upsert_user_from_identity(subject, handle, email, name)` and `caller_for(subject, is_operator)`. Port `src/store/projects.rs`: `create_project(actor, org, slug, name)`, `access_in(caller, &ProjectRef)` (non-member gets the same not-found, operator gets read access only), and project roles limited to organization members — Done when: both files contain no SQLite type
- [x] T14 (FR-16) Port `src/store/specs.rs` (lock the project row `FOR UPDATE` before `max(number)+1`) and `src/store/documents.rs` (`ensure_document` with `ON CONFLICT DO NOTHING`, then `current_revision_id` read `FOR UPDATE`) — Done when: both files contain no SQLite type
- [x] T15 (FR: —) Port `src/store/tasks.rs` and `src/store/transfer.rs` to `PgConnection` and `ProjectRef` — Done when: both files contain no SQLite type
- [x] T16 (FR: —) Port every remaining `packages/core/tests/store_*.rs`, `tests/tree.rs`, `tests/perf.rs` and `tests/perf_import.rs` to the new fixtures (organization + project via `ProjectRef`, users via `upsert_user_from_identity`), changing no behavioural assertion — Done when: `git diff` of those files touches only fixture and setup code
- [x] T17 (FR-16, FR-18) Close the switch group: fix the remaining compile errors and map errors — Done when: `grep -rni sqlite packages/core` is empty, T08 tests green and the full gate passes
- [x] T18 (FR-10) Write `packages/core/src/identity.rs` unit tests (valid token, expired with leeway 0, wrong audience, wrong issuer, unknown `kid` triggers one JWKS refresh) and `FakeKeycloak` in testkit (JWKS endpoint, `mint_token(subject, handle, audience, roles, ttl)`) — Done when: tests written and red
- [x] T19 (FR-10) Implement `identity.rs` (`JwksCache` with rate-limited refresh, `verify(token, audience) -> Claims` with RS256, leeway 0) and `src/config.rs` (typed settings for database, Keycloak, SMTP, hosts) — Done when: T18 tests green
- [x] T20 (FR-33, FR-39) Create `apps/api` (crate `spec-store-api`) and write `tests/http_health.rs` (`/healthz` 200; `/readyz` 503 with the database unreachable or JWKS unreachable) and `tests/http_hosts.rs` (unknown host refused, configured hosts accepted) — Done when: tests written and red
- [x] T21 (FR-33, FR-39) Implement `apps/api/src/main.rs` (config, preflight, migrate, serve, graceful SIGTERM), `router.rs` (health routes, host allow-list, 2 MB body limit), `error.rs` (`StoreError` → status and `{code, message}`) and `auth.rs` (bearer extractor with audience `spec-store-api` → `Caller`, store calls through `spawn_blocking`) — Done when: T20 tests green

## Phase 3 — US-5 (P1) Agent workflow unchanged
- [x] T22 (FR-19, FR-33) Add `apps/mcp` as a workspace member (crate `spec-store-mcp`). Port `tests/mcp_tools.rs` from stdio to the rmcp HTTP client with a `FakeKeycloak` bearer token and `org/project` addresses, keeping every spec 001 assertion. Add `tests/mcp_health.rs` — Done when: tests written and red
- [x] T23 (FR-19, FR-33) Implement `apps/mcp/src/main.rs`, `router.rs` (`/mcp`, health routes), `auth.rs` (bearer verified with audience = MCP URL, `Caller` in the request extensions) and port `tools.rs` to `ProjectRef` and `spawn_blocking` — Done when: T22 tests green
- [x] T24 (FR-16) Write `apps/mcp/tests/mcp_replicas.rs`: two MCP servers on one database, concurrent `write_document` with the same expected revision — Done when: test green with exactly one success and one conflict naming the current revision

## Phase 4 — US-3 (P1) Agents connect by authorizing in the browser
- [x] T25 (FR-11, FR-12) Write `apps/mcp/tests/mcp_auth.rs`: no token → 401 with `WWW-Authenticate` `resource_metadata` and `scope`; both metadata endpoints return the resource and the realm issuer; wrong audience → 401; token one second past `exp` → 401 — Done when: tests written and red
- [x] T26 (FR-11, FR-12) Implement the protected-resource metadata routes and the full 401 header in `apps/mcp/src/router.rs` and `auth.rs` — Done when: T25 tests green
- [x] T27 (FR-12, FR-13) Write `deploy/keycloak/realm/spec-store.json` with placeholders: clients `spec-store-app` (confidential, code + PKCE, back-channel logout URL), `spec-store-cli` (public, device grant only, code lifespan 600 s), `spec-store-api-admin` (service account with `view-users`, `manage-users`, `view-clients`); client scope `spec-store-mcp` with an Audience mapper as a default scope; anonymous DCR with a trusted-hosts policy; access token 5 min, revoke refresh token, offline idle 30 days, offline max off; `en` and `es` enabled; registration and email verification on; SMTP from env. Add a stock `keycloak` service to compose with `--import-realm` — Done when: with compose up, the realm metadata lists `registration_endpoint` and `device_authorization_endpoint`, and a token minted for a DCR client contains the MCP URL in `aud` (checked with `scripts/check-realm.sh`)
- [x] T28 (FR-12) Write `apps/api/tests/http_clients.rs` (list my authorized clients, revoke one, revoke an unknown client → 404) and `FakeKeycloak` admin endpoints for consents — Done when: tests written and red
- [x] T29 (FR-12) Implement `packages/core/src/keycloak.rs` (service-account token, list and revoke consents) and `apps/api/src/routes/clients.rs` — Done when: T28 tests green

## Phase 5 — US-1 (P1) Organizations own projects
- [x] T30 (FR-1, FR-3, FR-4, FR-5) Write `packages/core/tests/organizations.rs` (create, duplicate slug, role change, removal drops project roles, last owner refused) and `tests/tenancy.rs` (same project slug in two organizations, project role requires organization member, outsider gets the same not-found, operator reads but cannot write) — Done when: tests written and red
- [x] T31 (FR-1, FR-3, FR-4, FR-5) Complete `src/store/organizations.rs` (list mine, members, role change, removal with project-role cleanup, last-owner rule) and the operator rules in `projects.rs` — Done when: T30 tests green
- [x] T32 [P] (FR-2) Write `packages/core/tests/invitations.rs` (create with 7-day expiry, resend keeps expiry, revoke, expired / revoked / used refused, accept with a different verified email refused, decline) and `tests/outbox.rs` (enqueue in the same transaction, claim with SKIP LOCKED by two connections gets different rows, retry until one hour, then failed) — Done when: tests written and red
- [x] T33 [P] (FR-17) Write `packages/core/tests/audit.rs`: one entry per organization, invitation, membership, project and document change, with organization and project; an owner sees their organization's entries; an operator sees all; anyone else is forbidden — Done when: tests written and red
- [x] T34 (FR-2) Implement `src/store/invitations.rs` and `src/store/outbox.rs` — Done when: T32 tests green
- [x] T35 (FR-17) Implement `src/store/audit.rs` (organization and global pages of 50 with `before` cursor) and add the audit calls to organizations, invitations and projects — Done when: T33 tests green
- [ ] T36 (FR-1, FR-2, FR-3, FR-5) Write `apps/api/tests/http_orgs.rs` and `tests/http_invitations.rs` (invitation email visible in Mailpit within 5 minutes with organization, inviter and acceptance link, in the inviter's language; resend; mail service down → invitation recorded and sent after it returns) — Done when: tests written and red
- [ ] T37 (FR-2) Implement `packages/core/src/email.rs` (en/es invitation message, `lettre` SMTP) and `apps/api/src/outbox.rs` (sender loop every 10 s, backoff, failure logged without the address in clear) — Done when: the email tests of T36 are green
- [ ] T38 (FR-1, FR-2, FR-3) Implement `apps/api/src/routes/{organizations,invitations,members}.rs` — Done when: all T36 tests green
- [ ] T39 (FR-4, FR-5, FR-17) Write `apps/api/tests/http_projects.rs` (create, members, repository URL, other tenant 404, operator read 200 and write 403) and `tests/http_audit.rs` — Done when: tests written and red
- [ ] T40 (FR-4, FR-5, FR-17) Implement `apps/api/src/routes/{projects,audit}.rs` — Done when: T39 tests green
- [ ] T41 (FR-5) Write `apps/mcp/tests/mcp_tenancy.rs` (an agent of organization A lists and addresses nothing of organization B) — Done when: test green

## Phase 6 — US-4 (P1) The CLI works against the hosted service
- [ ] T42 (FR-13) Add `apps/cli` as a workspace member (crate `spec-store-cli`, bin `spec-store`); write `tests/cli_login.rs` (device code and address printed, success after approval, timeout error after the lifespan, credentials file mode 0600 in the fallback store, `logout` removes them) and `FakeKeycloak` device endpoints — Done when: tests written and red
- [ ] T43 (FR-13) Implement `apps/cli/src/login.rs` (device flow with `reqwest`, polling interval, keyring with 0600-file fallback) and `login`, `logout`, `whoami` in `cli.rs` — Done when: T42 tests green
- [ ] T44 (FR-14) Write `apps/cli/tests/cli_commands.rs` with one case per command in plan §6.3 except import/export, running against the in-process API — Done when: tests written and red
- [ ] T45 (FR-14) Implement `apps/cli/src/client.rs` (bearer with transparent refresh, exit code 1 and stderr message on error) and the organization, project and audit commands in `cli.rs` — Done when: T44 tests green
- [ ] T46 (FR-15) Port `apps/cli/tests/cli_transfer.rs` to the API (spec 003 FR-1–FR-21, byte-identical round trip) and write `apps/api/tests/http_transfer.rs` (import refused with cookie auth, dry run, owner only) — Done when: tests written and red
- [ ] T47 (FR-15) Implement `apps/api/src/routes/transfer.rs` (bearer only) — Done when: `http_transfer.rs` green
- [ ] T48 (FR-15) Implement `apps/cli/src/transfer.rs` (`tree::scan` → POST, GET → `tree::plan_export` / `write_export`) and the `import` and `export` commands — Done when: `cli_transfer.rs` green

## Phase 7 — US-2 (P1) Sign up and sign in through the identity provider
- [ ] T49 (FR-8) Write `packages/core/tests/users.rs`: first sign-in creates the profile from the verified email, name and username; a later sign-in updates email and name; handle uniqueness — Done when: tests written and red
- [ ] T50 (FR-8) Complete `upsert_user_from_identity` in `src/store/users.rs` — Done when: T49 tests green
- [ ] T51 (FR-6, FR-7) Write unit tests for `src/crypto.rs` (seal/open round trip, a tampered ciphertext refused) and `tests/sessions.rs` (create session, find by secret, end, end by identity session id, login attempt refused after 10 minutes) — Done when: tests written and red
- [ ] T52 (FR-6, FR-7) Implement `src/crypto.rs` (AES-256-GCM with `aws-lc-rs`) and `src/store/sessions.rs` (browser sessions, login attempts, sweep) — Done when: T51 tests green
- [ ] T53 (FR-6, FR-7, FR-10) Write `apps/api/tests/http_bff.rs` (login redirects with PKCE, `return_to` and `ui_locales` from the language cookie; callback sets `__Host-spec_store_session` with `Secure`, `HttpOnly`, `SameSite=Strict`; logout ends the session and redirects to `end_session` with `id_token_hint`; back-channel logout ends matching sessions) and `tests/http_auth.rs` (cookie auth, a state-changing request without `X-Spec-Store-Request` gets 403, an expired access token is refreshed, a disabled user's refresh is refused and the session ends with 401), plus `FakeKeycloak` token and logout endpoints — Done when: tests written and red
- [ ] T54 (FR-6, FR-7) Add code exchange, refresh, logout URL and back-channel token verification to `packages/core/src/keycloak.rs`; implement `apps/api/src/bff.rs` — Done when: `http_bff.rs` green
- [ ] T55 (FR-10) Add the cookie path to `apps/api/src/auth.rs` (session lookup, refresh when the access token has expired, session ended on refusal, CSRF header) — Done when: `http_auth.rs` green
- [ ] T56 (FR-9) Write the `spec-store` login theme in `deploy/keycloak/themes/spec-store/login/` (`theme.properties` with `parent=keycloak.v2`, CSS mapping `packages/ui/src/styles/tokens.css` onto PatternFly variables, logo, `messages_en.properties`, `messages_es.properties`) — Done when: with compose up, screenshots of sign-in, sign-up, verify email, reset password and error pages in en and es show the logo and product colours, saved under `specs/005-saas-foundation/evidence/`
- [ ] T57 (FR-9) Write the `email` theme type (verification and password reset in en and es, with the same logo and colours) — Done when: a verification email captured in Mailpit shows the branding in each language
- [ ] T58 (FR-9) Write `deploy/keycloak/Dockerfile` (keycloak 26.8.1, theme and tokens copied, health and postgres build options, `kc.sh build`, `start --optimized`) and switch compose to it with the realm's login and email theme set to `spec-store` — Done when: `docker compose build keycloak` succeeds and the realm serves the themed sign-in page

## Phase 8 — US-6 (P1) Browser app
- [ ] T59 (FR: —) Scaffold `apps/app`: Vite 8, React Router 7.18 data mode, Tailwind 4 with `@workspace/ui/globals.css`, Vitest with jsdom, scripts `dev`, `build`, `typecheck`, `test` — Done when: `pnpm --filter app build && pnpm --filter app test` exit 0
- [ ] T60 (FR-20) Add utoipa and utoipa-axum to the API router, a `spec-store-api openapi` subcommand, a `make openapi` target that writes `apps/app/src/api/schema.d.ts`, and a drift check in `make lint` — Done when: `make lint` fails after hand-editing `schema.d.ts` and passes after `make openapi`
- [ ] T61 (FR-20, FR-22, FR-38) Write `apps/api/tests/http_read.rs` (me, specs, spec, document, constitution, revisions, revision, diff, tasks, markers, status: same data as spec 002 shows), `tests/http_parity.rs` (the same failing operation gives the same error code and message through the API and MCP) and `tests/http_readonly.rs` (no route writes specs, documents, constitutions or statuses for cookie-authenticated callers) — Done when: tests written and red
- [ ] T62 (FR-20, FR-22, FR-38) Implement `apps/api/src/routes/{me,specs,documents,history}.rs` with diffs from `core::diff` — Done when: T61 tests green
- [ ] T63 (FR-23, FR-24) Write `apps/app` tests: `i18n.parity.test.ts`, `language.test.ts` (browser language on first visit, English fallback, cookie `spec_store_lang` on `eduindev.com` remembered), `dates.test.ts` (relative under 7 days, absolute beyond, both languages, browser time zone) and `Markdown.test.tsx` (ported `src/ui/render.rs` cases: raw HTML literal, unsafe schemes dropped, tables, task boxes) — Done when: tests written and red
- [ ] T64 (FR-23, FR-24) Implement `apps/app/src/lib/{i18n,language,dates}.ts`, `src/i18n/{en,es}.ts` (`es` typed as the `en` shape) and `src/components/Markdown.tsx` — Done when: T63 tests green
- [ ] T65 (FR-9, FR-24) Write `apps/api/tests/http_language.rs` (`PUT /api/v1/me/language` sets the cookie and the Keycloak `locale` attribute) and implement it in `routes/me.rs` and `core::keycloak` — Done when: `http_language.rs` green
- [ ] T66 (FR-6, FR-21, FR-37) Write `apps/app` tests for the router (a 401 from a loader redirects to `/auth/login?return_to=`), onboarding (no organization shows the create form and pending invitations) and the organization switcher with its project list — Done when: tests written and red
- [ ] T67 (FR-6, FR-21, FR-37) Implement `apps/app/src/main.tsx`, `router.tsx`, `api/client.ts` (openapi-fetch, same-origin, request header), `routes/onboarding.tsx` and `routes/organization.tsx` — Done when: T66 tests green
- [ ] T68 (FR-22) Write `apps/app` tests for the project, spec (tasks table with commit links, markers) and document screens, and for the history, revision, diff and audit screens — Done when: tests written and red
- [ ] T69 (FR-22) Implement `apps/app/src/routes/{project,spec,document}.tsx` — Done when: the project, spec and document tests of T68 are green
- [ ] T70 (FR-22) Implement `apps/app/src/routes/{history,revision,diff,audit}.tsx` — Done when: all T68 tests green
- [ ] T71 (FR-12, FR-36, FR-38) Write `apps/app` tests for members and invitations management, invitation acceptance and decline, project creation, project roles, repository URL, the profile's authorized clients with revoke, and the absence of any document-writing control — Done when: tests written and red
- [ ] T72 (FR-36) Implement `apps/app/src/routes/{members,invitations,invitation-accept,project-new,project-settings}.tsx` — Done when: the management tests of T71 are green
- [ ] T73 (FR-12, FR-24, FR-38) Implement `apps/app/src/routes/profile.tsx` (authorized clients, revoke) and the language switcher in the layout — Done when: all T71 tests green
- [ ] T74 (FR-6, FR-7, FR-9) Write `e2e/sign-in.spec.ts` (Playwright, run by `make e2e`): sign up, verify the email through Mailpit, land in the app on the requested screen, sign out, then a protected screen asks for credentials again; a Spanish run screenshots the sign-in page — Done when: `make e2e` sign-in suite green against compose

## Phase 9 — US-7 (P2) Public landing page
- [ ] T75 (FR-24, FR-25) Scaffold `apps/landing` (React Router 7.18 framework mode, `ssr: false`, `prerender: ["/", "/es"]`, Tailwind with `@workspace/ui`) and write `prerender.test.ts` (built `index.html` and `es/index.html` contain the headline, body copy, and sign-up and sign-in links to the identity provider) and the i18n parity test — Done when: tests written and red
- [ ] T76 (FR-24, FR-25) Implement `app/root.tsx`, `app/routes/home.tsx` with placeholder copy in en/es, `hreflang` links, and sign-up and sign-in actions pointing at `/auth/login` on the app host (with `kc_action=register` for sign-up) — Done when: T75 tests green
- [ ] T77 (FR-25, NFR-7) Write `deploy/landing.conf` (static files plus a language redirect map: cookie, then `Accept-Language`) and `deploy/landing/Dockerfile`, and add Lighthouse CI (`lighthouserc.json`, error at 0.9 for performance and accessibility on mobile) to `make e2e` — Done when: `make e2e` Lighthouse step passes on the built landing

## Phase 10 — US-8 (P2) One repository, one entry point
- [ ] T78 (FR-26) Complete `docker-compose.yml` (themed Keycloak with `deploy/keycloak/realm/dev-users.json` seeding a verified test person; mailpit), add a `make dev` target running api, mcp, app and landing, and write the README development section — Done when: on a fresh clone, `make install && make dev` lets the seeded person sign in to the app, and `make uninstall` removes every container and volume
- [ ] T79 (FR-27) Check each gate target against an injected failure in every package (one Rust crate, one TS app), and record the results in `specs/005-saas-foundation/evidence/gate.md` — Done when: the evidence shows a non-zero exit for each injected failure and zero after reverting
- [ ] T80 (FR-28) Write `.github/workflows/ci.yml` (self-hosted runner, `ensure-tools`, `make lint`, `make fmt-check`, `make test`, on push to main and tags) — Done when: a run on main is green and a pushed commit with a clippy warning makes the run fail

## Phase 11 — US-9 (P2) Automatic deployments
- [ ] T81 (FR-31) Manual, by the user: create the `eduinlight-org/spec-store-cd` repository, the `CD_TOKEN` Actions secret, the `ghcr-pull` secret, namespaces `spec-store` and `spec-store-staging` with secrets `postgres-credentials`, `api-credentials`, `mcp-credentials`, `keycloak-credentials` (separate values per environment), and the Garage bucket `spec-store-prod-backups` with `backup-credentials` — Done when: `kubectl get secrets -n spec-store` and `-n spec-store-staging` list every secret, and the bucket accepts an upload with the backup key
- [ ] T82 (FR-29) Write `deploy/{api,mcp}/Dockerfile` (debian:trixie-slim), `deploy/app/Dockerfile` with `deploy/spa.conf` (nginx:1.27-alpine), and the whitelist `.dockerignore` — Done when: each image builds locally from prebuilt artifacts and its container answers its health or index route
- [ ] T83 (FR-29) Write `.github/workflows/cd-bump.yml` (reusable, `kustomize edit set image`, rebase-and-retry, `CD_TOKEN`) and `deploy-{api,mcp,app,landing,keycloak}.yml` with `on.push.paths` — Done when: a push changing only `apps/api` builds and pushes only the api image and commits a new tag to `staging/kustomization.yaml`
- [ ] T84 (FR-30) Write `.github/workflows/release.yml` (tag must equal `[workspace.package].version`, rebuild every image as `:<version>`, `cd-bump` with `overlay: prod`) — Done when: a `v0.2.0` tag pushes every image and the prod overlay pins `0.2.0` for each
- [ ] T85 (FR-31, FR-33) In `spec-store-cd`, write `application.yaml` and `base/`: postgres StatefulSet and Service with an init ConfigMap; Deployments and Services for api, mcp, app, landing and keycloak with readiness and liveness probes, `preStop` sleep 5 s, `terminationGracePeriodSeconds: 30`; PDBs; a keycloak-config-cli PostSync Job — Done when: `kustomize build base` succeeds and `kubeconform` validates the output
- [ ] T86 (FR-31, FR-39) Write the `staging/` and `prod/` overlays (namespace, `configMapGenerator` per deliverable with FR-39 hostnames and the environment's realm, Ingress rules, image pins, staging at 1 replica without PDBs) — Done when: both overlays build, and ArgoCD shows `spec-store-staging` Synced and Healthy with every hostname answering
- [ ] T87 (FR-34, FR-35) Write `deploy/backup/Dockerfile` (postgres:18 + rclone), `backup.sh` (`pg_dump -Fc` per database, `pg_dumpall --globals-only`, `meta.json` counts from the same snapshot, upload, prune older than 30 days) and `restore-check.sh` (restore into a scratch PostgreSQL, compare counts, exit 1 on mismatch), with a `make e2e` step running both against compose and an rclone local remote, plus a tampered-count case — Done when: the `make e2e` backup step passes, and the tampered case exits 1
- [ ] T88 (FR-34, FR-35) Add the backup CronJob (daily 03:00 UTC, `concurrencyPolicy: Forbid`) and restore-check CronJob (weekly) to `prod/`, plus a `deploy-backup.yml` workflow and write the manual restore procedure in the `spec-store-cd` README — Done when: a manually triggered backup Job leaves a dump and `meta.json` for both databases in the bucket, a manually triggered restore-check Job succeeds, and a timed run of the documented procedure on staging is recorded under 4 hours in `evidence/deploy.md`
- [ ] T89 (FR-29, FR-30, FR-31, FR-32, FR-39) Run and record the pipeline and cluster checks in `specs/005-saas-foundation/evidence/deploy.md`: merge to staging within 30 min, tag to production within 30 min, a staging token refused by the production API and MCP, a reverted bump serving the previous version within 10 min, every FR-39 hostname answering, Claude Code connected through browser approval with Keycloak showing an offline session for its client — Done when: each check is recorded with its timestamp and result

## Final phase — Polish and cross-cutting
- [ ] T90 (NFR-1, NFR-3) Write `apps/api/tests/perf_read.rs` (200 KB spec, 10 concurrent callers, p95 < 100 ms in release) and `apps/cli/tests/perf_import.rs` (100 specs × five 20 KB documents in < 30 s) — Done when: both green under `make test`
- [ ] T91 (NFR-4) Write `apps/api/tests/secrets_leak.rs`: after sign-in, refresh, device login and MCP calls, captured logs, audit rows, error bodies and redirect URLs contain none of the issued access tokens, refresh tokens or device codes — Done when: test green
- [ ] T92 (NFR-5, NFR-6) Record in `evidence/deploy.md`: deleting one api pod and one mcp pod during a load of 10 concurrent readers fails no request other than those in flight; plain HTTP and unknown hosts are refused — Done when: both checks recorded with their results
- [ ] T93 (FR: —) Update `README.md` (architecture, development, CLI install with `cargo install --git`, MCP connection with browser authorization) and `CLAUDE.md` (description, commands matching the Makefile, Postgres and Keycloak instead of SQLite) — Done when: every command named in `CLAUDE.md` exists as a Makefile target
- [ ] T94 (FR: —) Remove leftovers: root `spec-store.db*` files from the working tree, `rusqlite`/`libsqlite3-sys` references, `pulldown-cmark` and unused dependencies — Done when: `grep -rni sqlite --exclude-dir=specs --exclude-dir=target .` is empty and the full gate passes

## Dependencies
- Phase 1 blocks everything. T01 → T02 → T05 (Makefile); T03 → T04.
- Phase 2 blocks all stories. T06 → T08; T07 → T08; the switch group T08–T17 runs in order; T18 → T19 → T20 → T21.
- US-5 (T22–T24) needs T21's patterns and T19.
- US-3: T25–T26 need T23; T27 is independent of code; T28–T29 need T21.
- US-1: T30–T35 need Phase 2; T36–T40 need T21 and T34/T35; T41 needs T23 and T31.
- US-4: T42 needs T19; T44–T48 need T38 and T40 (API routes).
- US-2: T53–T55 need T27 (realm) and T52; T56–T58 need T27.
- US-6: T59 needs T04; T60 needs T21; T61–T62 need T40; T65 needs T29; T66–T73 need T60 and T64; T74 needs T55, T58 and T73.
- US-7: T75 needs T04; T77 needs T76.
- US-8: T78 needs T58, T67 and T76; T80 needs T05.
- US-9: T81 is manual and blocks T83–T89; T82 needs T21, T23 and T59; T85–T86 need T58; T87 needs T02; T89 needs T84 and T86.
- Final phase needs all stories; T93–T94 come last.

## Coverage
| FR | Tasks |
|---|---|
| FR-1 | T09, T30, T31, T36, T38 |
| FR-2 | T32, T34, T36, T37, T38 |
| FR-3 | T30, T31, T36, T38 |
| FR-4 | T09, T12, T13, T30, T31, T39, T40 |
| FR-5 | T13, T30, T31, T36, T39, T40, T41 |
| FR-6 | T51, T52, T53, T54, T66, T67, T74 |
| FR-7 | T51, T52, T53, T54, T74 |
| FR-8 | T13, T49, T50 |
| FR-9 | T56, T57, T58, T65, T74 |
| FR-10 | T18, T19, T53, T55 |
| FR-11 | T25, T26 |
| FR-12 | T25, T26, T27, T28, T29, T71, T73 |
| FR-13 | T27, T42, T43 |
| FR-14 | T44, T45 |
| FR-15 | T46, T47, T48 |
| FR-16 | T08, T14, T17, T24 |
| FR-17 | T09, T11, T33, T35, T39, T40 |
| FR-18 | T08, T10, T17 |
| FR-19 | T22, T23 |
| FR-20 | T60, T61, T62 |
| FR-21 | T66, T67 |
| FR-22 | T61, T62, T68, T69, T70 |
| FR-23 | T63, T64 |
| FR-24 | T63, T64, T65, T73, T75, T76 |
| FR-25 | T75, T76, T77 |
| FR-26 | T78 |
| FR-27 | T05, T79 |
| FR-28 | T80 |
| FR-29 | T82, T83, T89 |
| FR-30 | T84, T89 |
| FR-31 | T81, T85, T86, T89 |
| FR-32 | T89 |
| FR-33 | T20, T21, T22, T23, T85 |
| FR-34 | T87, T88 |
| FR-35 | T87, T88 |
| FR-36 | T71, T72 |
| FR-37 | T66, T67 |
| FR-38 | T61, T62, T71, T73 |
| FR-39 | T20, T21, T86, T89 |
