# Constitution — spec-store

Non-negotiable principles. They prevail over any other practice. Changing them requires
explicit approval and recording the version.

1. **The spec rules.** No implementation without an approved spec; when a decision is not
   covered, work stops and the question is asked.
2. **Stack and limits.** Rust stable 1.97, edition 2024; `rmcp`, `axum`, `tokio` and
   `rusqlite` are the base; any other dependency is justified in the plan with the rejected
   alternative; no runtime other than the single `spec-store` binary.
3. **Tests as a gate.** Tests are written before the implementation, and no task is done
   until `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.
4. **Simplicity.** Frameworks are used directly; every abstraction layer is justified in
   writing in the plan.
5. **Markdown is the source of truth.** Documents are stored as markdown; index tables are
   always derived from them and never edited on their own; templates stay human-readable.
6. **Nothing is lost.** SQLite in one file is the only persistent state, changed only through
   numbered migrations; every document write is an append-only revision with an audit entry
   and optimistic locking; no in-place update or hard delete of documents.
7. **Access is scoped.** Every call is authenticated with a per-user bearer token and
   authorized against the project's memberships; readers never write.
8. **Reference sources.** Technical decisions are grounded in the official documentation or
   source of `rmcp`, `axum`, `rusqlite` and the MCP specification, cited in `research.md`.
9. **Language.** Everything is in English: code, identifiers, commit messages, documentation,
   SDD artifacts and all communication with the user, even when the user writes in another language.
10. **Human approval.** Commits, merges, constitution changes and any operation that deletes
    data require the user's explicit OK, one authorization per action.

Version 1.0 · Ratified 2026-10-17 · Last amended 2026-10-17
