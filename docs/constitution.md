# Constitution — spec-store

Non-negotiable principles. They prevail over any other practice. Changing them requires
explicit approval and recording the version.

1. **The spec rules.** No implementation without an approved spec; when a decision is not
   covered, work stops and the question is asked.
2. **Stack and limits.** Rust stable 1.97, edition 2024, with `rmcp`, `axum`, `tokio` and
   `diesel` (PostgreSQL backend) for `packages/core` and `apps/{api,mcp,cli}`; TypeScript with
   React 19, React Router 7 (`apps/landing`), Vite (`apps/app`), Tailwind CSS 4, shadcn/ui, dayjs
   and react-i18next; PostgreSQL 18 and Keycloak 26 as the only services; business rules live
   only in `packages/core`; any other dependency or deliverable is justified in the plan with
   the rejected alternative.
3. **Tests as a gate.** Tests are written before the implementation, and no task is done
   until `make lint`, `make fmt-check` and `make test` pass for every package and app.
4. **Simplicity.** Frameworks are used directly; every abstraction layer is justified in
   writing in the plan.
5. **Markdown is the source of truth.** Documents are stored as markdown; index tables are
   always derived from them and never edited on their own; templates stay human-readable.
6. **Nothing is lost.** PostgreSQL is the only persistent state of the platform (identities
   live in Keycloak's own database), changed only through numbered migrations; every document
   write is an append-only revision with an audit entry and optimistic locking; no in-place
   update or hard delete of documents.
7. **Access is scoped.** Every call carries a short-lived credential issued by Keycloak and is
   authorized against the caller's organization and project memberships; readers never write.
8. **Reference sources.** Technical decisions are grounded in the official documentation or
   source of `rmcp`, `axum`, `diesel`, the MCP specification (including authorization),
   PostgreSQL, Keycloak, React Router, Vite, Tailwind CSS and shadcn/ui, cited in `research.md`.
9. **Language.** Everything is in English: code, identifiers, commit messages, documentation,
   SDD artifacts and all communication with the user, even when the user writes in another language.
10. **Human approval.** Commits, merges, constitution changes and any operation that deletes
    data require the user's explicit OK, one authorization per action.

Version 2.0 · Ratified 2026-10-17 · Last amended 2026-10-22 (2.0: SaaS foundation of spec 005 —
PostgreSQL replaces SQLite, monorepo deliverables replace the single binary, Keycloak credentials
replace bearer tokens, reference sources extended; P3 gate runs through make)
