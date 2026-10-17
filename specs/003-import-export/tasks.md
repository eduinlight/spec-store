# Tasks — Spec 003

Plan: `specs/003-import-export/plan.md` · Date: 2026-10-17

Format: `- [ ] Tnn [P] (FR-x, FR-y) Description with path — Done when: verifiable criterion`
`[P]` = can run in parallel with other `[P]` tasks of the same phase (no shared files).
Each task < 30 minutes. Tests before implementation.

## Phase 1 — Setup
- [x] T01 (FR: —) Create empty `src/tree.rs` and `src/store/transfer.rs`, register `pub mod tree` in `src/lib.rs` and `pub mod transfer` in `src/store/mod.rs` — Done when: `cargo build`, `cargo test` and `cargo clippy --all-targets -- -D warnings` pass

## Phase 2 — Foundation (blocks the stories)
- [x] T02 (FR-1, FR-3, FR-5, NFR-3) Write unit tests in `src/tree.rs`: `parse_spec_folder` accepts `001-store-core`, `042-a`, `1000-x` and rejects `0001-x`, `01-x`, `000-x`, `001-Name`, `001-a-b-c-d-e`, `001`, `abc-x`, `001-a/b`, `001-..`; `status_word` reads `approved` from `Status: approved · Created` and `draft` from the template line, `None` without a line; `version_line` reads `1.0` from `Version 1.0 · Ratified` and `None` without a line — Done when: tests written and red
- [x] T03 (FR-1, FR-3, FR-5, NFR-3) Implement `parse_spec_folder`, `status_word` and `version_line` in `src/tree.rs` — Done when: T02 tests green

## Phase 3 — US-1 (P1) Import an existing tree
- [x] T04 (FR-1, FR-2, FR-6, FR-8) Write `tests/tree.rs` scan tests over temp directories: constitution and two spec folders read with documents in kind order and exact bytes (BOM, CRLF, no final newline); `specs/README.md` and `specs/001-x/notes.md` and `specs/001-x/contracts/` skipped; invalid folder name, duplicate number, duplicate slug, non-UTF-8 file and a file of `MAX_BODY_BYTES + 1` each add a problem while a file of exactly `MAX_BODY_BYTES` does not; empty directory adds a problem; missing directory is an `io::Error` — Done when: tests written and red
- [x] T05 (FR-1, FR-2, FR-6, FR-8) Implement `ProjectTree`, `TreeFile`, `TreeSpec` and `scan` in `src/tree.rs` — Done when: T04 tests green
- [x] T06 (FR-1, FR-2, FR-3, FR-4, FR-5, FR-6, FR-7, FR-9, FR-10, FR-11, FR-13) Write `tests/store_transfer.rs` import tests: owner imports a scanned tree with numbers, slugs, branches, statuses and byte-identical bodies; revisions authored by the owner with `import <path>` messages; audit has `spec.import`, `document.put`, `constitution.put`; status fallback reported; version from the line and from the option; missing version refused; existing number, existing slug and existing constitution refused with every problem listed and counts unchanged; tree problems are listed too; editor and non-owner admin get `Forbidden`; dry run returns the report and changes nothing; a trigger that aborts on a marked body leaves counts unchanged; `create_spec` after import gets highest + 1 — Done when: tests written and red
- [x] T07 (FR-1, FR-2, FR-3, FR-4, FR-5, FR-6, FR-7, FR-9, FR-10, FR-11, FR-13) Implement `ImportOptions`, `ImportReport`, `ImportedSpec` and `Store::import_tree` in `src/store/transfer.rs` — Done when: T06 tests green
- [x] T08 (FR-5, FR-6, FR-8, FR-10, FR-11, FR-12, NFR-2) Write `tests/cli_transfer.rs` import tests: success prints the constitution line, one line per spec and the summary; skipped entries and status fallback on stderr; `--dry-run` prints the dry-run summary and the store stays empty; refusal exits 1 with every problem on stderr and empty stdout; editor `--as` fails; `--constitution-version` overrides — Done when: tests written and red
- [x] T09 (FR-5, FR-6, FR-8, FR-10, FR-11, FR-12, NFR-2) Implement the `import` subcommand in `src/cli.rs` — Done when: T08 tests green

