# Research — Spec 003

Spec: `specs/003-import-export/spec.md` · Date: 2026-10-17

## R1 — Does a failed import leave the store unchanged?

- Question: FR-7 requires that an error after writing started leaves no trace. Is returning an error from `Store::with_tx` enough?
- Finding: `rusqlite::Transaction` is created with `drop_behavior: DropBehavior::Rollback` and its `Drop` implementation calls `finish_`, which rolls back when the transaction was not committed. `Store::with_tx` only calls `commit()` after the closure returns `Ok`, so any `Err` drops the transaction and rolls it back.
- Source: `~/.asdf/installs/rust/1.97.1/registry/src/index.crates.io-1949cf8c6b5b557f/rusqlite-0.40.2/src/transaction.rs` lines 128, 226 and 245; `src/store/mod.rs::with_tx`.
- Decision: the whole import runs inside one `with_tx` closure; the test forces a failure in the middle with a SQLite trigger that aborts on a marked body and checks that every table count is unchanged.

## R2 — Directory listing order

- Question: can the import rely on `std::fs::read_dir` order for reports and numbering?
- Finding: the documentation of `read_dir` says the order in which the iterator returns entries is platform and filesystem dependent.
- Source: `~/.asdf/installs/rust/1.97.1/toolchains/1.97.1-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/fs.rs` lines 209 and 3264 (`read_dir` and `ReadDir` docs).
- Decision: entries are collected and sorted by name before checks and reports, so output is deterministic.

## R3 — Reading files byte for byte

- Question: how to keep the exact bytes, including a byte order mark and CRLF, and detect non-UTF-8 files?
- Finding: `std::fs::read` returns the raw `Vec<u8>`; `String::from_utf8` accepts the vector without changing any byte (a UTF-8 BOM is valid UTF-8 and stays as U+FEFF) and fails on invalid sequences. rusqlite binds `&str` as SQLite TEXT and returns the same bytes, which the CRLF raw-view test of spec 002 already exercises.
- Source: `~/.asdf/installs/rust/1.97.1/toolchains/1.97.1-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/fs.rs` line 341 (`read`); Rust std documentation of `String::from_utf8` (https://doc.rust-lang.org/std/string/struct.String.html#method.from_utf8); `tests/ui_specs.rs` raw-view test.
- Decision: size is checked on the byte length before decoding, against `MAX_BODY_BYTES`; decoding uses `String::from_utf8`; no normalisation anywhere.

## R4 — Writing the export

- Question: how to create the layout and compare with existing files?
- Finding: `std::fs::create_dir_all` creates every missing parent and succeeds when the directory exists; `std::fs::write` creates or truncates the file; `std::fs::read` of an existing target gives the bytes to compare.
- Source: `~/.asdf/installs/rust/1.97.1/toolchains/1.97.1-aarch64-apple-darwin/lib/rustlib/src/rust/library/std/src/fs.rs` lines 420 (`write`) and 3076 (`create_dir_all`).
- Decision: export first reads every existing target and classifies it as new, unchanged or different; it refuses before writing anything when a different file exists without `--force`, then writes new and different files.

## R5 — Valid spec folder names

- Question: which folder names can round-trip?
- Finding: the store derives the branch, and therefore the export folder, as `format!("{number:03}-{slug}")` and validates slugs with `validate_slug`.
- Source: `src/store/specs.rs::branch_name`, `src/model.rs::validate_slug`.
- Decision: a folder is accepted only when its digits parse to a number of at least 1, the rest passes `validate_slug`, and `branch_name(number, slug)` equals the folder name; this also guarantees NFR-3 because no stored name can contain a separator or `..`.

## R6 — New dependencies

- Question: is any crate needed?
- Finding: directory walking is one level deep (`specs/*/`) and the line parsing is a prefix match, both covered by the standard library.
- Decision: no new dependency; `walkdir` and `regex` were considered and rejected as unnecessary.
