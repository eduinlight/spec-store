# Technical plan — Spec 004

Spec: `specs/004-diesel-orm/spec.md` · Constitution: v1.1 · Date: 2026-10-21

## 1. Summary

`Store` replaces its single mutex-guarded `rusqlite::Connection` with an `r2d2` pool of Diesel `SqliteConnection`s. A connection customizer gives every connection the 5-second busy timeout and foreign keys, and write transactions use `BEGIN IMMEDIATE`. Every store query moves to the Diesel DSL over a hand-written `src/schema.rs`, keeping the same SQL semantics, transaction boundaries and public `Store` API. The two SQL migrations become embedded Diesel migrations. At startup, one immediate transaction copies the previous `schema_migrations` rows into `__diesel_schema_migrations` and then runs the pending migrations.

## 2. Technical context

- Language/version: Rust 1.97.1, edition 2024.
- Framework/platform: `diesel` 2.3.14 (sqlite, r2d2, returning_clauses_for_sqlite_3_35), `diesel_migrations` 2.3.2, `libsqlite3-sys` 0.38 bundled (R1). `rmcp`, `axum` and `tokio` are unchanged.
- Dependencies: `diesel`, `diesel_migrations` and `libsqlite3-sys` are added; `rusqlite` is removed at the end. `r2d2` comes in through Diesel's re-export `diesel::r2d2`, so it is not a direct dependency.
- Storage: the same SQLite file and schema. The migration history moves to `__diesel_schema_migrations` (R2, R3).
- Tests: `cargo test` integration tests in `tests/`, plus the ignored performance tests `tests/perf*.rs` for NFR-1, NFR-2 and NFR-4.
- Constraints: no schema change; no client-visible change; single binary with migrations embedded (NFR-3).
- Unknowns: none left. R1–R8 in `research.md` resolve versions, history format, version strings, unknown versions, write-lock behaviour, pool settings, SQLite expressions and error exposure.

## 3. Constitution check (gate)

| Principle | Complies | Note |
|---|---|---|
| 1. The spec rules | yes | Spec 004 is clarified, with no open questions |
| 2. Stack and limits | yes | `diesel` is base since v1.1. `diesel_migrations` is justified in D3, `libsqlite3-sys` in D1, and `r2d2` (through `diesel::r2d2`) in D2 |
| 3. Tests as a gate | deviation | Mid-switch tasks cannot compile, so the gate runs at the end of the switch group (section 8) |
| 4. Simplicity | yes | Diesel DSL used directly; no repository layer; rows are loaded as tuples into the existing structs; the pool is justified in D2 |
| 5. Markdown source of truth | yes | Bodies stay TEXT and are bound as `&str` with no transformation; index data is still derived on read |
| 6. Nothing is lost | yes | The migrations are the same numbered SQL; the carry-over keeps the old table (FR-4); no `down.sql`; revision + audit stay in one transaction (D4) |
| 7. Access is scoped | yes | `access_in` and role checks move unchanged; only their queries are ported |
| 8. Reference sources | yes | `research.md` cites the Diesel, `diesel_migrations`, `r2d2` and SQLite sources |
| 9. Language | yes | English throughout |
| 10. Human approval | yes | No data is deleted; commits and the merge are authorized by the user for this spec |

## 4. Module / file structure

