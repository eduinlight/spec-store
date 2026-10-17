# Technical plan — Spec 003

Spec: `specs/003-import-export/spec.md` · Constitution: v1.0 · Date: 2026-10-17

## 1. Summary

A new module `src/tree.rs` reads a project directory into an in-memory tree (constitution, spec folders, skipped entries, problems) and writes an export file list to a directory. Two store operations in `src/store/transfer.rs` turn a tree into specs, documents and revisions inside one transaction, and collect the current documents of a project for export. Two CLI subcommands, `import` and `export`, connect them with the same `--db` and `--as HANDLE` model as the other administration commands. No schema change and no new crate.

## 2. Technical context

- Language/version: Rust 1.97, edition 2024.
- Framework/platform: existing `clap` CLI over the `Store`.
- Dependencies: none added (research R6).
- Storage: existing tables `specs`, `documents`, `revisions`, `audit_log`; no migration.
- Tests: integration tests in `tests/` (store level and binary level), unit tests in `src/tree.rs`.
- Constraints: one transaction per import (R1); deterministic order (R2); byte-exact bodies (R3).
- Unknowns: none left; R1–R6 resolved.

## 3. Constitution check (gate)

| Principle | Complies | Note |
|---|---|---|
| 1. The spec rules | yes | Spec 003 approved; everything below maps to an FR |
| 2. Stack and limits | yes | Standard library only; no new dependency |
| 3. Tests as a gate | yes | Every implementation task follows its test task |
| 4. Simplicity | yes | One file-system module and one store module; no trait or abstraction layer |
| 5. Markdown is the source of truth | yes | Bodies stored and exported as given; status and version are read from the markdown lines |
| 6. Nothing is lost | yes | Import only appends revisions with audit entries; export never deletes files and overwrites only with `--force` |
| 7. Access is scoped | yes | Import requires owner, export requires membership or admin, checked by `access_in` |
| 8. Reference sources | yes | rusqlite transaction source and std docs cited in research.md |
| 9. Language | yes | English everywhere |
| 10. Human approval | yes | Overwriting files needs the explicit `--force` flag; commits under the run authorization |

## 4. Module / file structure

| File | Responsibility |
|---|---|
| `src/tree.rs` (new) | `ProjectTree`, `TreeFile`, `TreeSpec`, `scan(dir)`, `parse_spec_folder`, `status_word`, `version_line`, `ExportFile`, `TargetState`, `ExportPlan`, `plan_export(dir, files)`, `write_export(plan, force)` |
| `src/store/transfer.rs` (new) | `ImportOptions`, `ImportReport`, `ImportedSpec`, `Store::import_tree`, `ProjectExport`, `StatusNotice`, `Store::export_documents` |
| `src/store/specs.rs` | `spec_rows_in` lists a project's spec rows by number, shared by `list_specs_in` and the export |
| `src/store/mod.rs` | `pub mod transfer` |
| `src/lib.rs` | `pub mod tree` |
| `src/cli.rs` | `Import` and `Export` subcommands and their output |
| `tests/tree.rs` (new) | scanning and export writing against temp directories |
| `tests/store_transfer.rs` (new) | import and export at store level |
| `tests/cli_transfer.rs` (new) | binary contract and the round trip over this repository's tree |
| `tests/perf_import.rs` (new, ignored) | NFR-1 |
| `README.md` | "Import and export" section |

## 5. Data model

No new table. An import writes:

- `specs(project_id, number, slug, status, branch)` with the number and slug from the folder, the status from FR-5 and `branch = branch_name(number, slug)`.
- One `documents` row and one `revisions` row per file, `author_id` = acting user, `message = "import <relative path>"`, `version` set only for the constitution.
- Audit: `spec.import` with target `<branch>`, `document.put` with target `<NNN>/<kind>@<revision>`, `constitution.put` with target `v<version>@<revision>`.

In-memory types:

- `TreeFile { relative_path: String, body: String }`.
- `TreeSpec { folder: String, number: i64, slug: String, documents: Vec<(DocumentKind, TreeFile)> }` in kind order.
- `ProjectTree { constitution: Option<TreeFile>, specs: Vec<TreeSpec>, skipped: Vec<String>, problems: Vec<String> }`.
- `ImportOptions { constitution_version: Option<String>, dry_run: bool }`.
- `ImportReport { constitution_version: Option<String>, specs: Vec<ImportedSpec>, status_fallbacks: Vec<String>, documents: usize }`, `ImportedSpec { branch, status, kinds }`.
- `ExportFile { relative_path, body }` (shared by the store and the writer), `StatusNotice { branch, stored: SpecStatus, written: String }`, `ProjectExport { files: Vec<ExportFile>, notices: Vec<StatusNotice> }`.

