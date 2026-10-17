# Tasks — Spec 002

Plan: `specs/002-web-ui/plan.md` · Date: 2026-10-17

Format: `- [ ] Tnn [P] (FR-x, FR-y) Description with path — Done when: verifiable criterion`
`[P]` = can run in parallel with other `[P]` tasks of the same phase (no shared files).
Each task < 30 minutes. Tests before implementation.

## Phase 1 — Setup
- [x] T01 (FR: —) Add `pulldown-cmark` (html feature only) and `similar` (text feature) to `Cargo.toml`; create `src/ui/mod.rs` with empty modules `cookies`, `html`, `render`, `diff`, `pages` (`login`, `profile`, `projects`, `specs`, `history`, `audit`) and `pub mod ui` in `src/lib.rs` — Done when: `cargo build`, `cargo test` and `cargo clippy --all-targets -- -D warnings` pass

## Phase 2 — Foundation (blocks the stories)
- [x] T02 [P] (FR-2, FR-4, FR-16) Extend `tests/db.rs`: opening a store creates `sessions` with its columns and `projects.repository_url`, and a second open is idempotent — Done when: tests written and red
- [x] T03 (FR-2, FR-4, FR-16) Write `src/migrations/0002_web_ui.sql` (plan §5) and register it in `src/db.rs` — Done when: T02 tests green
- [x] T04 [P] (FR-2, FR-4, FR-5) Write `tests/store_sessions.rs`: `create_session` for a valid token returns an `sss_` secret stored only hashed; `session_caller` returns the caller and token id and updates `last_used_at`; unknown secret, ended session, revoked token and expired session (expiry moved to the past with SQL) give `Unauthenticated`; `end_session` is idempotent — Done when: tests written and red
- [x] T05 (FR-2, FR-4, FR-5) Implement `src/store/sessions.rs` (`create_session`, `session_caller`, `end_session`, `SESSION_LIFETIME_DAYS`) — Done when: T04 tests green
- [x] T06 [P] (FR-6, FR-11, FR-12) Write unit tests in `src/ui/html.rs` (escape of `& < > " '`, layout shows title, handle, admin mark, profile and logout controls) and `src/ui/render.rs` (headings, lists, table, checked and unchecked task boxes, `<script>` and inline tags rendered as text, `javascript:` link dropped, `https:` link kept, CRLF body) — Done when: tests written and red
- [x] T07 (FR-6, FR-11, FR-12) Implement `src/ui/html.rs` and `src/ui/render.rs` — Done when: T06 tests green
- [x] T08 [P] (FR-1, FR-2, FR-19) Write unit tests in `src/ui/diff.rs` (insert, delete and equal rows with line numbers, hunks with 3 lines of context, identical bodies give no rows), `src/ui/cookies.rs` (session cookie from one and from several `Cookie` headers, missing, `Set-Cookie` attributes with and without `Secure`, clearing cookie) and `src/ui/pages/login.rs::safe_next` (`/ui/p/demo` kept, `//evil`, `https://evil`, `/mcp` and empty fall back to `/ui/`) — Done when: tests written and red
- [x] T09 (FR-1, FR-2, FR-19) Implement `src/ui/diff.rs`, `src/ui/cookies.rs` and `safe_next` — Done when: T08 tests green