| File | Responsibility |
|---|---|
| `Cargo.toml` | Add `diesel`, `diesel_migrations` and `libsqlite3-sys` (bundled); remove `rusqlite` in the final task |
| `migrations/0001_init/up.sql` (moved from `src/migrations/0001_init.sql`) | Migration `0001`, content byte-identical |
| `migrations/0002_web_ui/up.sql` (moved from `src/migrations/0002_web_ui.sql`) | Migration `0002`, content byte-identical |
| `src/schema.rs` (new) | `diesel::table!` for `users`, `api_tokens`, `projects`, `memberships`, `specs`, `documents`, `revisions`, `audit_log`, `sessions`; `joinable!` and `allow_tables_to_appear_in_same_query!` |
| `src/db.rs` | `DbPool` type; `open_pool(path)`, `open_memory_pool()`; `ConnectionSettings` customizer (busy timeout, foreign keys); `set_wal`; `migrate(conn)` = immediate transaction { `carry_over_history`, `refuse_empty_history`, `run_pending_migrations(MIGRATIONS)` }; `MIGRATIONS = embed_migrations!()` |
| `src/model.rs` | `StoreError::Database(diesel::result::Error)`, new `StoreError::Pool(diesel::r2d2::PoolError)` and `StoreError::Migration { step, detail }` |
| `src/store/mod.rs` | `Store { pool }`; `with_conn(&mut SqliteConnection)`, `with_tx` via `immediate_transaction`; `audit`, `project_id_by_slug`, audit listings ported |
| `src/store/users.rs`, `projects.rs`, `specs.rs`, `documents.rs`, `tasks.rs`, `sessions.rs`, `transfer.rs` | Same functions and signatures, with `&mut SqliteConnection` in place of `&Connection`/`&Transaction`; queries in the Diesel DSL |
| `src/mcp.rs`, `src/ui/mod.rs` | `StoreError::Pool` and `StoreError::Migration` mapped like `Database` (generic internal error, detail only in the log) |
| `src/lib.rs` | `pub mod schema` |
| `tests/common/mod.rs` | `count(store, sql)` and `exec(store, sql)` helpers on `sql_query`/`batch_execute`, replacing the `rusqlite` calls in helper code |
| `tests/db.rs` | Runner-table assertions rewritten against `__diesel_schema_migrations` (FR-7 exemption, FR-12, FR-13) |
| `tests/db_upgrade.rs` (new) | Previous-release fixture and FR-1 to FR-6 |
| `tests/db_pool.rs` (new) | FR-8, FR-10, FR-11 on a file database |
| `tests/schema.rs` (new) | Every `table!` column list equals `PRAGMA table_info` for that table |
| `tests/perf_open.rs` (new, ignored) | NFR-4 |
| Other `tests/*.rs` | Only helper code that reads the database directly changes (`with_conn`/`with_tx` closures) |
| `README.md` | Note about the migration history move on upgrade |

## 5. Data model

The tables, columns, constraints and indexes are unchanged (0001 and 0002). Column type mapping in `src/schema.rs`: `INTEGER` → `BigInt` (with `Nullable` where the column allows NULL), `TEXT` → `Text`. The `users.is_admin` integer is read as `BigInt` and compared with 0, as today. Timestamps stay `Text`, so stored strings round-trip byte for byte.

Migration history:

- Previous: `schema_migrations(version INTEGER PRIMARY KEY, applied_at TEXT)`. It is read only, never written or dropped (FR-4), and not declared in `schema.rs`.
- New: `__diesel_schema_migrations(version VARCHAR(50) PRIMARY KEY, run_on TIMESTAMP)`. Version `n` is `format!("{n:04}")` (R3).

Carry-over (FR-1 to FR-6), inside one `BEGIN IMMEDIATE`:

1. `CREATE TABLE IF NOT EXISTS __diesel_schema_migrations (...)` (same statement as Diesel, R2).
2. If `schema_migrations` exists: `INSERT OR IGNORE INTO __diesel_schema_migrations (version, run_on) SELECT printf('%04d', version), applied_at FROM schema_migrations WHERE version IN (1, 2)`, and log one info line per row inserted (FR-2, FR-5, unknown-version edge case).
3. If `__diesel_schema_migrations` is empty and table `users` exists: fail with `Migration { step: "carry-over", detail: "schema_migrations is empty but the schema exists" }` (edge case).
4. `run_pending_migrations(MIGRATIONS)`. Each migration runs in a savepoint nested in the outer transaction; an error becomes `Migration { step: <version>, detail }` (FR-3, FR-6, FR-12).

Any error rolls back the whole transaction, so schema, data and both histories stay as they were (FR-6).

## 6. Contracts / interfaces

