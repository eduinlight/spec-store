# Tasks — Spec 001

Plan: `specs/001-store-core/plan.md` · Date: 2026-10-17

Format: `- [ ] Tnn [P] (FR-x, FR-y) Description with path — Done when: verifiable criterion`
`[P]` = can run in parallel with other `[P]` tasks of the same phase (no shared files).
Each task < 30 minutes. Tests before implementation.

## Phase 1 — Setup
- [x] T01 (FR: —) Create `Cargo.toml` (lib + bin `spec-store`, dependencies and features of plan §2, `rmcp = "=3.5.1"`), `src/lib.rs` with the module tree as empty modules, `src/main.rs` printing clap help, `.gitignore` for `target/` and `*.db*` — Done when: `cargo build`, `cargo test` (0 tests) and `cargo clippy --all-targets -- -D warnings` pass

## Phase 2 — Foundation (blocks the stories)
- [x] T02 [P] (FR-16, FR-21, FR-23) Write unit tests in `src/model.rs` for slug/handle validation, commit validation, `DocumentKind`/`Role`/`SpecStatus` parsing and the status transition table — Done when: tests written and red
- [x] T03 (FR-16, FR-21, FR-23) Implement `src/model.rs`: enums, `Caller`, `StoreError` (thiserror) and validators — Done when: T02 tests green
- [x] T04 [P] (FR: —) Write `tests/db.rs`: `db::open` on a temp path creates every table of plan §5, enables WAL and foreign keys, and is idempotent on a second open — Done when: tests written and red
- [x] T05 (FR: —) Implement `src/migrations/0001_init.sql` and `src/db.rs` (open, pragmas, migration runner with `schema_migrations`) — Done when: T04 tests green
- [x] T06 (FR-32) Write `tests/store_audit.rs`: `Store::new` wraps a connection, `with_tx` commits or rolls back, `audit` inserts actor/project/action/target/time and `list_audit` filters by project and limit — Done when: tests written and red
- [x] T07 (FR-32) Implement `src/store/mod.rs` (Store, transaction helper, audit insert and list) — Done when: T06 tests green

## Phase 3 — US-1 (P1) Administer users, tokens, projects and members
- [x] T08 [P] (FR-1, FR-2, FR-3, FR-7) Write `tests/store_users.rs`: create user and duplicate handle, first user is admin, token issued once with `sst_` prefix and listed without secret, revoke blocks that token only, `authenticate` resolves user and updates `last_used_at` — Done when: tests written and red
- [x] T09 (FR-1, FR-2, FR-3, FR-7) Implement `src/auth.rs` (generate_token, hash_token) and `src/store/users.rs` (create_user, issue_token, revoke_token, list_tokens, authenticate) — Done when: T08 tests green
- [x] T10 [P] (FR-4, FR-5, FR-8, FR-9) Write `tests/store_projects.rs`: create project and duplicate slug, set_member replaces role, remove_member ends access, list_projects by membership and admin sees all, `require_role` returns Forbidden for non-members without existence leak — Done when: tests written and red
- [x] T11 (FR-4, FR-5, FR-8, FR-9) Implement `src/store/projects.rs` (create_project, set_member, remove_member, list_projects, role_of, require_role) — Done when: T10 tests green
- [ ] T12 (FR-6, FR-32) Write `tests/cli.rs` running `CARGO_BIN_EXE_spec-store` with `--db` on a temp file: `user add` first user without `--as` becomes admin, second `user add` without `--as` fails, `--as` with a non-admin fails, `token issue` prints a token once, `project add`, `member set`, `audit list` shows the commands — Done when: tests written and red
- [ ] T13 (FR-6, FR-32) Implement `src/cli.rs` admin subcommands (`user add`, `token issue|revoke|list`, `project add`, `member set|remove`, `audit list`) and `src/main.rs` dispatch with exit codes — Done when: T12 tests green

