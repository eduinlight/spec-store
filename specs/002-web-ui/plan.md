# Technical plan — Spec 002

Spec: `specs/002-web-ui/spec.md` · Constitution: v1.0 · Date: 2026-10-17

## 1. Summary

The read-only UI is a second presentation of the store's existing read path, served by the same axum router under `/ui`. A browser session, created from a pasted API token and stored hashed in SQLite, replaces the bearer header for UI routes; every page handler resolves a `Caller` from the session and calls the same `Store` functions the MCP tools use. Markdown is rendered with `pulldown-cmark` with raw HTML neutralised, diffs come from `similar`, and pages are built by small HTML functions with mandatory escaping. New state is limited to the `sessions` table and a nullable `repository_url` on projects.

## 2. Technical context

- Language/version: Rust 1.97, edition 2024, two-space indentation.
- Framework/platform: `axum` 0.8 (default features: `form`, `query`, `original-uri`), `tokio`, `rmcp` untouched.
- Dependencies added: `pulldown-cmark` 0.13 (`html` feature only, R1), `similar` 3 (`text` feature, R2). No templating or cookie crate (R4, R6).
- Storage: migration `0002_web_ui.sql`: `sessions` table and `projects.repository_url`.
- Tests: `tower::ServiceExt::oneshot` against `build_router` as in `tests/http_auth.rs`; store tests on temp databases; unit tests for escaping, rendering, cookies, diff and redirect validation; CLI tests through the binary; an ignored UI performance test.
- Constraints: every value from the store passes through `escape`; the UI never calls a store write except session create and end; tokens never reach pages, URLs, cookies or logs.
- Unknowns: none left; R1–R9 resolve them.

## 3. Constitution check (gate)

| Principle | Complies | Note |
|---|---|---|
| 1. The spec rules | yes | 25 FRs, clarified, no open markers |
| 2. Stack and limits | yes | Two new crates, each justified with the rejected alternative (R1, R2); no new runtime |
| 3. Tests as a gate | yes | Test task before each implementation task; suite, clippy and fmt per task |
| 4. Simplicity | yes | No template engine, no cookie crate, no cache layer; HTML functions and direct store calls |
| 5. Markdown is the source of truth | yes | Rendering and diffs computed per request from stored bodies; nothing derived is stored |
| 6. Nothing is lost | yes | Migration 0002 only adds; sessions are ended by setting `ended_at`, never deleted; no document write |
| 7. Access is scoped | yes | Session resolves to a user through their token; every page uses `access_in` exactly like the tools; readers never write |
| 8. Reference sources | yes | research.md cites crate sources, RFC 6265 and MDN |
| 9. Language | yes | |
| 10. Human approval | yes | User authorized commits and the merge for this branch on 2026-10-17 |

## 4. Module / file structure

| File | Responsibility |
|---|---|
| `Cargo.toml` | add `pulldown-cmark`, `similar` |
| `src/migrations/0002_web_ui.sql` | `sessions` table; `ALTER TABLE projects ADD COLUMN repository_url TEXT` |
| `src/db.rs` | register the second migration in the ordered list |
| `src/store/sessions.rs` | `create_session(token)`, `session_caller(secret)`, `end_session(secret)`, `SESSION_LIFETIME_DAYS` |
| `src/store/projects.rs` | `repository_url` on `Project`; `set_repository_url(actor, slug, url: Option<&str>)`; `list_members(caller, slug)`; `project(caller, slug)` returning `Project` with access |
| `src/store/users.rs` | `list_own_tokens(caller)` |
| `src/store/mod.rs` | `AuditPage`, `list_audit_page(caller, project: Option<&str>, before: Option<i64>, limit)` with authorization (owner of the project or admin); existing `list_audit` kept for the CLI |
| `src/store/tasks.rs` | `project_overview(caller)`: for each visible project, constitution version, spec count, active spec progress (FR-7) |
| `src/ui/mod.rs` | `UiConfig { secure_cookies }`, `ui_router(store, config)`, `/` redirect, `require_session` middleware, `UiError` mapping `StoreError` to pages |
| `src/ui/cookies.rs` | read the session cookie from `Cookie` headers; build `Set-Cookie` for create and clear |
| `src/ui/html.rs` | `escape`, `layout`, shared fragments (status badge, time, pagination links) |
| `src/ui/render.rs` | markdown to HTML with raw-HTML neutralisation and link scheme filter |
| `src/ui/diff.rs` | line diff of two bodies to HTML rows with hunks |
| `src/ui/pages/login.rs` | login form, `POST /ui/login`, `POST /ui/logout`, `next` validation |
| `src/ui/pages/projects.rs` | projects page, project page (specs, members, repository URL, constitution) |
| `src/ui/pages/specs.rs` | spec page (documents, tasks grouped by phase, markers), document page rendered or raw |
| `src/ui/pages/history.rs` | revision list, revision page, diff page |
| `src/ui/pages/audit.rs` | project audit page, global audit page |
| `src/ui/pages/profile.rs` | profile with own tokens |
| `src/ui/style.css` | plain stylesheet served via `include_str!` at `/ui/style.css` |
| `src/server.rs` | merge `ui_router`; `ServerConfig::secure_cookies` |
| `src/cli.rs` | `serve --secure-cookies`; `project set-repo SLUG [URL] [--clear] --as` |
| `README.md` | "Browser view" section |
| `tests/store_sessions.rs`, `tests/store_projects.rs`, `tests/store_users.rs`, `tests/store_audit.rs`, `tests/cli.rs` | store and CLI tests |
| `tests/ui_session.rs`, `tests/ui_projects.rs`, `tests/ui_specs.rs`, `tests/ui_history.rs`, `tests/ui_audit.rs`, `tests/ui_readonly.rs`, `tests/perf_ui.rs` | router tests and performance |