- The public `Store` API is unchanged except the closure parameter of `with_conn`/`with_tx`, which becomes `&mut SqliteConnection`. Those two are used only by tests outside `src/store`.
- `Store::open(path)`: sets WAL (FR-11), migrates, builds the file pool (max 8, connection timeout 5 s). Errors: `Database`, `Pool`, `Migration`.
- `Store::in_memory()`: memory pool (max 1, min idle 1, no idle timeout, no max lifetime), then migrates (FR-13).
- CLI and server: a failed `Store::open` already ends the process with exit code 1 and the error on the error stream (`src/cli.rs:181`, `with_context("opening …")`). The `Migration` display is `migration step {step} failed: {detail}` (FR-6).
- MCP, JSON API and web UI: unchanged. `Pool` and `Migration` map to the same internal error as `Database` (R8).

## 7. Technical decisions

| Decision | Discarded alternative | Why |
|---|---|---|
| D1 Bundle SQLite through `libsqlite3-sys` `bundled` | System SQLite | Same as today (`rusqlite` bundled); hosts need no SQLite library (NFR-3) |
| D2 `r2d2` pool from `diesel::r2d2`, file max 8, timeout 5 s | Keep one `Mutex<SqliteConnection>` | Approved in the spec assumptions; WAL allows concurrent readers; the 5 s timeout matches the busy timeout from the spec's edge case |
| D3 `diesel_migrations` with `embed_migrations!()` from `migrations/` | Keep the hand-written runner | Approved in the spec; standard layout, migrations embedded in the binary (NFR-3) |
| D4 `with_tx` uses `immediate_transaction` | `Connection::transaction` (deferred) | A deferred read-then-write can get `SQLITE_BUSY` without waiting (R5), which would turn the FR-8 race into a database error instead of a conflict |
| D5 Carry-over and pending migrations in one immediate transaction | Carry-over and each migration in separate transactions | Two processes starting at once cannot both compute the same pending migration; a failure leaves everything as it was (FR-6) |
| D6 DSL queries, rows loaded as tuples and mapped into the existing structs | `#[derive(Queryable)]` model structs for every table | No new model layer (principle 4); public structs stay free of Diesel derives; the mapping sits next to the query as today |
| D7 `sql_query` only for pragmas and the carry-over | Raw SQL everywhere | Build-time schema checks are the reason for this spec |
| D8 Hand-written `src/schema.rs`, checked by `tests/schema.rs` | `diesel print-schema` with `diesel_cli` | No extra tool on developer machines; the test catches drift |
| D9 No `down.sql` | Write reverts | A revert deletes data (principle 6); Diesel only needs `down.sql` to revert (R3) |
| D10 Test fixtures for the "previous release" are built with the old DDL through `batch_execute` | Keep `rusqlite` as a dev-dependency or check in a binary database file | Removes the dependency completely; the fixture is readable and versioned |

## 8. Deviations from the constitution

| Deviation | Why it is necessary | Why the simple alternative does not work |
|---|---|---|
| Principle 3 at task level: the tasks that port the store modules (switch group) are verified with the full gate only at the end of the group, and are committed as one commit | `Store::with_conn`/`with_tx` hand every module the same connection type, so changing it breaks every module until all are ported; nothing in between compiles | (a) Bridging `rusqlite` and Diesel through raw `sqlite3*` handles needs `unsafe` code that is thrown away at the end. (b) Two connections to one shared-cache in-memory database use table-level locks that return `SQLITE_LOCKED`, which the busy timeout does not cover, so concurrent tests would be flaky. (c) One task larger than 30 minutes hides progress. Every task outside the group keeps the full gate |

## 9. Test strategy

- Unit-level integration on the database (`tests/db.rs`, `tests/db_upgrade.rs`, `tests/db_pool.rs`, `tests/schema.rs`). Fixtures:
  - Previous-release database: a file with `schema_migrations` created by the old runner's DDL, migrations 1 (and 2) applied with fixed `applied_at` values, and rows in every table (user, token, project, membership, spec, documents, revisions, audit, session).
  - Its rows are dumped before and after `Store::open` with `SELECT * ... ORDER BY rowid` for every table and compared (FR-1, FR-3).
  - Failures are injected by pre-creating `sessions` in a migration-1-only database, so migration 0002 fails (FR-6), and by an empty `schema_migrations` with the tables present (edge case).