## Phase 3 — US-1 (P1) Sign in with my token
- [x] T10 (FR-1, FR-2, FR-3, FR-4, FR-5, FR-6, FR-24, NFR-2, NFR-4) Write `tests/ui_session.rs` (router via `oneshot`: `/` and `/ui/` without cookie redirect to `/ui/login?next=`; login page has no project data; valid token sets the cookie with `HttpOnly`, `SameSite=Lax`, `Path=/`, `Max-Age` and `Secure` only with `secure_cookies`, then redirects to `next`; invalid token answers 200 with "not valid", no `Set-Cookie` and no echo; `next` outside `/ui/` falls back; page with cookie shows handle; revoked token redirects to login; logout clears the cookie; `/mcp` still works with the bearer; token absent from bodies, headers, logs and audit; `/ui/profile` lists own tokens marking the current one without secrets) and extend `tests/store_users.rs` with `list_own_tokens` — Done when: tests written and red
- [x] T11 (FR-1, FR-2, FR-3, FR-4, FR-5, FR-6, FR-24, NFR-4) Implement `src/ui/mod.rs` (`UiConfig`, `UiError`, `require_session`, `ui_router`, `/` redirect, `/ui/style.css`), `src/ui/pages/login.rs`, `src/ui/pages/profile.rs`, a first `src/ui/pages/projects.rs` listing `list_projects`, `Store::list_own_tokens` in `src/store/users.rs`, `ServerConfig::secure_cookies` and the merge in `src/server.rs`, `serve --secure-cookies` in `src/cli.rs` — Done when: T10 tests green

## Phase 4 — US-2 (P1) Browse my projects and specs
- [x] T12 [P] (FR-7, FR-9, FR-16, FR-25) Extend `tests/store_projects.rs` (`list_members` ordering owners first and empty list, `set_repository_url` accepts `http`/`https`, rejects other values with Validation, clears with `None`, audits `project.repo`, editor Forbidden, owner and admin allowed, `project()` returns the URL and Forbidden for outsiders) and `tests/store_tasks.rs` (`project_overview` returns per visible project constitution version, spec count and active progress; admin sees all with `membership: None`) — Done when: tests written and red
- [x] T13 (FR-7, FR-9, FR-16, FR-25) Implement `Project::repository_url`, `set_repository_url`, `list_members`, `project` in `src/store/projects.rs` and `project_overview` in `src/store/tasks.rs` — Done when: T12 tests green
- [ ] T14 (FR-16) Extend `tests/cli.rs`: `project set-repo demo https://example.com/repo --as alice`, `--clear`, invalid URL fails, editor `--as` fails, owner `--as` works — Done when: tests written and red
- [ ] T15 (FR-16) Implement `project set-repo` in `src/cli.rs` — Done when: T14 tests green
- [ ] T16 (FR-6, FR-7, FR-8, FR-9, FR-15, FR-25) Write `tests/ui_projects.rs`: projects page shows only the member's projects with role, constitution version or absence, spec count, active spec progress and marker count; admin sees every project marked read-only without membership; empty message; project page shows name, slug, repository URL, constitution link or absence, specs with kinds, done over total, active marked, members owners first and empty members message; non-member gets identical 404 bodies for an existing and a missing project — Done when: tests written and red
- [ ] T17 (FR-7, FR-8, FR-9, FR-15, FR-25) Implement `src/ui/pages/projects.rs` — Done when: T16 tests green
- [ ] T18 (FR-10, FR-11, FR-12, FR-13, FR-14, FR-15, NFR-3, NFR-5) Write `tests/ui_specs.rs`: spec page sections in kind order with not-written marks and revision author, time, message; document page rendered; `?view=raw` returns the exact bytes for a CRLF body and a 1 MB body as `text/plain`; `<script>` shown as text; tasks grouped by phase with totals and malformed list; commit links with repository URL and plain text without; markers list; links to raw, history and constitution present in the HTML — Done when: tests written and red
- [ ] T19 (FR-10, FR-11, FR-12, FR-13, FR-14, FR-15) Implement `src/ui/pages/specs.rs` (spec page, document page, constitution page, raw view) — Done when: T18 tests green

## Phase 5 — US-4 (P2) Follow the history of a document
- [ ] T20 (FR-17, FR-18, FR-19, NFR-3) Write `tests/ui_history.rs`: history lists revisions newest first with author, time, message, current marked and constitution version; revision page labelled not current with link to current; unknown revision gives 404; diff page marks added and removed lines; first revision says it has no predecessor; links between pages present — Done when: tests written and red
- [ ] T21 (FR-17, FR-18, FR-19) Implement `src/ui/pages/history.rs` — Done when: T20 tests green