## 5. Data model

Migration `0002_web_ui.sql`:

```sql
ALTER TABLE projects ADD COLUMN repository_url TEXT;

CREATE TABLE sessions (
  id INTEGER PRIMARY KEY,
  secret_hash TEXT NOT NULL UNIQUE,
  token_id INTEGER NOT NULL REFERENCES api_tokens(id),
  user_id INTEGER NOT NULL REFERENCES users(id),
  created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  expires_at TEXT NOT NULL,
  ended_at TEXT
);
```

- Session (FR-2, FR-4, FR-5): the cookie value is `sss_` + base64url of 32 random bytes; only its SHA-256 hex is stored. `session_caller` joins `api_tokens` and returns `Unauthenticated` when `ended_at` is set, `expires_at <= now`, or the token has `revoked_at`; on success it updates `api_tokens.last_used_at`. `expires_at = created_at + 30 days` (spec assumption). `end_session` sets `ended_at`.
- Repository URL (FR-16): nullable text; validated as absolute `http://` or `https://` with a host; one trailing `/` trimmed when building links (R9). Audit action `project.repo`, target `<slug>` or `<slug> cleared`.
- Members (FR-25): read from `memberships` joined with `users`, ordered owner, editor, reader, then handle.
- Audit page (FR-20, FR-21): cursor pagination by `id < before`, 50 per page, plus `has_more`; authorization: admin for any scope, owner for their project scope only; others `Forbidden`.
- Project overview (FR-7): per visible project the fields of FR-7; uses `project_status` internals per project.

## 6. Contracts / interfaces

Routes (all under the existing router; `Host` protection as today):

| Method and path | Auth | Response | FRs |
|---|---|---|---|
| `GET /` | none | 303 to `/ui/` | — |
| `GET /ui/style.css` | none | stylesheet | — |
| `GET /ui/login[?next=]` | none | login form; `next` echoed only when valid (R8) | FR-1, FR-3 |
| `POST /ui/login` (`token`, `next`) | none | valid → `Set-Cookie`, 303 to `next` or `/ui/`; invalid → 200 login page with error, no cookie | FR-2, FR-3 |
| `POST /ui/logout` | session | ends session, clears cookie, 303 to `/ui/login` | FR-5 |
| `GET /ui/` | session | projects page | FR-7 |
| `GET /ui/profile` | session | profile and own tokens | FR-6, FR-24 |
| `GET /ui/audit[?project=&before=]` | session, admin | global audit page | FR-21, FR-22 |
| `GET /ui/p/{slug}` | session, member or admin | project page | FR-9, FR-14, FR-25 |
| `GET /ui/p/{slug}/audit[?before=]` | session, owner or admin | project audit page | FR-20, FR-22 |
| `GET /ui/p/{slug}/constitution[?view=raw]` | session | constitution rendered or raw | FR-11, FR-12 |
| `GET /ui/p/{slug}/constitution/history` | session | revisions | FR-17 |
| `GET /ui/p/{slug}/specs/{number}` | session | spec page | FR-10, FR-13, FR-14, FR-15 |
| `GET /ui/p/{slug}/specs/{number}/{kind}[?view=raw]` | session | one document | FR-11, FR-12 |
| `GET /ui/p/{slug}/specs/{number}/{kind}/history` | session | revisions | FR-17 |
| `GET /ui/p/{slug}/r/{revision}[?view=raw]` | session | one revision | FR-18 |
| `GET /ui/p/{slug}/r/{revision}/diff` | session | diff against predecessor | FR-19 |

