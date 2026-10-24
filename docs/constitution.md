# Constitution — SDD Vault

Non-negotiable principles. They prevail over any other practice. Changing them requires
explicit approval and recording the version.

1. **The spec rules.** No implementation without an approved spec; when a decision is not
   covered, work stops and the question is asked.
2. **Stack and limits.** Rust stable 1.97, edition 2024, with `rmcp`, `axum`, `tokio` and
   `diesel` (PostgreSQL backend) for `packages/core` and `apps/*` Rust deliverables; TypeScript with
   React 19, React Router 7 (`apps/landing`), Vite (`apps/app`), Tailwind CSS 4, the TapTap design
   system ported to `packages/ui`, dayjs and react-i18next; PostgreSQL 18 and Keycloak 26 as the
   systems of record, Redis 8 for state that may be lost (presence, claims, rate limits, live
   fan-out), RabbitMQ 4 for durable messages fed from a transactional outbox, and S3-compatible
   object storage (Garage) for files; business rules live only in `packages/core`; any other
   dependency or deliverable is justified in the plan with the rejected alternative.
3. **Tests as a gate.** Tests are written before the implementation, and no task is done
   until `make lint`, `make fmt-check` and `make test` pass for every package and app.
4. **Separation of concerns.** Business rules depend on ports (traits) for persistence,
   messaging, storage, identity and time, never on a concrete adapter; adapters live at the edge
   of the deliverables. Each design pattern and abstraction layer is justified in writing in the
   plan with the simpler alternative it replaces.
5. **Markdown is the source of truth.** Documents are stored as markdown; index tables are
   always derived from them and never edited on their own; templates stay human-readable.
6. **Nothing is lost.** PostgreSQL is the only system of record of the platform (identities
   live in Keycloak's own database; Redis and RabbitMQ hold only data that can be rebuilt from it;
   files live in object storage and are referenced from it), changed only through numbered migrations; every
   document write is an append-only revision with an audit entry and optimistic locking; no in-place
   update or hard delete of documents.
7. **Access is scoped.** Every call carries a short-lived credential issued by Keycloak and is
   authorized against the caller's organization and project memberships, roles and permissions and
   the organization plan's entitlements; readers never write.
8. **Reference sources.** Technical decisions are grounded in the official documentation or
   source of `rmcp`, `axum`, `diesel`, the MCP specification (including authorization),
   PostgreSQL, Keycloak, Redis, RabbitMQ, the S3 API, React Router, Vite, Tailwind CSS and the TapTap
   design system, cited in `research.md`.
9. **Language.** Everything is in English: code, identifiers, commit messages, documentation,
   SDD artifacts and all communication with the user, even when the user writes in another language.
10. **Human approval.** Commits, merges, constitution changes and any operation that deletes
    data require the user's explicit OK, one authorization per action, unless the user grants a
    standing authorization for a named set of tickets; it is recorded in those tickets.

Version 3.0 · Ratified 2026-10-17 · Last amended 2026-10-24 (3.0: product renamed SDD Vault; Redis,
RabbitMQ and S3-compatible storage admitted; TapTap design system replaces shadcn/ui; ports and
adapters replace the no-layers rule; entitlements join the access rule; standing authorizations,
granted by the owner on 2026-10-24 for SV-1 to SV-21. 2.0: SaaS foundation of spec 005)