## Phase 4 — US-2 (P1) Export a project to files
- [x] T10 [P] (FR-14, FR-16, FR-17) Extend `tests/tree.rs` with `plan_export` and `write_export`: targets classified new, unchanged, different; write creates missing folders and new files; unchanged files keep their modification time; different files are kept without force and overwritten with force; unrelated files in the directory remain — Done when: tests written and red
- [x] T11 (FR-14, FR-16, FR-17) Implement `ExportFile`, `ExportPlan`, `plan_export` and `write_export` in `src/tree.rs` — Done when: T10 tests green
- [ ] T12 [P] (FR-14, FR-15, FR-18, FR-19, FR-20, NFR-3) Extend `tests/store_transfer.rs` with export: reader and non-member admin get the constitution first then spec documents by number and kind with current bodies; every path is `docs/constitution.md` or `specs/<branch>/<kind>.md`; `Some(number)` filters and unknown number is `NotFound`; outsider `Forbidden`; status mismatch gives a notice; revision and audit counts unchanged — Done when: tests written and red
- [ ] T13 (FR-14, FR-15, FR-18, FR-19, FR-20, NFR-3) Implement `ExportedDocument`, `StatusNotice` and `Store::export_documents` in `src/store/transfer.rs` — Done when: T12 tests green
- [ ] T14 (FR-14, FR-15, FR-16, FR-17, FR-18, FR-19, FR-20, NFR-2) Extend `tests/cli_transfer.rs` with export: files written and listed with the summary; second run reports unchanged; a modified file makes the export fail with the path listed and no file written; `--force` overwrites; `--spec` exports one spec; status notice on stderr; outsider fails — Done when: tests written and red
- [ ] T15 (FR-14, FR-15, FR-16, FR-17, FR-18, FR-19, FR-20, NFR-2) Implement the `export` subcommand in `src/cli.rs` — Done when: T14 tests green

## Phase 5 — US-3 (P2) Round trip
- [ ] T16 (FR-21) Extend `tests/cli_transfer.rs`: copy this repository's `docs/constitution.md` and `specs/` into a temp directory, import into a new project, export to an empty directory and compare every imported file byte for byte — Done when: test green

## Final phase — Polish and cross-cutting
- [ ] T17 (NFR-1) Write `tests/perf_import.rs` (ignored): 100 specs with five 20 KB documents each imported through `scan` and `import_tree` under 10 s — Done when: `cargo test --release --test perf_import -- --ignored` passes
- [ ] T18 (FR: —) Add the "Import and export" section to `README.md` and run the full lint — Done when: README has the section and `cargo clippy --all-targets -- -D warnings && cargo fmt --check` passes

## Dependencies
- Phase 1 blocks everything; T03 needs T01.
- US-1: T05 needs T03; T07 needs T05; T09 needs T07.
- US-2: T11 needs T01; T13 needs T07 (shared module and fixture); T15 needs T09, T11 and T13.
- US-3: T16 needs T15.
- Polish: T17 needs T07; T18 needs T16.

## Coverage

| FR | Tasks |
|---|---|
| FR-1 | T02, T03, T04, T05, T06, T07 |
| FR-2 | T04, T05, T06, T07 |
| FR-3 | T02, T03, T06, T07 |
| FR-4 | T06, T07 |
| FR-5 | T02, T03, T06, T07, T08, T09 |
| FR-6 | T04, T05, T06, T07, T08, T09 |
| FR-7 | T06, T07 |
| FR-8 | T04, T05, T08, T09 |
| FR-9 | T06, T07 |
| FR-10 | T06, T07, T08, T09 |
| FR-11 | T06, T07, T08, T09 |
| FR-12 | T08, T09 |
| FR-13 | T06, T07 |
| FR-14 | T10, T11, T12, T13, T14, T15 |
| FR-15 | T12, T13, T14, T15 |
| FR-16 | T10, T11, T14, T15 |
| FR-17 | T10, T11, T14, T15 |
| FR-18 | T12, T13, T14, T15 |
| FR-19 | T12, T13, T14, T15 |
| FR-20 | T12, T13, T14, T15 |
| FR-21 | T16 |
| NFR-1 | T17 |
| NFR-2 | T08, T09, T14, T15 |
| NFR-3 | T02, T03, T12, T13 |