Raw view: `?view=raw` answers `text/plain; charset=utf-8` with the stored body unchanged (FR-11, NFR-5).

Error mapping (`UiError`): no session → 303 to `/ui/login?next=<path>`; `Forbidden` and `NotFound` → 404 "not found" page (FR-8), except audit routes where `Forbidden` → 403 "not allowed" page (FR-22); `Validation`/`Malformed` → 400 page; `Database` → 500 page with no detail.

Cookie: `spec_store_session=<secret>; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000`, plus `Secure` when `secure_cookies`. Clearing uses `Max-Age=0`.

CLI: `spec-store project set-repo demo https://github.com/org/repo --as alice` stores; `--clear` removes; output names the project and the value; validation errors exit 1.

Store API additions: `create_session(&self, token: &str) -> StoreResult<String>` (returns the secret), `session_caller(&self, secret: &str) -> StoreResult<(Caller, i64)>` (caller and token id), `end_session(&self, secret: &str) -> StoreResult<()>`, `set_repository_url(&self, actor: &Caller, slug: &str, url: Option<&str>) -> StoreResult<()>`, `list_members(&self, caller: &Caller, slug: &str) -> StoreResult<Vec<Member>>`, `project(&self, caller: &Caller, slug: &str) -> StoreResult<Project>`, `list_own_tokens(&self, caller: &Caller) -> StoreResult<Vec<TokenInfo>>`, `list_audit_page(&self, caller: &Caller, project: Option<&str>, before: Option<i64>, limit: usize) -> StoreResult<AuditPage>`, `project_overview(&self, caller: &Caller) -> StoreResult<Vec<ProjectOverview>>`.

## 7. Technical decisions

| Decision | Discarded alternative | Why |
|---|---|---|
| Sessions in SQLite, hashed secret in cookie | In-memory map; signed stateless cookie | Constitution 6 single state; logout and revocation must end sessions immediately (R5) |
| Hand-written cookie handling | `axum-extra` cookies | One cookie, twenty lines; avoids three crates (R4) |
| HTML built by functions with `escape` | `askama`, `maud` | Twelve simple pages; no proc-macro build step (R6) |
| `pulldown-cmark` with event rewriting | `comrak`; hand-written renderer | Smallest renderer with tables and task lists; events let us neutralise raw HTML (R1) |
| `similar` grouped hunks | `diff` crate; own Myers | Context grouping and change tags ready to render (R2) |
| `--secure-cookies` flag | Trust `X-Forwarded-Proto` | Spoofable without a trusted-proxy list (R7) |
| `next` restricted to `/ui/` paths | Accept any path | Open redirect prevention (R8) |
| `Forbidden` and `NotFound` both render 404 outside audit | Distinct 403 | FR-8 forbids revealing project existence; audit pages are only reached by members, where FR-22 asks for an authorization error |
| UI under `/ui`, root redirects | UI at `/` | Keeps `/mcp` and `/healthz` free of path collisions, one prefix for the session middleware |
| Cursor pagination `before=<id>` | Offset pagination | Stable under concurrent inserts; the audit log grows at the head |

## 8. Deviations from the constitution

None.

## 9. Test strategy

- Unit tests (in-module): `html::escape` and layout, `render` (tables, task boxes, raw HTML as text, `javascript:` links dropped, CRLF), `diff` (insert, delete, equal context, first revision), `cookies` (several headers, missing, attributes), `login::safe_next`.
- Store tests (temp databases): sessions create, resolve, expire, end, revoked token; members list ordering; repository URL validation, clearing and audit; own tokens; audit page authorization and cursor; project overview.
- Router tests (`oneshot`): login flow and cookie attributes, redirect with `next`, invalid token message without echo, logout, pages contain the expected escaped values, 404 for non-member on existing and missing project with identical bodies, 403 for editor on audit, raw view bytes, revision and diff pages, read-only guarantee (POST/PUT/DELETE to UI paths other than login/logout answer 405 or 404, and revision counts unchanged).
- CLI test: `project set-repo` set, clear, invalid URL, non-owner rejected.
- Leak test: token never in any page, `Set-Cookie`, log line or audit row (extends the tracing capture of `tests/http_auth.rs`).
- Performance: `tests/perf_ui.rs` (ignored) renders the spec page of a 200 KB spec from 10 threads, p95 < 300 ms in release.

## 10. FR coverage

