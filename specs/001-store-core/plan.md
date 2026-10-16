# Technical plan — Spec 001

Spec: `specs/001-store-core/spec.md` · Constitution: v1.0 · Date: 2026-10-17

## 1. Summary

One Rust crate, `spec-store`, builds a library plus a binary. The library holds the SQLite store (users, tokens, projects, memberships, specs, documents, revisions, audit), the markdown parsers for tasks and markers, and the MCP handler with 16 tools built on `rmcp`. The binary exposes `serve` (axum + Streamable HTTP with a bearer-token middleware), `stdio` (same handler over stdin/stdout for local use and tests) and the administration subcommands of US-1, which open the database file directly.

## 2. Technical context

- Language/version: Rust stable 1.97, edition 2024.
- Framework/platform: `rmcp` 3.5 (server, macros, schemars, transport-io, transport-streamable-http-server), `axum` 0.8, `tokio` 1 (rt-multi-thread, macros, signal).
- Dependencies: `rusqlite` 0.40 (bundled), `clap` 4 (derive), `serde` + `serde_json`, `schemars` 1 (tool parameter schemas, required by rmcp), `sha2` 0.11, `rand` 0.10, `base64` (token encoding), `thiserror` 2, `anyhow` 1 (binary only), `tracing` + `tracing-subscriber` (env-filter), `tokio-util` (CancellationToken used by rmcp config). Dev: `tower` (util) and `http-body-util` for in-process HTTP tests, `rmcp` client features (`client`, `transport-child-process`) for stdio end-to-end tests.
- Storage: one SQLite file, WAL, `foreign_keys = ON`, numbered migrations embedded in the binary.
- Tests: `cargo test` (unit tests beside modules, integration tests in `tests/`), `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- Constraints: single binary; markdown is the source of truth; no hard deletes; all logs to stderr.
- Unknowns: all resolved in `research.md` (R1 to R9).

## 3. Constitution check (gate)

| Principle | Complies | Note |
|---|---|---|
| 1. The spec rules | yes | Plan derived from spec 001, status clarified, no markers |
| 2. Stack and limits | yes | Base crates as named; extra crates justified in section 7 |
| 3. Tests as a gate | yes | Section 9; tasks will put tests before implementation |
| 4. Simplicity | yes | No ORM, no connection pool, no CommonMark parser, no web framework beyond axum |
| 5. Markdown is the source of truth | yes | Index derived at read time from the current revision; commit lives in the task line |
| 6. Nothing is lost | yes | Append-only `revisions`, `audit_log`, migrations table, optimistic locking |
| 7. Access is scoped | yes | Middleware authenticates; every store write checks role |
| 8. Reference sources | yes | `research.md` cites rmcp, axum example, rusqlite, MCP spec, Claude Code docs |
| 9. Language | yes | |
| 10. Human approval | yes | Server never deletes data; commits need the user's OK |

## 4. Module / file structure

```
Cargo.toml                      crate manifest, lib + bin "spec-store"
src/lib.rs                      module tree and re-exports
src/main.rs                     clap entry: parse, init tracing to stderr, dispatch to cli
src/cli.rs                      clap definitions and admin command handlers (US-1), serve/stdio dispatch
src/db.rs                       open(path): pragmas (WAL, busy_timeout, foreign_keys), run migrations
src/migrations/0001_init.sql    full schema of section 5
src/model.rs                    Role, SpecStatus, DocumentKind, Caller, ids, StoreError
src/store/mod.rs                Store { conn: Arc<Mutex<Connection>> }, audit helper, transaction helper
src/store/users.rs              create_user, issue_token, revoke_token, list_tokens, authenticate(token) (FR-1..3, FR-6, FR-7)
src/store/projects.rs           create_project, set_member, remove_member, list_projects, role_of (FR-4, FR-5, FR-8, FR-9)
src/store/specs.rs              create_spec, list_specs, active_spec, set_status (FR-10, FR-11, FR-15, FR-16, FR-21)
src/store/documents.rs          get_document, put_document, get/put_constitution, list_revisions, get_revision (FR-12, FR-13, FR-17..20, FR-30, FR-31)
src/store/tasks.rs              mark_task_done, list_tasks, list_markers, project_status (FR-22..29)
src/markdown/tasks.rs           parse_tasks(body) -> (Vec<Task>, Vec<Malformed>), mark_done(body, id, commit) -> Result<String>
src/markdown/markers.rs         scan_markers(body, kind) -> Vec<Marker> with nearest heading
src/templates.rs                include_str! of .sdd/templates/*.md, get(kind) (FR-14)
src/auth.rs                     token generation and hashing (R7), axum middleware, Caller extraction from Parts
src/mcp.rs                      SpecStoreMcp handler: #[tool_router] with the 16 tools, parameter structs, StoreError -> ErrorData
src/server.rs                   build_router(store, config), serve(bind, allowed_hosts), run_stdio(store, caller)
tests/store_*.rs                integration tests on a temp database per store module
tests/http_auth.rs              router tests with tower::ServiceExt::oneshot (FR-7, FR-9)
tests/mcp_stdio.rs              end-to-end through rmcp client over child process in stdio mode
tests/cli.rs                    binary smoke tests via CARGO_BIN_EXE_spec-store
```

The templates seed is read at compile time from `.sdd/templates/`, so the repo files remain the single source and no templates table exists.

## 5. Data model

All timestamps are RFC 3339 UTC text produced by SQLite. All ids are `INTEGER PRIMARY KEY`.

- `schema_migrations(version INTEGER PK, applied_at TEXT)`
- `users(id, handle TEXT UNIQUE, is_admin INTEGER, created_at)` — handle validated like a slug (FR-1, Assumptions)
- `api_tokens(id, user_id FK, token_hash TEXT UNIQUE, label TEXT, created_at, last_used_at TEXT NULL, revoked_at TEXT NULL)` (FR-2, FR-3)
- `projects(id, slug TEXT UNIQUE, name TEXT, created_at)` (FR-4)
- `memberships(project_id FK, user_id FK, role TEXT CHECK(role IN ('owner','editor','reader')), PRIMARY KEY(project_id, user_id))` (FR-5)
- `specs(id, project_id FK, number INTEGER, slug TEXT, status TEXT CHECK(status IN ('draft','clarified','approved','archived')), branch TEXT, created_at, UNIQUE(project_id, number), UNIQUE(project_id, slug))` (FR-15, FR-16, FR-21)
- `documents(id, project_id FK, spec_id FK NULL, kind TEXT CHECK(kind IN ('constitution','spec','plan','tasks','research','validation')), current_revision_id INTEGER NULL, UNIQUE(project_id, spec_id, kind))` — constitution rows have `spec_id NULL` (FR-12, FR-13, FR-17)
- `revisions(id, document_id FK, body TEXT, version TEXT NULL, author_id FK, message TEXT, created_at)` — `version` used by constitution revisions only (FR-17, FR-20, FR-30, FR-31)
- `audit_log(id, actor_id FK, project_id FK NULL, action TEXT, target TEXT, created_at)` (FR-6, FR-32)

Derived, never stored (constitution 5): `Task { id, phase, parallel, frs, description, done_when, done, commit }`, `Malformed { line, text }`, `Marker { kind, document_kind, heading, text }`, `ProjectStatus`.

Validation rules: slug/handle `^[a-z0-9]+(-[a-z0-9]+){0,3}$`; commit `^[0-9a-f]{7,40}$`; body ≤ 1 048 576 bytes (NFR-2); version must differ from current (FR-20).

Status transitions (FR-21): `draft→clarified`, `clarified→approved`, `clarified→draft`, `approved→draft` for editor or owner; `*→archived` for owner only; nothing leaves `archived`.

## 6. Contracts / interfaces

### 6.1 Error model

`StoreError` variants and their MCP mapping (`rmcp::ErrorData`):

| Variant | Meaning | MCP code | `data.kind` |
|---|---|---|---|
| `Unauthenticated` | no, unknown or revoked token | HTTP 401 before MCP; in stdio mode `-32001` | `unauthenticated` |
| `Forbidden` | no membership, wrong role, archived spec | `-32003` | `forbidden` |
| `NotFound(what)` | project, spec, document kind, revision, task | `-32002` resource_not_found | `not_found`, `what` |
| `Conflict { current_revision }` | revision mismatch, same version | `-32602` invalid_params | `conflict`, `current_revision` |
| `Validation(msg)` | slug, commit, status transition, kind, size | `-32602` invalid_params | `validation` |
| `Malformed(msg)` | tasks document cannot be edited safely | `-32602` invalid_params | `malformed` |

The message always names the cause in English (FR-12, FR-16, FR-21, FR-23).

### 6.2 MCP tools

Every tool takes `project: String` (slug) unless noted; the caller comes from the request context.

| Tool | Input | Output | FR |
|---|---|---|---|
| `list_projects` | — | `[{slug, name, role}]` (role `admin` when read through the admin flag) | 8 |
| `list_specs` | project | `[{number, slug, status, branch, kinds: [..]}]` | 10 |
| `get_active_spec` | project | `{number, slug, status, branch} \| null` with `message` when none | 11 |
| `get_document` | project, number, kind | `{body, revision: {id, author, created_at, message}}` | 12 |
| `get_constitution` | project | `{body, version, revision}` | 13 |
| `get_template` | kind | `{kind, body}` | 14 |
| `create_spec` | project, slug | `{number, slug, branch, status}` | 15, 16 |
| `put_document` | project, number, kind, body, message, expected_revision? | `{revision_id}` | 17, 18, 19, 24 |
| `put_constitution` | project, body, version, message, expected_revision? | `{revision_id}` | 20 |
| `set_spec_status` | project, number, status | `{number, status}` | 21 |
| `mark_task_done` | project, number, task_id, commit | `{revision_id, line}` | 22, 23 |
| `list_tasks` | project, number, pending_only? | `{tasks: [..], malformed: [{line, text}]}` | 25, 26 |
| `list_markers` | project, number | `[{kind, document_kind, heading, text}]` | 27 |
| `project_status` | project | `{constitution_version?, specs: [..], active: {number, tasks_done, tasks_total, current_phase, next_task, markers}?}` | 28, 29 |
| `list_revisions` | project, number?, kind | `[{id, author, created_at, message, version?}]` (`number` absent → constitution) | 30 |
| `get_revision` | project, revision_id | `{body, author, created_at, message}` | 31 |

`number` is the integer spec number; formatting as `NNN` happens in `branch` and in output `slug`-based names.

### 6.3 HTTP

- `POST|GET|DELETE /mcp`: rmcp Streamable HTTP; route layer `auth::require_bearer` → `401` + `WWW-Authenticate: Bearer` on failure (FR-7, NFR-3); on success inserts `Caller { user_id, handle, is_admin }` into request extensions and updates `last_used_at`.
- `GET /healthz`: `200 ok`, unauthenticated.
- Config: `--bind` (default `127.0.0.1:7391`), `--db` (default `./spec-store.db`), `--allowed-host` repeatable (passed to `with_allowed_hosts`), body limit 2 MiB.

### 6.4 CLI (FR-6)

```
spec-store serve [--bind A] [--db P] [--allowed-host H]...
spec-store stdio --db P --as HANDLE
spec-store user add HANDLE [--admin] [--as ADMIN]        first user on empty store: --as not allowed, becomes admin
spec-store token issue HANDLE --label L --as ADMIN        prints token once
spec-store token revoke TOKEN_ID --as ADMIN
spec-store token list HANDLE --as ADMIN                   id, label, created_at, last_used_at, revoked_at
spec-store project add SLUG NAME --as ADMIN
spec-store member set PROJECT HANDLE ROLE --as ACTOR      ACTOR admin or owner of PROJECT
spec-store member remove PROJECT HANDLE --as ACTOR
spec-store audit list [--project SLUG] [--limit N] --as ADMIN
```

Exit code 0 on success, 1 on any error with the message on stderr. Stdio mode acts for `--as HANDLE` with that user's memberships; no token involved (NFR-4).

## 7. Technical decisions

| Decision | Discarded alternative | Why |
|---|---|---|
| Single `Arc<Mutex<Connection>>`, store calls run inline in async handlers | r2d2 pool or `spawn_blocking` per call | Every operation is a few sub-millisecond statements; serialization gives FR-24 and the first-write race for free; the pool adds a dependency and no benefit at NFR-1 scale |
| Bearer middleware inserts `Caller` into request extensions; tools read it via `Parts` (R2, R3) | Validate the token inside every tool | One check per request, no token reaches the handler, 401 at HTTP level as the SDK example does |
| Stdio mode takes `--as HANDLE` | Stdio with a token argument | Stdio runs on the server host where the data file is already trusted (FR-6 logic); keeps tokens out of process lists |
| Templates embedded with `include_str!` | `templates` table seeded at first start | Repo files stay the single source, no sync problem, no write path for something read-only in 001 |
| Line-oriented hand parser for tasks and markers (R8) | `pulldown-cmark` + `regex` | The format is fixed and single-line; two fewer dependencies (constitution 2) |
| SHA-256 of a 256-bit random token (R7) | `argon2` | Slow hashing protects low-entropy secrets only |
| Integer revision ids from one `revisions` table | ULIDs | Simpler, sortable, unique across documents, no extra crate |
| Timestamps from SQLite `strftime` (R9) | `time` or `chrono` | No date crate; one source of time |
| Index computed at read time from the current revision | Index tables rebuilt on write | Zero drift by construction (FR-29, constitution 5); documents are small (NFR-2) so parsing per read is within NFR-1 |
| `expected_revision` mandatory on existing documents | Last write wins | FR-18; agents working on the same spec must not overwrite each other |
| Admin CLI opens the database directly, WAL + busy_timeout | Admin over HTTP | FR-6; no token bootstrap problem; WAL lets the CLI and the server share the file |
| `base64` crate for token text | hex | Shorter tokens; `base64` is tiny and ubiquitous |
| No MCP resources in 001 | Expose documents as resources too | Tools cover every FR; resources double the surface for no FR |

## 8. Deviations from the constitution

| Deviation | Why it is necessary | Why the simple alternative does not work |
|---|---|---|
| none | | |

## 9. Test strategy

- Unit tests (in-module): slug/commit/version validation, token generation and hashing, `markdown::tasks` (parse, malformed lines, mark_done rewrite, CRLF, duplicate ids), `markdown::markers` (nearest heading, both marker kinds), status transition table, error mapping.
- Store integration tests (`tests/store_*.rs`): each test opens a fresh temp database through `db::open`, exercises the store API and asserts rows and errors. Covers FR-1 to FR-6, FR-8 to FR-13, FR-15 to FR-24 (FR-24 with two threads and a barrier), FR-25 to FR-32.
- HTTP tests (`tests/http_auth.rs`): `build_router` + `tower::ServiceExt::oneshot`: no header → 401 with `WWW-Authenticate`; revoked token → 401; valid token → request reaches `/mcp` (initialize succeeds); `/healthz` open. Covers FR-7, NFR-3 (token absent from body and logs captured by a test subscriber).
- MCP end-to-end (`tests/mcp_stdio.rs`): spawn the binary in `stdio --as` mode with a seeded database via the rmcp client over child process; call `list_projects`, `create_spec`, `put_document`, `mark_task_done`, `project_status`, assert JSON results. Covers the tool contracts and NFR-4.
- CLI tests (`tests/cli.rs`): run the binary with `CARGO_BIN_EXE_spec-store` for `user add` (first admin, duplicate), `token issue`, `project add`, `member set`, `audit list`; assert exit codes and output. Covers FR-6 and the US-1 flow.
- Performance (NFR-1): one ignored-by-default test measuring 200 reads of a 200 KB spec with 10 threads, asserting p95 < 100 ms; run in `/sdd-validate`.

## 10. FR coverage

| FR | Module/file | Planned test |
|---|---|---|
| FR-1 | store/users.rs | store_users::creates_user_and_rejects_duplicate_handle |
| FR-2 | store/users.rs, auth.rs | store_users::issues_token_once_and_lists_without_secret |
| FR-3 | store/users.rs | store_users::revoked_token_fails_other_tokens_work |
| FR-4 | store/projects.rs | store_projects::creates_project_and_rejects_duplicate_slug |
| FR-5 | store/projects.rs | store_projects::set_member_replaces_role_and_remove_ends_access |
| FR-6 | cli.rs | cli::first_user_is_admin_then_as_required_and_audited |
| FR-7 | auth.rs, server.rs | http_auth::missing_unknown_revoked_token_get_401 |
| FR-8 | store/projects.rs | store_projects::list_projects_by_membership_and_admin_sees_all |
| FR-9 | store/projects.rs | store_projects::non_member_gets_forbidden_without_existence_leak |
| FR-10 | store/specs.rs | store_specs::list_specs_with_kinds_ordered |
| FR-11 | store/specs.rs | store_specs::active_spec_skips_archived_and_reports_none |
| FR-12 | store/documents.rs | store_documents::get_document_and_distinct_not_found |
| FR-13 | store/documents.rs | store_documents::get_constitution_and_not_found |
| FR-14 | templates.rs | templates::returns_kinds_and_rejects_unknown |
| FR-15 | store/specs.rs | store_specs::create_spec_assigns_next_number_and_branch |
| FR-16 | store/specs.rs, model.rs | store_specs::rejects_bad_or_duplicate_slug |
| FR-17 | store/documents.rs | store_documents::put_creates_document_and_revision |
| FR-18 | store/documents.rs | store_documents::put_requires_matching_expected_revision |
| FR-19 | store/documents.rs | store_documents::reader_and_archived_cannot_write |
| FR-20 | store/documents.rs | store_documents::constitution_owner_only_and_version_must_change |
| FR-21 | store/specs.rs | store_specs::status_transitions_table |
| FR-22 | store/tasks.rs, markdown/tasks.rs | store_tasks::mark_done_rewrites_line_with_commit |
| FR-23 | store/tasks.rs | store_tasks::mark_done_rejects_unknown_done_or_bad_commit |
| FR-24 | store/documents.rs | store_documents::concurrent_puts_exactly_one_wins |
| FR-25 | markdown/tasks.rs, store/tasks.rs | markdown_tasks::parses_all_fields; store_tasks::list_tasks_pending_filter |
| FR-26 | markdown/tasks.rs | markdown_tasks::reports_malformed_lines_with_numbers |
| FR-27 | markdown/markers.rs, store/tasks.rs | markdown_markers::finds_both_kinds_with_heading; store_tasks::list_markers_across_documents |
| FR-28 | store/tasks.rs | store_tasks::project_status_counts_phase_next_and_markers |
| FR-29 | store/tasks.rs | store_tasks::status_reflects_last_write |
| FR-30 | store/documents.rs | store_documents::list_revisions_newest_first |
| FR-31 | store/documents.rs | store_documents::get_revision_body_and_not_found |
| FR-32 | store/mod.rs, cli.rs | store_audit::every_write_logged; cli::audit_list |
| NFR-1 | tests/perf.rs | perf::p95_read_under_100ms (ignored by default) |
| NFR-2 | store/documents.rs | store_documents::rejects_over_1mb_and_keeps_bytes |
| NFR-3 | auth.rs | http_auth::token_never_in_logs_or_responses |
| NFR-4 | server.rs | mcp_stdio::same_tools_over_stdio |
| NFR-5 | all | reviewed in validation |

## 11. Risks

- `rmcp` 3.x API drift between minor versions: pin `rmcp = "=3.5.1"` in Cargo.toml and re-check the two example files cited in R1/R2 if compilation fails.
- Reading `Parts` from `ctx.extensions` depends on the SDK injecting them; covered by the first HTTP test, with the fallback of a tower layer that copies `Caller` into rmcp's extensions if needed.
- Holding a `std::sync::Mutex` across SQLite calls inside async handlers blocks a tokio worker for microseconds; acceptable at NFR-1 scale, revisit with `spawn_blocking` if the perf test fails.
- Hand-written task parser versus real tasks documents: fixtures in tests use this project's own `tasks.md` once it exists, plus the template.
- Stdio mode bypasses tokens by design; the binary must only be reachable by people with host access, which the README of a later spec states.