## Phase 6 — US-5 (P2) See who did what
- [ ] T22 [P] (FR-20, FR-21, FR-22) Extend `tests/store_audit.rs`: `list_audit_page` for an owner returns their project's entries newest first with `has_more` and cursor; owner on another project Forbidden; editor Forbidden; admin without filter sees every project and entries without project; filter by slug; page size respected — Done when: tests written and red
- [ ] T23 (FR-20, FR-21, FR-22) Implement `AuditPage` and `list_audit_page` in `src/store/mod.rs` — Done when: T22 tests green
- [ ] T24 (FR-20, FR-21, FR-22) Write `tests/ui_audit.rs`: owner sees the project audit page with entries and older/newer links; editor gets a 403 page and no audit link on the project page; admin sees the global page with project filter — Done when: tests written and red
- [ ] T25 (FR-20, FR-21, FR-22) Implement `src/ui/pages/audit.rs` and the audit links in the layout and project page — Done when: T24 tests green

## Final phase — Polish and cross-cutting
- [ ] T26 (FR-23) Write `tests/ui_readonly.rs`: `POST`, `PUT` and `DELETE` to every UI path except login and logout answer 405 or 404; after a browsing session over every page the counts of projects, specs, documents, revisions, memberships, users and tokens are unchanged — Done when: tests green (fix any route that lets a write through)
- [ ] T27 (NFR-1) Write `tests/perf_ui.rs` (ignored by default): 10 threads, 200 spec-page requests for a 200 KB spec through the router, assert p95 < 300 ms — Done when: `cargo test --release --test perf_ui -- --ignored` passes
- [ ] T28 (NFR-4, NFR-6, FR: —) Add the "Browser view" section to `README.md` (login, `--secure-cookies`, `project set-repo`) and run the full lint — Done when: README has the section and `cargo clippy --all-targets -- -D warnings && cargo fmt --check` passes

## Dependencies
- Phase 1 blocks everything. Phase 2 blocks all stories: T03 needs T01; T05 needs T03; T07 needs T01; T09 needs T01.
- US-1: T11 needs T05, T07 and T09.
- US-2: T13 needs T03; T15 needs T13; T17 needs T11 and T13; T19 needs T17.
- US-4: T21 needs T19.
- US-5: T23 needs T05; T25 needs T23 and T17.
- Polish: T26 needs T25 and T21; T27 needs T19; T28 needs T26.
- US-4 and US-5 are independent of each other once US-2 is done.

## Coverage
| FR | Tasks |
|---|---|
| FR-1 | T08, T09, T10, T11 |
| FR-2 | T02, T03, T04, T05, T08, T09, T10, T11 |
| FR-3 | T10, T11 |
| FR-4 | T02, T03, T04, T05, T10, T11 |
| FR-5 | T04, T05, T10, T11 |
| FR-6 | T06, T07, T10, T11, T16 |
| FR-7 | T12, T13, T16, T17 |
| FR-8 | T16, T17 |
| FR-9 | T12, T13, T16, T17 |
| FR-10 | T18, T19 |
| FR-11 | T06, T07, T18, T19 |
| FR-12 | T06, T07, T18, T19 |
| FR-13 | T18, T19 |
| FR-14 | T18, T19 |
| FR-15 | T16, T17, T18, T19 |
| FR-16 | T02, T03, T12, T13, T14, T15 |
| FR-17 | T20, T21 |
| FR-18 | T20, T21 |
| FR-19 | T08, T09, T20, T21 |
| FR-20 | T22, T23, T24, T25 |
| FR-21 | T22, T23, T24, T25 |
| FR-22 | T22, T23, T24, T25 |
| FR-23 | T26 |
| FR-24 | T10, T11 |
| FR-25 | T12, T13, T16, T17 |
| NFR-1 | T27 |
| NFR-2 | T10, T11 |
| NFR-3 | T18, T20 |
| NFR-4 | T10, T11, T28 |
| NFR-5 | T18, T19 |
| NFR-6 | T28 |
