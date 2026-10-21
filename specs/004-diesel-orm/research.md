# Research — Spec 004

Spec: `specs/004-diesel-orm/spec.md` · Date: 2026-10-21

All registry paths below are relative to `~/.asdf/installs/rust/1.97.1/registry/src/index.crates.io-1949cf8c6b5b557f/`.

## R1 — Versions and SQLite linkage

- Question: which crate versions can be used, and can the bundled SQLite be kept?
- Finding: the latest releases are `diesel` 2.3.14, `diesel_migrations` 2.3.2 and `r2d2` 0.8.10. Diesel's `sqlite` feature depends on `libsqlite3-sys` but has no `bundled` feature of its own, so bundling is enabled by depending on `libsqlite3-sys` with `bundled`. A probe crate with both `diesel` (sqlite, r2d2) and `rusqlite` 0.40 resolves to one `libsqlite3-sys` 0.38.2 shared by both, so the two libraries can live in the same build while the port goes module by module.
- Source: `cargo search` and `cargo info diesel@2.3.14` (2026-10-21); `cargo tree -i libsqlite3-sys` on a probe crate; https://docs.rs/diesel/2.3.14/diesel/
- Decision: `diesel = { version = "2.3", default-features = false, features = ["sqlite", "r2d2", "returning_clauses_for_sqlite_3_35"] }`, `diesel_migrations = { version = "2.3", features = ["sqlite"] }`, `libsqlite3-sys = { version = "0.38", features = ["bundled"] }`. `rusqlite` is removed in the last task.

## R2 — Name and shape of Diesel's migration history

- Question: where does Diesel record applied migrations, and in what format?
- Finding: the harness creates `__diesel_schema_migrations (version VARCHAR(50) PRIMARY KEY NOT NULL, run_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP)` with `CREATE TABLE IF NOT EXISTS` before it reads the applied versions.
- Source: `diesel-2.3.14/src/migration/setup_migration_table.sql`; `diesel_migrations-2.3.2/src/migration_harness.rs` (`applied_migrations` calls `setup_database`).
- Decision: the carry-over creates the table with the same statement, then copies rows into it. `run_on` is SQLite TEXT, so the previous `applied_at` text is copied as is (FR-2).

## R3 — Version of an embedded migration

- Question: what version string does a migration folder get?
- Finding: `version_from_string` takes the part of the folder name before the first `_` and removes `-`. A folder holds `up.sql` and, optionally, `down.sql`; a missing `down.sql` is an error only when reverting.
- Source: `migrations_internals-2.3.0/src/lib.rs` line 71; `diesel_migrations-2.3.2/src/errors.rs` line 27; `diesel_migrations-2.3.2/src/lib.rs` line 27.
- Decision: the folders are `migrations/0001_init/up.sql` and `migrations/0002_web_ui/up.sql`, so the versions are `0001` and `0002`. Previous number `n` maps to `format!("{n:04}")`. There is no `down.sql`, because reverting would delete data (constitution principle 6).

## R4 — Unknown applied versions and transactions

- Question: what does the harness do with an applied version that has no migration, and does it wrap migrations in transactions?
- Finding: `pending_migrations` removes applied versions from the known set and ignores the rest, so an unknown applied version does not fail. `run_migration` runs the migration and its history insert inside `self.transaction(...)` when `run_in_transaction` is true, which is the default.
- Source: `diesel_migrations-2.3.2/src/migration_harness.rs` lines 111–131 and 176–197.
- Decision: the edge case "number the binary does not know" needs no code beyond not carrying it over. Each migration stays all-or-nothing (FR-6).

## R5 — Write transactions with several connections

- Question: today one mutex serializes every access. With a pool, can a read-then-write transaction fail without waiting on the busy timeout?
- Finding: Diesel's `Connection::transaction` on SQLite issues a deferred `BEGIN`. A deferred transaction that read first and then tries to write while another connection has committed a write gets `SQLITE_BUSY` immediately. The busy handler is not used for that lock upgrade, because waiting could deadlock. `SqliteConnection::immediate_transaction` issues `BEGIN IMMEDIATE`, which takes the write lock up front and waits on the busy timeout.
- Source: `diesel-2.3.14/src/sqlite/connection/mod.rs` lines 360–395; SQLite documentation https://www.sqlite.org/lang_transaction.html ("BEGIN IMMEDIATE") and https://www.sqlite.org/c3ref/busy_handler.html (deadlock note).
- Decision: `Store::with_tx` uses `immediate_transaction`. Writers are serialized as they are today, and optimistic locking keeps its meaning (FR-8, FR-9).

## R6 — Pool settings and in-memory databases

- Question: how are per-connection pragmas applied, and is an in-memory database safe in a pool?
- Finding: `r2d2::Builder` has `max_size` (default 10), `connection_timeout` (default 30 s), `idle_timeout` (default 10 min), `max_lifetime` (default 30 min) and `connection_customizer`. `CustomizeConnection::on_acquire` runs on every new connection. Each SQLite `:memory:` connection is a separate empty database, and r2d2 closes connections when they reach the idle timeout or max lifetime.
- Source: `r2d2-0.8.10/src/config.rs` lines 56–61, 102–198; `diesel-2.3.14/src/r2d2.rs` (`CustomizeConnection` example); https://www.sqlite.org/inmemorydb.html
- Decision: a customizer sets `busy_timeout = 5000` and `foreign_keys = ON` on every connection (FR-10, FR-11). File pools have `max_size` 8 and `connection_timeout` 5 s. In-memory pools have `max_size` 1, `min_idle` 1, `idle_timeout` None and `max_lifetime` None, so the single database is never dropped (FR-13).

## R7 — SQLite-specific expressions

- Question: the current SQL uses `spec_id IS ?` to match NULL, and `last_insert_rowid()`. What are the Diesel equivalents?
- Finding: `diesel::sqlite::SqliteExpressionMethods` provides `is` and `is_not`. With `returning_clauses_for_sqlite_3_35`, `insert_into(..).returning(col)` returns the new id, and the bundled SQLite 3.5x supports `RETURNING`.
- Source: `diesel-2.3.14/src/sqlite/expression/expression_methods.rs` lines 15–79; `cargo info diesel@2.3.14` feature list.
- Decision: queries use the DSL with `.is(...)` and `.returning(id)`. `diesel::sql_query` is used only for pragmas and for the carry-over statements.

## R8 — Where database errors reach users

- Question: does replacing the error type change any client-visible text?
- Finding: `StoreError::Database` is mapped to a generic internal error in `src/mcp.rs` (`"database error"`) and `src/ui/mod.rs` (`UiError::Internal`). The error detail only goes to `tracing`.
- Source: `src/mcp.rs` lines 418–421; `src/ui/mod.rs` lines 54–57.
- Decision: `StoreError::Database` wraps `diesel::result::Error`. A new `StoreError::Pool` wraps `diesel::r2d2::PoolError`. Both map to the same generic responses, and the client text stays the same.