## Phase 4 — US-2 (P1) Read the documents of my projects
- [ ] T14 [P] (FR-10, FR-11, FR-14) Write `tests/store_specs.rs` (list_specs with kinds ordered by number, active_spec skips archived and returns None) and unit tests in `src/templates.rs` (five kinds returned, `research` and unknown rejected with the list of valid kinds) — Done when: tests written and red
- [ ] T15 (FR-14) Implement `src/templates.rs` with `include_str!` of `.sdd/templates/*.md` — Done when: templates tests of T14 green
- [ ] T16 (FR-10, FR-11) Implement `list_specs` and `active_spec` in `src/store/specs.rs` (with a test-only insert helper until T23) — Done when: T14 tests green
- [ ] T17 [P] (FR-12, FR-13, FR-17) Write `tests/store_documents.rs` read part: get_document returns body and revision metadata, not-found distinguishes missing spec from missing kind, get_constitution body and version, not-found when absent; basic put creating document and revision — Done when: tests written and red
- [ ] T18 (FR-12, FR-13, FR-17) Implement `get_document`, `get_constitution` and the basic `put_document`/`put_constitution` path in `src/store/documents.rs` (locking and roles come in T25) — Done when: T17 tests green
- [ ] T19 (FR-7, FR-8, FR-9, FR-10, FR-11, FR-12, FR-13, FR-14) Write `tests/http_auth.rs` (no/unknown/revoked token → 401 with `WWW-Authenticate`, valid token reaches `/mcp` initialize, `/healthz` open) and `tests/mcp_stdio.rs` read part (rmcp client over child process in `stdio --as`: `list_projects`, `list_specs`, `get_active_spec`, `get_document`, `get_constitution`, `get_template`, forbidden project) — Done when: tests written and red
- [ ] T20 (FR-7, FR-8) Implement `src/mcp.rs` skeleton (`SpecStoreMcp` with `#[tool_router]`, caller extraction from `Parts`, `StoreError → ErrorData` mapping, `list_projects`), `src/auth.rs` middleware (`require_bearer`), `src/server.rs` (`build_router`, `serve`, `run_stdio`) and the `serve`/`stdio` subcommands in `src/cli.rs` — Done when: `http_auth` tests and the `list_projects` cases of `mcp_stdio` green
- [ ] T21 (FR-9, FR-10, FR-11, FR-12, FR-13, FR-14) Implement the six read tools (`list_specs`, `get_active_spec`, `get_document`, `get_constitution`, `get_template`, forbidden project handling) in `src/mcp.rs` — Done when: T19 tests green

## Phase 5 — US-3 (P1) Create specs and write documents
- [ ] T22 [P] (FR-15, FR-16, FR-21) Extend `tests/store_specs.rs`: create_spec assigns next number counting archived, branch `NNN-slug`, rejects bad and duplicate slug, status transitions accepted/rejected per plan §5, archive by owner only, reader forbidden — Done when: tests written and red
- [ ] T23 (FR-15, FR-16, FR-21) Implement `create_spec` and `set_status` in `src/store/specs.rs` and remove the test-only insert helper — Done when: T22 tests green
- [ ] T24 [P] (FR-17, FR-18, FR-19, FR-20, FR-24) Extend `tests/store_documents.rs`: expected_revision missing or stale → Conflict with current id, reader and archived → Forbidden, constitution owner only and same version → Conflict, body over 1 MiB rejected and bytes kept exactly, two threads with a barrier → exactly one success — Done when: tests written and red
- [ ] T25 (FR-17, FR-18, FR-19, FR-20, FR-24) Implement locking, role checks, archived check, size limit and version check in `src/store/documents.rs` — Done when: T24 tests green
- [ ] T26 [P] (FR-22, FR-23, FR-25, FR-26) Write unit tests in `src/markdown/tasks.rs`: parse every field of the template format, phase from `##` headings, `[P]` flag, FR list, `(FR: —)`, done box, `· commit:` suffix, CRLF bodies, malformed lines with numbers, `mark_done` rewrites exactly one line and rejects unknown, done or duplicate ids — Done when: tests written and red
- [ ] T27 (FR-22, FR-23, FR-25, FR-26) Implement `src/markdown/tasks.rs` (`parse_tasks`, `mark_done`) — Done when: T26 tests green
- [ ] T28 (FR-22, FR-23) Write `tests/store_tasks.rs` mark part: mark_task_done stores a new revision with message naming the task, read back shows `[x]` and commit, rejects malformed commit, unknown task, already done, archived spec, reader — Done when: tests written and red
- [ ] T29 (FR-22, FR-23) Implement `mark_task_done` in `src/store/tasks.rs` — Done when: T28 tests green
- [ ] T30 (FR-15, FR-17, FR-18, FR-20, FR-21, FR-22) Extend `tests/mcp_stdio.rs`: `create_spec`, `put_document` (create, update with expected_revision, conflict error data), `put_constitution`, `set_spec_status`, `mark_task_done` — Done when: tests written and red
- [ ] T31 (FR-15, FR-17, FR-18, FR-20, FR-21, FR-22) Implement the five write tools in `src/mcp.rs` — Done when: T30 tests green

## Phase 6 — US-4 (P2) See progress without parsing
- [ ] T32 [P] (FR-27) Write unit tests in `src/markdown/markers.rs`: both marker kinds, nearest heading, several per line, none — Done when: tests written and red
- [ ] T33 (FR-27) Implement `src/markdown/markers.rs` (`scan_markers`) — Done when: T32 tests green
- [ ] T34 [P] (FR-25, FR-26, FR-27, FR-28, FR-29) Extend `tests/store_tasks.rs`: list_tasks with pending filter and malformed list, list_markers across the spec's documents, project_status (constitution version or absence, specs, done/total, current phase, next task with done-when, marker count, no active spec), status reflects the last write — Done when: tests written and red
- [ ] T35 (FR-25, FR-26, FR-27, FR-28, FR-29) Implement `list_tasks`, `list_markers`, `project_status` in `src/store/tasks.rs` — Done when: T34 tests green
- [ ] T36 (FR-25, FR-27, FR-28) Extend `tests/mcp_stdio.rs`: `list_tasks`, `list_markers`, `project_status` JSON shapes — Done when: tests written and red
- [ ] T37 (FR-25, FR-27, FR-28) Implement the three derived tools in `src/mcp.rs` — Done when: T36 tests green

