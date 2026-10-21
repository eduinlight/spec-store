# Validation — Spec 004

Date: 2026-10-21 · Suite executed: `cargo test` (plus `cargo test --release --test perf --test perf_ui --test perf_import --test perf_open -- --ignored` and `cargo clippy --all-targets -- -D warnings && cargo fmt --check`) · Overall result: 192 pass / 0 fail (4 ignored performance tests, all passing in release)

Test names are `file::function`.

| FR | Test(s) | Result | Notes |
|---|---|---|---|
| FR-1 | `tests/db_upgrade.rs::previous_release_opens_unchanged` | ✅ | Every row of the 10 tables of a release-2 database is dumped with `quote()` before and after the open and compared; history is `0001`, `0002`; the CRLF constitution body reads back byte for byte |
| FR-2 | `tests/db_upgrade.rs::carry_over_keeps_apply_times` | ✅ | `run_on` equals the previous `applied_at` text exactly |
| FR-3 | `tests/db_upgrade.rs::release_with_only_first_migration_gets_second` | ✅ | `0001` keeps its time, `0002` is applied once, `sessions` and `repository_url` appear, release-1 columns unchanged |
| FR-4 | `tests/db_upgrade.rs::previous_history_table_is_kept` | ✅ | `schema_migrations` rows identical after the open |
| FR-5 | `tests/db_upgrade.rs::second_open_changes_no_history` | ✅ | Diesel history identical after a second open |
| FR-6 | `tests/db_upgrade.rs::failing_migration_leaves_database_untouched`, `tests/db_upgrade.rs::empty_previous_history_is_refused`, `tests/cli.rs::open_failure_names_migration_step` | ✅ | A blocking `sessions` table makes `0002` fail: data, table list and both histories unchanged (the `ALTER TABLE` of `0002` and the carry-over are rolled back); the CLI exits 1 with `migration step 0002 failed` on stderr |
| FR-7 | Full suite: every test file of specs 001–003 | ✅ | `git diff main -- tests` shows no changed assertion outside `tests/db.rs`; only helper code moved to `tests/common` (T04) |
| FR-8 | `tests/db_pool.rs::file_pool_puts_exactly_one_wins`, `tests/store_documents.rs::concurrent_puts_exactly_one_wins` | ✅ | 20 rounds on a file pool with 8 connections, one win and one conflict per round, repeated 4 times without a failure |
| FR-9 | `tests/db_pool.rs::failed_write_leaves_no_rows`, `tests/store_audit.rs::with_tx_rolls_back_on_error`, `tests/store_transfer.rs::a_failure_in_the_middle_leaves_the_store_unchanged` | ✅ | A trigger aborts the audit insert after the document and revision were written: no document, revision or audit row remains |
| FR-10 | `tests/db_pool.rs::every_pooled_connection_enforces_foreign_keys` | ✅ | 8 connections held at once; each reports `foreign_keys = 1` and refuses a membership for project 999 |
| FR-11 | `tests/db_pool.rs::every_pooled_connection_waits_and_uses_wal`, `tests/db.rs::open_enables_wal_and_foreign_keys` | ✅ | Each of the 8 connections reports `busy_timeout = 5000` and `journal_mode = wal` |
| FR-12 | `tests/db.rs::open_records_diesel_history_only`, `tests/db.rs::new_file_and_memory_stores_record_both_migrations` | ✅ | No `schema_migrations` table on a new database |
| FR-13 | `tests/db.rs::open_in_memory_runs_migrations_too`, `tests/db.rs::memory_store_has_connection_settings`, `tests/db.rs::new_file_and_memory_stores_record_both_migrations` | ✅ | Same tables and history; foreign keys and busy timeout set |

Non-functional requirements:

| NFR | Test | Result | Notes |
|---|---|---|---|
| NFR-1 | `tests/perf.rs::p95_read_under_100ms` (release, ignored) | ✅ | Also `tests/perf_ui.rs::p95_spec_page_under_300ms` from spec 002 |
| NFR-2 | `tests/perf_import.rs::importing_one_hundred_specs_takes_under_ten_seconds` (release, ignored) | ✅ | 0.22 s |
| NFR-3 | `tests/cli.rs::binary_needs_no_migration_files` | ✅ | Binary run from an empty directory creates a database with history `0001`, `0002` |
| NFR-4 | `tests/perf_open.rs::opening_a_full_database_takes_under_one_second` (release, ignored) | ✅ | Upgrade open and steady-state open of a 101-spec database, both under 1 s |

## Edge cases

| Case | Test | Result |
|---|---|---|
| Previous history records a number the binary does not know | `tests/db_upgrade.rs::unknown_previous_version_is_ignored` | ✅ |
| Previous history empty while the tables exist | `tests/db_upgrade.rs::empty_previous_history_is_refused` | ✅ |
| Two processes start on the same file at once | no test | Covered by design: carry-over and migrations run in one `BEGIN IMMEDIATE` transaction (plan D5), so the second process waits and then finds nothing to do |
| All pooled connections busy | `tests/db_pool.rs::exhausted_pool_fails_after_timeout` | ✅ (fails with `StoreError::Pool` after about 5 s, works again once released) |
| Database read-only on the file system | no test | Not exercised; opening still goes through the same error path (`opening <path>: …`, exit 1) |
| CRLF, byte order mark and 1 MB bodies | `tests/ui_specs.rs` raw view, `tests/store_documents.rs` size tests, `tests/db_upgrade.rs::previous_release_opens_unchanged` (CRLF) | ✅ |

## Manual validation

Release binary of `main` (commit `4d1a940`) against the release binary of this branch, on a scratch copy.

| Step | Observed | Result |
|---|---|---|
| With the `main` binary: add `root`, project `demo`, owner membership, issue a token, import this repository's `docs/` and `specs/`, log in through `/ui/login` | 3 specs, 16 documents, 16 revisions, 1 session; `schema_migrations` holds 1 and 2 | ✅ |
| Copy the file and start `serve` from this branch on the copy | Log shows `carried over migration … version=0001` and `version=0002`; history `0001`/`0002` with the original apply times | ✅ |
| Open `/ui/` with the session cookie issued by the old binary | 200, the existing session still works | ✅ |
| Log in again with the existing token and list projects | 303 to `/ui/`; the page lists `demo` and its specs | ✅ |
| Open the history of spec 001's `spec.md` | 200, shows the `import specs/001-store-core/spec.md` revision | ✅ |
| `sqlite3 .dump` before and after, ignoring the Diesel history table | Only differences: the token's `last_used_at` and the one new session from the logins above | ✅ |

## Verdict

Spec 004 is fulfilled. Every FR and NFR is covered by a green test, and the manual upgrade of a real `main` database worked end to end. No defects were found. The switch group (T03–T12) went green on its first full run.

Two edge cases have no test: simultaneous startup of two processes, and a read-only database file. The first is covered by the single immediate transaction, and the second takes the unchanged open-error path. Neither needs a new task.

One error message changed, as FR-6 requires: when the database cannot be opened, the CLI now prints the cause after `opening <path>:`, so the failing migration step is visible.
