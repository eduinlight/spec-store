# Validation — Spec 003

Date: 2026-10-17 · Suite executed: `cargo test` (plus `cargo test --release --test perf --test perf_ui --test perf_import -- --ignored` and `cargo clippy --all-targets -- -D warnings && cargo fmt --check`) · Overall result: 172 pass / 0 fail (3 ignored performance tests, all passing in release)

Test names are `file::function`; unit tests inside `src/` are named by module.

| FR | Test(s) | Result | Notes |
|---|---|---|---|
| FR-1 | `src/tree.rs::accepts_canonical_spec_folders`, `tests/tree.rs::reads_constitution_and_spec_documents_in_kind_order_with_exact_bytes`, `tests/store_transfer.rs::owner_imports_numbers_slugs_statuses_and_exact_bodies` | ✅ | Numbers 1, 2 and 5 kept with their slugs and branches |
| FR-2 | `tests/tree.rs::reads_constitution_and_spec_documents_in_kind_order_with_exact_bytes`, `tests/store_transfer.rs::owner_imports_numbers_slugs_statuses_and_exact_bodies` | ✅ | BOM, CRLF and missing final newline kept |
| FR-3 | `src/tree.rs::reads_the_version_token`, `tests/store_transfer.rs::constitution_version_comes_from_the_option_or_is_required`, `tests/cli_transfer.rs::only_an_owner_imports_and_the_version_can_be_given` | ✅ | The option overrides the file's line |
| FR-4 | `tests/store_transfer.rs::constitution_version_comes_from_the_option_or_is_required` | ✅ | Error names `docs/constitution.md`; counts unchanged |
| FR-5 | `src/tree.rs::reads_the_status_word`, `tests/store_transfer.rs::owner_imports_numbers_slugs_statuses_and_exact_bodies`, `tests/cli_transfer.rs::import_prints_a_report_and_notices` | ✅ | `approved`, `archived` restored; missing line and missing `spec.md` fall back to draft with a notice |
| FR-6 | `tests/tree.rs::every_problem_is_collected`, `tests/tree.rs::an_empty_directory_is_a_problem_and_a_missing_one_an_error`, `tests/store_transfer.rs::conflicts_with_existing_data_are_all_listed_and_nothing_is_written`, `tests/store_transfer.rs::tree_problems_refuse_the_import`, `tests/cli_transfer.rs::refusal_lists_every_problem_and_writes_nothing` | ✅ | Invalid name, duplicate number and slug, non-UTF-8, size limit + 1, existing number, slug and constitution, empty tree |
| FR-7 | `tests/store_transfer.rs::a_failure_in_the_middle_leaves_the_store_unchanged` | ✅ | A trigger aborts the third revision; specs, documents, revisions and audit counts unchanged |
| FR-8 | `tests/tree.rs::unknown_entries_are_skipped_and_named`, `tests/cli_transfer.rs::import_prints_a_report_and_notices` | ✅ | Files in `specs/`, extra files and subfolders in a spec folder |
| FR-9 | `tests/store_transfer.rs::owner_imports_numbers_slugs_statuses_and_exact_bodies` | ✅ | Author, `import <path>` message, `spec.import`, `document.put`, `constitution.put` |
| FR-10 | `tests/store_transfer.rs::only_owners_import`, `tests/cli_transfer.rs::only_an_owner_imports_and_the_version_can_be_given` | ✅ | Editor, reader, non-owner admin and outsider refused |
| FR-11 | `tests/store_transfer.rs::dry_run_reports_without_writing`, `tests/cli_transfer.rs::dry_run_reports_and_writes_nothing` | ✅ | |
| FR-12 | `tests/cli_transfer.rs::import_prints_a_report_and_notices` | ✅ | Exact stdout compared |
| FR-13 | `tests/store_transfer.rs::next_spec_after_import_is_one_above_the_highest` | ✅ | After 001, 002, 005 the next spec is 006 |
| FR-14 | `tests/store_transfer.rs::export_returns_current_bodies_in_layout_order_for_readers_and_admins`, `tests/tree.rs::export_into_a_missing_directory_writes_every_file`, `tests/cli_transfer.rs::export_writes_lists_and_then_reports_unchanged` | ✅ | Latest revision exported after an edit; reader, owner and non-member admin |
| FR-15 | `tests/store_transfer.rs::export_of_one_spec_and_unknown_spec`, `tests/cli_transfer.rs::export_of_one_spec_with_a_status_notice` | ✅ | |
| FR-16 | `tests/tree.rs::unchanged_files_are_left_alone_and_different_ones_need_force`, `tests/cli_transfer.rs::differing_files_block_the_export_unless_forced` | ✅ | Every differing path listed; no file written |
| FR-17 | `tests/tree.rs::unchanged_files_are_left_alone_and_different_ones_need_force`, `tests/cli_transfer.rs::export_writes_lists_and_then_reports_unchanged`, `tests/cli_transfer.rs::differing_files_block_the_export_unless_forced` | ✅ | Modification time of unchanged files kept; unrelated files remain |
| FR-18 | `tests/store_transfer.rs::status_mismatch_produces_a_notice_and_keeps_the_bytes`, `tests/cli_transfer.rs::export_of_one_spec_with_a_status_notice` | ✅ | No notice when `spec.md` has no `Status:` line |
| FR-19 | `tests/store_transfer.rs::outsiders_cannot_export`, `tests/cli_transfer.rs::outsiders_cannot_export` | ✅ | Target folder not created |
| FR-20 | `tests/cli_transfer.rs::export_writes_lists_and_then_reports_unchanged`, `tests/store_transfer.rs::export_returns_current_bodies_in_layout_order_for_readers_and_admins` | ✅ | Revision and audit counts unchanged |
| FR-21 | `tests/cli_transfer.rs::this_repository_round_trips_byte_for_byte` | ✅ | Runs on this repository's own tree |
| NFR-1 | `tests/perf_import.rs::importing_one_hundred_specs_takes_under_ten_seconds` (release, ignored by default) | ✅ | 100 specs × 5 documents × 20 KB: 87 ms and 162 ms in two runs against a 10 s limit |
| NFR-2 | `tests/cli_transfer.rs` (`fails` helper checks exit 1, empty stdout, explanation on stderr) | ✅ | |
| NFR-3 | `src/tree.rs::rejects_folders_that_cannot_round_trip`, `tests/store_transfer.rs::export_paths_stay_inside_the_layout` | ✅ | `..`, `/` and uppercase rejected at import; exported paths are `docs/constitution.md` or `specs/<branch>/<kind>.md` |