## Phase 7 — US-5 (P2) Know who changed what
- [ ] T38 (FR-30, FR-31) Extend `tests/store_documents.rs` (list_revisions newest first with author and message, constitution revisions with version, get_revision body and not-found) and `tests/mcp_stdio.rs` (`list_revisions`, `get_revision`) — Done when: tests written and red
- [ ] T39 (FR-30, FR-31) Implement `list_revisions` and `get_revision` in `src/store/documents.rs` — Done when: store tests of T38 green
- [ ] T40 (FR-30, FR-31) Implement the two history tools in `src/mcp.rs` — Done when: T38 tests green

## Final phase — Polish and cross-cutting
- [ ] T41 (NFR-1) Write `tests/perf.rs` (ignored by default): 10 threads, 200 reads of a 200 KB spec through the store, assert p95 < 100 ms — Done when: `cargo test --release --test perf -- --ignored` passes
- [ ] T42 (NFR-3) Extend `tests/http_auth.rs`: with a capturing tracing subscriber, the issued token never appears in logs, audit rows, error bodies or `token list` output; fix any leak found — Done when: test green
- [ ] T43 (NFR-4, NFR-5, FR: —) Write `README.md` (run `serve`, bootstrap admin and token, `claude mcp add --transport http ... --header`, `.mcp.json` with `${SPEC_STORE_TOKEN}`, stdio mode) and run the full lint — Done when: README has those sections and `cargo clippy --all-targets -- -D warnings && cargo fmt --check` passes

## Dependencies
- Phase 1 blocks everything. Phase 2 blocks all stories: T03 needs T01; T05 needs T01; T07 needs T03 and T05.
- US-1: T09 needs T07; T11 needs T09 (memberships reference users); T13 needs T09 and T11.
- US-2: T15 needs T01; T16 needs T07; T18 needs T16; T20 needs T09 and T11; T21 needs T15, T16, T18 and T20.
- US-3: T23 needs T16; T25 needs T18 and T23; T27 needs T03; T29 needs T25 and T27; T31 needs T23, T25, T29 and T21.
- US-4: T33 needs T03; T35 needs T27, T29 and T33; T37 needs T35 and T31.
- US-5: T39 needs T25; T40 needs T39 and T37.
- Polish: T41 needs T35; T42 needs T20 and T13; T43 needs T40.
- Stories US-3, US-4 and US-5 each build on US-2's read path; US-4 and US-5 are independent of each other.

## Coverage
| FR | Tasks |
|---|---|
| FR-1 | T08, T09 |
| FR-2 | T08, T09 |
| FR-3 | T08, T09 |
| FR-4 | T10, T11 |
| FR-5 | T10, T11 |
| FR-6 | T12, T13 |
| FR-7 | T08, T09, T19, T20 |
| FR-8 | T10, T11, T19, T20 |
| FR-9 | T10, T11, T19, T21 |
| FR-10 | T14, T16, T19, T21 |
| FR-11 | T14, T16, T19, T21 |
| FR-12 | T17, T18, T19, T21 |
| FR-13 | T17, T18, T19, T21 |
| FR-14 | T14, T15, T19, T21 |
| FR-15 | T22, T23, T30, T31 |
| FR-16 | T02, T03, T22, T23 |
| FR-17 | T17, T18, T24, T25, T30, T31 |
| FR-18 | T24, T25, T30, T31 |
| FR-19 | T24, T25 |
| FR-20 | T24, T25, T30, T31 |
| FR-21 | T02, T03, T22, T23, T30, T31 |
| FR-22 | T26, T27, T28, T29, T30, T31 |
| FR-23 | T02, T03, T26, T27, T28, T29 |
| FR-24 | T24, T25 |
| FR-25 | T26, T27, T34, T35, T36, T37 |
| FR-26 | T26, T27, T34, T35 |
| FR-27 | T32, T33, T34, T35, T36, T37 |
| FR-28 | T34, T35, T36, T37 |
| FR-29 | T34, T35 |
| FR-30 | T38, T39, T40 |
| FR-31 | T38, T39, T40 |
| FR-32 | T06, T07, T12, T13 |
| NFR-1 | T41 |
| NFR-2 | T24, T25 |
| NFR-3 | T42 |
| NFR-4 | T19, T20, T43 |
| NFR-5 | T43 |