## 6. Contracts / interfaces

`tree::scan(dir: &Path) -> io::Result<ProjectTree>`: reads `docs/constitution.md` if present; lists `specs/` sorted by name; files there go to `skipped`; folders with an invalid name add a problem; inside a valid folder, the five document files are read, anything else goes to `skipped`. Each read file adds a problem when its byte length exceeds `MAX_BODY_BYTES` or it is not UTF-8. Duplicate numbers or slugs among folders add a problem. A tree with no constitution and no spec folder adds a problem. `io::Error` only for an unreadable or missing directory.

`tree::status_word(body) -> Option<&str>`: first word after `Status:` on the first line that starts with `Status:`. `tree::version_line(body) -> Option<&str>`: first whitespace-delimited token after `Version ` on the first line that starts with `Version `.

`Store::import_tree(caller, project, tree, options) -> StoreResult<ImportReport>` (FR-1..FR-13): inside one transaction, `access_in` then owner check (`Forbidden` otherwise); collects store problems (number or slug already in the project, project already has a constitution, constitution without version) after `tree.problems`; any problem gives `StoreError::Validation` with one problem per line. With `dry_run` it returns the report before writing. Otherwise it inserts specs with their status, documents, revisions and audit entries in folder order.

`Store::export_documents(caller, project, spec: Option<i64>) -> StoreResult<ProjectExport>` (FR-14, FR-15, FR-18, FR-19): `access_in` (members and admins), constitution first then specs by number and documents in kind order; `NotFound` for an unknown spec number.

`tree::plan_export(dir, files) -> io::Result<ExportPlan>` classifies each target as `New`, `Unchanged` or `Different`; `tree::write_export(&plan, force) -> io::Result<()>` creates folders and writes `New` and, with `force`, `Different` targets. The CLI refuses before writing when `Different` targets exist without `--force` (FR-16, FR-17).

CLI (NFR-2):

```
spec-store import <PROJECT> <DIR> --as <HANDLE> [--constitution-version <V>] [--dry-run]
spec-store export <PROJECT> <DIR> --as <HANDLE> [--spec <NUMBER>] [--force]
```

Import stdout: `constitution v<V> from docs/constitution.md`, `spec <branch> <status>: <kinds joined by ", " or "no documents">`, then `imported <n> specs and <m> documents` or, with `--dry-run`, `dry run: would import <n> specs and <m> documents`. Stderr: `skipped <path>` and `notice: spec <branch> has no recognised Status line; imported as draft`. Refusal: exit 1, stderr `error: import refused:` followed by one `  - <problem>` line per problem.

Export stdout: `written <path>` or `unchanged <path>` per file, then `exported <n> files: <w> written, <u> unchanged`. Stderr: `notice: spec <branch> is <stored> in the store but its spec.md says <written>`. Refusal for differing files: exit 1, `error: export refused, files differ (use --force to overwrite):` and one `  - <path>` per file.

## 7. Technical decisions

| Decision | Discarded alternative | Why |
|---|---|---|
| Read the whole tree into memory before touching the store | Stream files while writing | Every problem must be listed before anything is written (FR-6); bodies are capped at 1 MiB |
| Problems are collected as strings and returned together | Fail on the first problem | FR-6 asks for every problem in one run |
| Store writes use `append_revision` and direct inserts inside `import_tree` | Call `create_spec`, `set_spec_status`, `put_document` in sequence | Those open their own transactions, assign numbers and enforce transitions, which breaks atomicity, numbering and direct status restore |
| Status and version from template lines with a prefix match | A markdown parser | The lines are plain text in the templates; a prefix match is enough and predictable |
| Export compares before writing and refuses as a whole | Write what it can and report the rest | FR-16 says no file is written when one conflicts |
| Separate `tree` (file system) and `store::transfer` (database) modules | One module | Store code stays free of file-system access and the scanner is testable without a database |

## 8. Deviations from the constitution

None.

## 9. Test strategy

