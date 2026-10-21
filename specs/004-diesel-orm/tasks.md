# Tasks — Spec 004

Plan: `specs/004-diesel-orm/plan.md` · Date: 2026-10-21

Format: `- [ ] Tnn [P] (FR-x, FR-y) Description with path — Done when: verifiable criterion`
`[P]` = can run in parallel with other `[P]` tasks of the same phase (no shared files).
Each task < 30 minutes. Tests before implementation.

Switch group (plan section 8): T03–T12 change the shared connection type, so the crate does not compile between them. The full gate (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`) runs at T12, and T03–T12 are committed as one commit `T03–T12: …` listing every task. T03–T11 are marked `[x]` only when the T12 gate passes (constitution principle 3).

## Phase 1 — Setup
- [x] T01 (FR: —) Add `diesel` (sqlite, r2d2, returning_clauses_for_sqlite_3_35), `diesel_migrations` (sqlite) and `libsqlite3-sys` (bundled) to `Cargo.toml` next to `rusqlite`; move `src/migrations/0001_init.sql` → `migrations/0001_init/up.sql` and `src/migrations/0002_web_ui.sql` → `migrations/0002_web_ui/up.sql` byte-identical, and point `src/db.rs` `include_str!` at the new paths — Done when: `cargo tree -i libsqlite3-sys` shows one version and the full gate passes
- [x] T02 (FR: —) Write `src/schema.rs` (`table!` for the 9 tables, `joinable!`, `allow_tables_to_appear_in_same_query!`), register it in `src/lib.rs`, and add `tests/schema.rs` comparing each table's declared columns with `PRAGMA table_info` on a database created by `db::open` — Done when: `tests/schema.rs` green and the full gate passes

## Phase 2 — Foundation: switch to Diesel (blocks the stories; switch group)
- [x] T03 (FR-7, FR-12, FR-13) Rewrite `tests/db.rs` on Diesel: table list without `schema_migrations`, `open_records_diesel_history_only` (versions `0001`, `0002`, no `schema_migrations` table), WAL and foreign keys, `memory_store_has_connection_settings` (foreign keys 1, busy timeout 5000) — Done when: tests written (red: the crate does not build yet)
- [x] T04 (FR-7) Add `count` and `exec` helpers to `tests/common/mod.rs` and port the helper code in `tests/*.rs` that calls `with_conn`/`with_tx` with `rusqlite` (`mcp_stdio`, `store_transfer`, `store_tasks`, `store_projects`, `store_specs`, `store_audit`, `ui_session`, `ui_readonly`, `store_sessions`, `store_users`, `store_documents`), changing no assertion — Done when: `git diff` of those files touches only helper code and `grep -rn rusqlite tests` is empty
- [x] T05 (FR-6, FR-10, FR-11, FR-12, FR-13) In `src/model.rs`, change `StoreError::Database` to `diesel::result::Error` and add `Pool` and `Migration { step, detail }`. In `src/db.rs`, add `DbPool`, `ConnectionSettings` customizer, `set_wal`, `open_pool`, `open_memory_pool`, `MIGRATIONS = embed_migrations!()` and `migrate` (immediate transaction + `run_pending_migrations`, errors mapped to `Migration`) — Done when: `src/db.rs` and `src/model.rs` contain no `rusqlite`
- [x] T06 (FR-8, FR-9) Port `src/store/mod.rs`: `Store { pool }`, `open`, `in_memory`, `with_conn` on `&mut SqliteConnection`, `with_tx` via `immediate_transaction`, `audit`, `project_id_by_slug`, `list_audit`, `list_audit_page` in the DSL — Done when: `src/store/mod.rs` contains no `rusqlite`
- [x] T07 (FR-7) Port `src/store/users.rs` to the DSL — Done when: the file contains no `rusqlite`
- [x] T08 (FR-7) Port `src/store/projects.rs` to the DSL — Done when: the file contains no `rusqlite`
- [x] T09 (FR-7) Port `src/store/specs.rs` to the DSL (`.is()` for nullable matches) — Done when: the file contains no `rusqlite`
- [x] T10 (FR-7, FR-9) Port `src/store/documents.rs` to the DSL (`.returning(id)` for new rows) — Done when: the file contains no `rusqlite`
- [x] T11 (FR-7) Port `src/store/tasks.rs` and `src/store/sessions.rs` to the DSL — Done when: both files contain no `rusqlite`
- [x] T12 (FR-6, FR-7) Port `src/store/transfer.rs`; map `StoreError::Pool` and `StoreError::Migration` like `Database` in `src/mcp.rs` and `src/ui/mod.rs` — Done when: `grep -rn rusqlite src` is empty, T03 tests green and the full gate passes (closes the switch group)

## Phase 3 — US-1 (P1) Upgrade without losing anything
- [x] T13 (FR-1, FR-2, FR-3, FR-4, FR-5, FR-6) Write `tests/db_upgrade.rs` with the previous-release fixture (old runner DDL, fixed `applied_at`, rows in every table): `previous_release_opens_unchanged`, `carry_over_keeps_apply_times`, `release_with_only_first_migration_gets_second`, `previous_history_table_is_kept`, `second_open_changes_no_history`, `failing_migration_leaves_database_untouched`, `empty_previous_history_is_refused`, `unknown_previous_version_is_ignored`; add `tests/cli.rs::open_failure_names_migration_step` — Done when: tests written and red
- [x] T14 (FR-1, FR-2, FR-3, FR-4, FR-5, FR-6) Implement `carry_over_history` and the empty-history refusal in `src/db.rs` `migrate`, with one info log line per carried-over migration — Done when: T13 tests green

## Phase 4 — US-2 (P1) Unchanged behaviour
- [x] T15 (FR-8, FR-9, FR-10, FR-11) Write `tests/db_pool.rs` on a file database: `file_pool_puts_exactly_one_wins` (20 rounds), `failed_write_leaves_no_rows`, `every_pooled_connection_enforces_foreign_keys` and `every_pooled_connection_waits_and_uses_wal` with 8 connections held at once, and `exhausted_pool_fails_after_timeout` (all connections held, a store call fails with `StoreError::Pool` after 4.5–10 s) — Done when: tests green (any red is a defect fixed in `src/db.rs` or `src/store/mod.rs` in this task, with its test)

## Phase 5 — US-3 (P2) New databases
- [x] T16 (FR-12, FR-13) Check that `tests/db.rs` covers a new file and a new in-memory database end to end through `Store::open` and `Store::in_memory`; add any missing assertion — Done when: `cargo test --test db` green

## Final phase — Polish and cross-cutting
- [ ] T17 (NFR-4) Write `tests/perf_open.rs` (ignored): a database with the full schema, history and 100 specs opens in under 1 s — Done when: `cargo test --release --test perf_open -- --ignored` passes
- [ ] T18 (NFR-1, NFR-2) Run `tests/perf.rs`, `tests/perf_ui.rs` and `tests/perf_import.rs` ignored tests in release — Done when: all three pass
- [ ] T19 (NFR-3) Add `tests/cli.rs::binary_needs_no_migration_files`: run the built binary from an empty temporary directory against a new database — Done when: test green
- [ ] T20 (FR: —) Remove `rusqlite` from `Cargo.toml`, add the upgrade note to `README.md` — Done when: `cargo tree -i rusqlite` fails with "did not match" and the full gate passes

## Dependencies
- T01 → T02 → switch group T03–T12 (in order; T03 and T04 can be written in any order) → stories.
- Phase 2 blocks every story. T13 → T14. T15 depends only on T12. T16 depends on T12.
- T17, T18 and T19 depend on T14. T20 is last.
- US-3 behaviour is delivered by the foundation (T05); Phase 5 only confirms its coverage.

## Coverage
| FR | Tasks |
|---|---|
| FR-1 | T13, T14 |
| FR-2 | T13, T14 |
| FR-3 | T13, T14 |
| FR-4 | T13, T14 |
| FR-5 | T13, T14 |
| FR-6 | T05, T12, T13, T14 |
| FR-7 | T03, T04, T07, T08, T09, T10, T11, T12 |
| FR-8 | T06, T15 |
| FR-9 | T06, T10, T15 |
| FR-10 | T05, T15 |
| FR-11 | T05, T15 |
| FR-12 | T03, T05, T16 |
| FR-13 | T03, T05, T16 |
| NFR-1 | T18 |
| NFR-2 | T18 |
| NFR-3 | T19 |
| NFR-4 | T17 |