- Behaviour: the existing suites of specs 001–003 run unchanged apart from helper code (FR-7). The existing `concurrent_puts_exactly_one_wins` runs on the in-memory store. `tests/db_pool.rs` repeats it on a file database with a real multi-connection pool, 20 rounds (FR-8), and adds a failing-write check (FR-9).
- Pool settings: 8 threads hold 8 pooled connections at once behind a barrier. Each checks `PRAGMA foreign_keys`, `PRAGMA busy_timeout` and `PRAGMA journal_mode` and tries a membership insert for a missing project (FR-10, FR-11).
- Performance (ignored tests, release build): `tests/perf.rs` (NFR-1), `tests/perf_import.rs` (NFR-2) and `tests/perf_open.rs` (NFR-4) run in validation.
- Manual: copy the developer's existing database and run the steps in spec section 10.

## 10. FR coverage

| FR | Module/file | Planned test |
|---|---|---|
| FR-1 | `src/db.rs` `carry_over_history` | `tests/db_upgrade.rs::previous_release_opens_unchanged` |
| FR-2 | `src/db.rs` `carry_over_history` | `tests/db_upgrade.rs::carry_over_keeps_apply_times` |
| FR-3 | `src/db.rs` `migrate` | `tests/db_upgrade.rs::release_with_only_first_migration_gets_second` |
| FR-4 | `src/db.rs` `carry_over_history` | `tests/db_upgrade.rs::previous_history_table_is_kept` |
| FR-5 | `src/db.rs` `carry_over_history` | `tests/db_upgrade.rs::second_open_changes_no_history` |
| FR-6 | `src/db.rs` `migrate`, `src/model.rs` | `tests/db_upgrade.rs::failing_migration_leaves_database_untouched`, `tests/db_upgrade.rs::empty_previous_history_is_refused`, `tests/cli.rs::open_failure_names_migration_step` |
| FR-7 | all `src/store/*` | full `cargo test` suite of specs 001–003 |
| FR-8 | `src/store/mod.rs` `with_tx` | `tests/db_pool.rs::file_pool_puts_exactly_one_wins`, `tests/store_documents.rs::concurrent_puts_exactly_one_wins` |
| FR-9 | `src/store/mod.rs` `with_tx`, `src/store/documents.rs` | `tests/db_pool.rs::failed_write_leaves_no_rows`, `tests/store_audit.rs::with_tx_rolls_back_on_error` |
| FR-10 | `src/db.rs` `ConnectionSettings` | `tests/db_pool.rs::every_pooled_connection_enforces_foreign_keys` |
| FR-11 | `src/db.rs` `set_wal`, `ConnectionSettings` | `tests/db_pool.rs::every_pooled_connection_waits_and_uses_wal`, `tests/db.rs::open_enables_wal_and_foreign_keys` |
| FR-12 | `src/db.rs` `migrate` | `tests/db.rs::open_records_diesel_history_only` |
| FR-13 | `src/db.rs` `open_memory_pool` | `tests/db.rs::open_in_memory_runs_migrations_too`, `tests/db.rs::memory_store_has_connection_settings` |

NFR-1: `tests/perf.rs` · NFR-2: `tests/perf_import.rs` · NFR-3: `tests/cli.rs` runs the built binary from a temporary directory with no `migrations/` next to it · NFR-4: `tests/perf_open.rs`.

## 11. Risks

- An in-memory pool loses the database if r2d2 recycles its connection: `idle_timeout` and `max_lifetime` are None, `max_size` and `min_idle` are 1 (R6), and every test that uses `Store::in_memory` exercises this.
- A hand-written `schema.rs` can drift from the migrations: `tests/schema.rs` compares them.
- Query semantics can change during the port (NULL matching, ordering, `LIMIT` with `has_more`): `.is()` is used where the SQL used `IS` (R7), ordering is ported explicitly, and the frozen suite (FR-7) covers every listing.
- The switch group leaves the branch red for several tasks: they are done in one sitting and committed together (section 8).
- DSL compile errors can be long for joined queries: complex joins are split into small typed helper functions, as the current code already does.