- Unit tests in `src/tree.rs`: `parse_spec_folder` valid and invalid names, `status_word`, `version_line`.
- `tests/tree.rs`: scan of temp directories (documents in kind order, skipped entries, invalid names, duplicates, non-UTF-8, size limit and limit + 1, empty tree, BOM and CRLF kept); `plan_export` and `write_export` (new, unchanged, different, force, folders created, unrelated files kept).
- `tests/store_transfer.rs`: import as owner (numbers, slugs, statuses, bodies, revision author and message, audit actions), status fallback, version from line and from option, missing version, conflicts with existing numbers, slugs and constitution, editor and admin non-owner refused, dry run writes nothing, forced mid-import failure leaves counts unchanged, next `create_spec` number; export for reader and admin, outsider refused, `--spec` filter and unknown spec, status notice, no revision or audit entry added.
- `tests/cli_transfer.rs`: binary output and exit codes for import, dry run, refusal, export, conflict, `--force`, and the round trip of this repository's `docs/constitution.md` and `specs/` compared byte for byte.
- `tests/perf_import.rs` (ignored): 100 specs × 5 documents × 20 KB under 10 s in release.

## 10. FR coverage

| FR | Module/file | Planned test |
|---|---|---|
| FR-1 | `tree::scan`, `Store::import_tree` | `tests/tree.rs`, `tests/store_transfer.rs` |
| FR-2 | `tree::scan`, `Store::import_tree` | `tests/tree.rs`, `tests/store_transfer.rs` |
| FR-3 | `tree::version_line`, `Store::import_tree` | `src/tree.rs` unit, `tests/store_transfer.rs` |
| FR-4 | `Store::import_tree` | `tests/store_transfer.rs` |
| FR-5 | `tree::status_word`, `Store::import_tree` | `src/tree.rs` unit, `tests/store_transfer.rs`, `tests/cli_transfer.rs` |
| FR-6 | `tree::scan`, `Store::import_tree` | `tests/tree.rs`, `tests/store_transfer.rs`, `tests/cli_transfer.rs` |
| FR-7 | `Store::import_tree` | `tests/store_transfer.rs` (trigger) |
| FR-8 | `tree::scan`, CLI | `tests/tree.rs`, `tests/cli_transfer.rs` |
| FR-9 | `Store::import_tree` | `tests/store_transfer.rs` |
| FR-10 | `Store::import_tree` | `tests/store_transfer.rs`, `tests/cli_transfer.rs` |
| FR-11 | `Store::import_tree`, CLI | `tests/store_transfer.rs`, `tests/cli_transfer.rs` |
| FR-12 | CLI | `tests/cli_transfer.rs` |
| FR-13 | existing `create_spec` | `tests/store_transfer.rs` |
| FR-14 | `Store::export_documents`, `tree::write_export` | `tests/store_transfer.rs`, `tests/tree.rs`, `tests/cli_transfer.rs` |
| FR-15 | `Store::export_documents` | `tests/store_transfer.rs`, `tests/cli_transfer.rs` |
| FR-16 | `tree::plan_export`, CLI | `tests/tree.rs`, `tests/cli_transfer.rs` |
| FR-17 | `tree::write_export` | `tests/tree.rs`, `tests/cli_transfer.rs` |
| FR-18 | `Store::export_documents`, CLI | `tests/store_transfer.rs`, `tests/cli_transfer.rs` |
| FR-19 | `Store::export_documents` | `tests/store_transfer.rs`, `tests/cli_transfer.rs` |
| FR-20 | CLI, `Store::export_documents` | `tests/cli_transfer.rs`, `tests/store_transfer.rs` |
| FR-21 | whole flow | `tests/cli_transfer.rs` round trip |
| NFR-1 | `Store::import_tree` | `tests/perf_import.rs` |
| NFR-2 | CLI | `tests/cli_transfer.rs` |
| NFR-3 | `tree::parse_spec_folder`, `Store::export_documents` | `src/tree.rs` unit, `tests/store_transfer.rs` (every path under `docs/` or `specs/<branch>/`) |

## 11. Risks

- Large trees held in memory: bounded by 1 MiB per file; a 100-spec tree is about 10 MB in the NFR-1 scenario.
- Status lines written differently from the template: covered by the draft fallback with a notice (FR-5).
- Running import against the live database while the server is up: SQLite WAL and the busy timeout of spec 001 serialise the writers; the import is one short transaction.
