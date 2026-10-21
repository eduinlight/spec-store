# Spec 004 — Diesel ORM

Status: approved · Created: 2026-10-21 · Constitution: v1.1

## 1. Context and goal

Specs 001 to 003 store everything in one SQLite file. Today every store module writes its own SQL by hand and maps rows to values by column position. Because of that, a renamed or retyped column is caught only at run time, and every new query repeats the same row-mapping code. Constitution v1.1 replaced `rusqlite` with `diesel` as the base database library. This spec moves the persistence layer onto it as a pure refactor. Every database already in use must keep working without manual steps and without losing data or migration history. Agents, CLI users and web UI users must not be able to tell that anything changed.

## 2. Users / actors

- Server operator: upgrades the `spec-store` binary on a host that already has a database file and expects it to start with every project, document, revision, audit entry, user, token and session intact.
- Agent, CLI user and web UI user: keep using the MCP tools, the JSON API, the CLI and the web UI exactly as specs 001 to 003 define them.
- Maintainer: changes the schema or adds queries, and gets schema mismatches reported when the code is built instead of at run time.

## 3. Current state

| Current piece | What it existed for | Decision | Justification (doc/code) |
|---|---|---|---|
| Hand-written SQL and positional row mapping in `src/store/*` | Access to every table | replace | Constitution v1.1 principle 2 names `diesel` as the base; queries are checked against the schema at build time |
| One shared connection behind a mutex (`src/store/mod.rs`) | Serialized access to SQLite from the async server | replace | A pool of synchronous connections; each connection gets the same settings the single connection has today |
| Connection settings: WAL journal for files, 5 s busy timeout, foreign keys on (`src/db.rs::open`, `configure`) | Concurrent readers, waiting on locks, referential integrity | keep | Behaviour of specs 001 to 003 depends on them; they are applied to every connection |
| Migration runner with its own `schema_migrations` table and migrations embedded in the binary (`src/db.rs::migrate`) | Numbered migrations applied once, each in a transaction (constitution principle 6) | replace | Diesel's embedded migrations keep the "one binary, numbered migrations" rule; existing history is carried over once so nothing is applied twice |
| Migration SQL `0001_init.sql`, `0002_web_ui.sql` | Schema of specs 001 and 002 | keep | The schema does not change; the same statements become Diesel migrations 1 and 2 |
| Transactions around revision + audit + index writes (`src/store/documents.rs`, `transfer.rs`) | Append-only revisions with audit and optimistic locking; all-or-nothing import (constitution principle 6, spec 003 FR-7) | keep | Same transaction boundaries, expressed through the new library |

## 4. User stories (by priority)

### US-1 (P1) — Upgrade without losing anything
As a server operator I want the new binary to open my existing database file as it is so that no data or migration history is lost and no migration runs twice.
Independent test: build a database with the previous release, fill it with a project, a spec with documents and revisions, a user with a token and a session, start the new binary on it and read everything back unchanged.

### US-2 (P1) — Unchanged behaviour
As an agent, CLI user or web UI user I want every operation to behave exactly as before so that the refactor needs no change on my side.
Independent test: the full test suite of specs 001 to 003 passes with no assertion changed.

### US-3 (P2) — New databases
As a server operator I want a new database file to get the full schema and a recorded migration history so that later upgrades work the same way.
Independent test: start the binary on a path that does not exist and check the tables and the recorded migrations.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): WHEN the system opens a database file whose previous migration history records migrations 1 and 2, THE SYSTEM SHALL record both as applied in the new migration history, apply no migration, and leave every row of every data table unchanged.
- **FR-2** (US-1): WHEN the system carries over a migration from the previous history, THE SYSTEM SHALL record it with the apply time stored in the previous history.
- **FR-3** (US-1): WHEN the system opens a database file whose previous history records only migration 1, THE SYSTEM SHALL carry over migration 1 and apply migration 2 once.
- **FR-4** (US-1): THE SYSTEM SHALL keep the previous migration history table and all of its rows after the carry-over.
- **FR-5** (US-1): WHEN the system opens a database file whose carry-over was already done, THE SYSTEM SHALL record no new history row and apply no migration.
- **FR-6** (US-1): IF the carry-over or a migration fails, THEN THE SYSTEM SHALL leave the schema, the data and both migration histories as they were before the failing step and refuse to start, with an error on the error stream naming the step.
- **FR-7** (US-2): THE SYSTEM SHALL pass every test that exists for specs 001, 002 and 003 when this spec starts, with no assertion changed or removed, except the assertions about the previous migration runner's own history table, which are replaced by tests of FR-1 to FR-6 and FR-12; test helper code that reads the database directly may change.
- **FR-8** (US-2): WHILE several requests run at the same time, THE SYSTEM SHALL apply optimistic locking as spec 001 defines it: of two writes to the same document with the same expected revision, exactly one succeeds and the other gets the conflict error.
- **FR-9** (US-2): IF a document write fails at any step, THEN THE SYSTEM SHALL store no revision, no audit entry and no index row for that write.
- **FR-10** (US-2): THE SYSTEM SHALL enforce foreign keys on every database connection it uses: a membership that references a missing project is refused on any connection.
- **FR-11** (US-2): WHERE the database is a file, THE SYSTEM SHALL keep it in WAL journal mode, and every connection SHALL wait up to 5 seconds for a lock before it reports the database as busy.
- **FR-12** (US-3): WHEN the system opens a path where no database exists, THE SYSTEM SHALL create the schema of migrations 1 and 2, record both in the new migration history and create no previous migration history table.
- **FR-13** (US-3): WHEN the system opens an in-memory database for tests, THE SYSTEM SHALL give it the same schema and connection settings as a file database, except the journal mode.