## Edge cases

| Case | Test | Result |
|---|---|---|
| Empty `specs/` with a constitution | `tests/tree.rs::constitution_alone_and_spec_folder_without_documents_are_fine` | ✅ |
| Spec folder without documents | `tests/tree.rs::constitution_alone_and_spec_folder_without_documents_are_fine`, `tests/cli_transfer.rs::import_prints_a_report_and_notices` | ✅ |
| Names `0001-x`, `01-x`, `001-Name`, five-word slug | `src/tree.rs::rejects_folders_that_cannot_round_trip` | ✅ |
| BOM, CRLF, no final newline | `tests/tree.rs::reads_constitution_and_spec_documents_in_kind_order_with_exact_bytes`, `tests/store_transfer.rs::owner_imports_numbers_slugs_statuses_and_exact_bodies` | ✅ |
| File of exactly the size limit, and one byte more | `tests/tree.rs::every_problem_is_collected` | ✅ |
| Malformed task lines | `tests/store_transfer.rs::owner_imports_numbers_slugs_statuses_and_exact_bodies` (`not a task line` kept) | ✅ |
| Re-running the same import | Manual run below | ✅ |
| Export of an empty project | `tests/store_transfer.rs::empty_project_exports_nothing` | ✅ |
| Export into a missing directory | `tests/tree.rs::export_into_a_missing_directory_writes_every_file` | ✅ |
| Archived specs | `tests/store_transfer.rs::export_returns_current_bodies_in_layout_order_for_readers_and_admins` (spec 005) | ✅ |
| Symbolic links | no test | Not specially treated, as the spec states |

## Manual validation

| Step | Observed | Result |
|---|---|---|
| Release binary: new store, user `alice`, project `sdd`, `alice` owner | Commands succeed | ✅ |
| `import sdd <copy of docs/ and specs/> --dry-run` | Constitution v1.0 and specs 001, 002, 003 listed; `skipped specs/.gitkeep`; "dry run: would import 3 specs and 15 documents" | ✅ |
| `import` without `--dry-run` | Same report ending in "imported 3 specs and 15 documents" | ✅ |
| `export sdd <empty dir>` then `diff -r` against the source copy | 15 files written; the only difference is `specs/.gitkeep`, which import skips by design | ✅ |
| Second `export` into the same directory | "exported 15 files: 0 written, 15 unchanged" | ✅ |
| Same `import` again | Exit 1 with seven problems listed: existing constitution and the numbers and slugs of the three specs | ✅ |

## Verdict

The spec is fulfilled. Every FR and NFR has a green covering test, and the manual round trip of this repository reproduces every imported file byte for byte.

Observations, not defects:

- When a spec folder is refused for its name or for a duplicate number or slug, its files are not read, so encoding or size problems inside that folder show only after the name is fixed. The import is still refused as a whole, so nothing is lost.
- During T05 the local suite script reported a clippy failure as exit 0 because of a pipe. The unpushed T05 commit was amended with the fix, which added an `allow(dead_code)` to the shared test helper. The script was corrected before T06.