| FR | Module/file | Planned test |
|---|---|---|
| FR-1 | `ui/mod.rs` require_session, `pages/login.rs` | `ui_session`: unauthenticated page → 303 `/ui/login?next=`, login page has no data |
| FR-2 | `store/sessions.rs`, `pages/login.rs` | `store_sessions`: create and resolve; `ui_session`: valid token → cookie + 303 to next; `last_used_at` updated |
| FR-3 | `pages/login.rs` | `ui_session`: invalid token → 200 with "not valid", no `Set-Cookie`, body lacks the submitted value |
| FR-4 | `store/sessions.rs`, `ui/mod.rs` | `store_sessions`: revoked and expired → Unauthenticated; `ui_session`: revoked mid-session → 303 login |
| FR-5 | `store/sessions.rs`, `pages/login.rs` | `ui_session`: logout clears cookie and later request → login |
| FR-6 | `ui/html.rs` layout | `ui_projects`: handle, admin mark, profile and logout controls present |
| FR-7 | `store/tasks.rs` project_overview, `pages/projects.rs` | `store_tasks`: overview fields; `ui_projects`: member sees own project only, admin sees all with marker, empty message |
| FR-8 | `ui/mod.rs` UiError | `ui_projects`: non-member on existing and missing project → identical 404 bodies |
| FR-9 | `pages/projects.rs` | `ui_projects`: project page shows specs, kinds, done/total, active marked, repository URL, constitution link or absence |
| FR-10 | `pages/specs.rs` | `ui_specs`: sections in kind order, missing kinds marked not written, revision metadata |
| FR-11 | `ui/render.rs`, `pages/specs.rs` | `render` unit tests; `ui_specs`: raw view returns exact bytes |
| FR-12 | `ui/render.rs` | `render` unit tests: `<script>` as text, `javascript:` link dropped |
| FR-13 | `pages/specs.rs` | `ui_specs`: grouped tasks table, phase totals, malformed list |
| FR-14 | `pages/specs.rs` | `ui_specs`: commit link with repository URL, plain text without |
| FR-15 | `pages/specs.rs`, `pages/projects.rs` | `ui_specs`: markers list; `ui_projects`: marker count next to the active spec |
| FR-16 | `store/projects.rs` set_repository_url, `cli.rs` | `store_projects`: valid, invalid, clear, audit, editor forbidden; `cli`: set-repo commands |
| FR-17 | `pages/history.rs` | `ui_history`: list newest first, current marked, constitution version shown |
| FR-18 | `pages/history.rs` | `ui_history`: old revision labelled not current with link; unknown id → 404 |
| FR-19 | `ui/diff.rs`, `pages/history.rs` | `diff` unit tests; `ui_history`: diff page marks added and removed lines; first revision says no predecessor |
| FR-20 | `store/mod.rs` list_audit_page, `pages/audit.rs` | `store_audit`: owner page and cursor; `ui_audit`: owner sees entries, pagination links |
| FR-21 | `pages/audit.rs` | `ui_audit`: admin global page with project filter |
| FR-22 | `store/mod.rs`, `pages/audit.rs` | `store_audit`: editor forbidden; `ui_audit`: 403 page and no audit link for editor |
| FR-23 | `ui/mod.rs` routes | `ui_readonly`: writes to UI paths rejected; store counts unchanged after a browsing session |
| FR-24 | `store/users.rs` list_own_tokens, `pages/profile.rs` | `store_users`: own tokens only; `ui_session`: profile shows labels, marks current, no secret |
| FR-25 | `store/projects.rs` list_members, `pages/projects.rs` | `store_projects`: ordering; `ui_projects`: members table, empty message |
| NFR-1 | `pages/specs.rs` | `perf_ui` (ignored, release) |
| NFR-2 | `store/sessions.rs`, `ui/cookies.rs` | `ui_session`: token absent from pages, cookies, logs, audit |
| NFR-3 | all pages | `ui_specs`, `ui_history`: raw, revision and diff reachable by `href` present in HTML |
| NFR-4 | `server.rs` | `ui_session`: `/mcp` still answers with bearer while a session exists |
| NFR-5 | `ui/render.rs` | `ui_specs`: 1 MB document renders and raw view is byte-identical |
| NFR-6 | all | by inspection |

## 11. Risks

- Rendering cost of large documents on every request: measured by `perf_ui`; if p95 exceeds 300 ms, add a per-revision render cache keyed by revision id (revisions are immutable), justified then in the plan.
- Markdown edge cases differing from what GitHub shows: acceptable; the raw view is always available.
- Cookie `SameSite=Lax` blocks the session on cross-site POST but allows top-level GET navigation, which is all the UI needs.
- A session table that grows: sessions are small rows; cleanup of expired rows is a later operational task, not a correctness issue.