## 6. Non-functional requirements

- **NFR-1**: Spec 001 NFR-1 still holds: a document read or derived read of a spec whose documents total up to 200 KB completes in under 100 ms at the 95th percentile with 10 concurrent callers on the reference development machine.
- **NFR-2**: Spec 003 NFR-1 still holds: importing 100 specs with five 20 KB documents each completes in under 10 seconds with a release build on the development machine.
- **NFR-3**: The server is still one `spec-store` binary that needs no other runtime and no migration files next to it (constitution principle 2).
- **NFR-4**: A database with the full specs 001 to 003 schema and history opens in under 1 second on the development machine.

## 7. Key entities (if applicable)

- Previous migration history: the list of migration numbers the earlier binary applied, each with its apply time.
- New migration history: the list the new library keeps; it becomes the only one the system reads after the carry-over.

## 8. Edge cases

- Database whose previous history records a number the binary does not know (made by a newer build): it opens as it does today, the unknown number is not carried over and nothing is applied for it.
- Previous history table exists but is empty, while the data tables exist: treated as a corrupted history; the system refuses to start and names the table (it never re-creates tables that already exist).
- Two server processes started on the same file at the same time: the carry-over runs inside one write transaction, so the second waits on the lock and then sees it as done (FR-5).
- All pooled connections busy: a request waits up to 5 seconds for a free connection and then fails like a busy database does today (internal error to the client, detail only in the log).
- Database opened read-only by the file system: the system refuses to start with the error from the database, as today.
- Document bodies with CRLF, a byte order mark or exactly 1 MB: stored and returned byte for byte, as spec 001 NFR-2 requires.

## 9. Out of scope

- Any schema change: no new tables, columns or indexes.
- Async database access (`diesel-async`) and any database other than SQLite.
- Downgrade: running an earlier binary against a database the new binary has opened is not guaranteed.
- Removing the previous migration history table; it can only go away in a later spec with explicit approval (constitution principle 10).
- Changes to MCP tools, the JSON API, CLI commands, web UI pages, messages or error texts.
- Performance work beyond keeping NFR-1 and NFR-2.

## 10. Completion criteria

- Every FR covered by at least one green test.
- `rusqlite` is no longer a direct dependency in `Cargo.toml`.
- Manual validation: copy a database file used with the release on `main` before this spec, start the new binary on the copy, and list projects, read a document's history and log in through the web UI with an existing account.

## 11. Assumptions

- Synchronous `diesel` with an `r2d2` connection pool, not `diesel-async` (approved by the user on 2026-10-21).
- Migrations are embedded in the binary with Diesel's migration support. The carry-over from the previous history runs once at startup, before pending migrations are checked (approved by the user on 2026-10-21).
- Full scope: every store module, including import/export, revisions and audit, moves to the new library in this spec (approved by the user on 2026-10-21).
- The previous history table is kept rather than dropped, because constitution principle 10 requires explicit approval for deleting data.
- A database whose history records a number the binary does not know keeps today's behaviour (it opens) rather than being refused.
- An in-memory database lives in a single connection, so its pool holds exactly one connection; file databases use a bounded pool whose size is set in the plan.
- Database error details stay out of client responses, as today (`src/mcp.rs`, `src/ui/mod.rs` map them to a generic internal error); only the logged text changes with the library.
- The carry-over writes one info log line per migration it carries over; no other new log output.

## 12. Open questions

None.

## Clarifications

- 2026-10-21 — Q: `tests/db.rs` asserts that a new database has a `schema_migrations` table with 2 rows, which FR-7 would freeze. How is it handled? A: The assertions about the replaced runner's own table are exempt from FR-7 and rewritten against the new history; new databases do not get the previous history table. Affected: FR-7, FR-12.

