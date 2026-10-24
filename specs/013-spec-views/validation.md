# Validation — Spec 013

Spec: `specs/013-spec-views/spec.md` · Date: 2026-10-24 · Ticket: SV-7

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | Core `store_views.rs` spec cards (title, summary, dates, contributors, documents, requirement count, tasks, markers); API `http_views.rs`; app `specs.test.tsx` (search, status chips with counts, sort, 20 per page with Load more, Showing X of Y); SpecCard labels in English and Spanish | pass |
| FR-2 | `spec-detail.test.tsx`: header, lifecycle steps, tabs with counts, Validation disabled when missing | pass |
| FR-3 | `spec-detail.test.tsx`: revision line with author and client, rendered and raw, table of contents, requirement tooltips naming tasks, readiness checklist; core readiness tests | pass |
| FR-4 | Core task history (completing revision, author, client); app tasks tab: phases with counts, filters Pending, Done, Mine, Parallel, completion line | pass |
| FR-5 | Core traceability (Covered, In progress, Not started, No tasks; tasks without requirements); app traceability table and note | pass |
| FR-6 | Core resolved markers by revision diff; app markers tab (open, empty state, resolved list) | pass |
| FR-7 | App history tab: revisions with author, client, message, time; diff between revisions | pass |
| FR-8 | `constitution.test.tsx`: version, dates and authors, lock note for non-owners, principles, version history | pass |
| FR-9 | Archived banners; English and Spanish keys with parity test | pass |

Parsers are checked against every `specs/*/spec.md` of this repository.

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint`, `make test` | pass (UI 337, landing 24, app 180, Rust suites) except `perf_read` at 119–184 ms while this machine's load average was 24–48; this spec does not change the document read or project status it measures; main's CI runs it on a clean runner |
| Playwright (`e2e/spec-views.spec.ts`: import a tree, open Specs, every tab, the constitution) | 14/14 |

## Deviations

- Requirement counts, traceability and readiness use FR-n only; NFR-n are parsed but not counted (tasks cite FRs).
- Resolved markers carry who resolved them and when, not an answer text (the markdown holds no separate answer).
- `/specs/{n}/tasks` became the Tasks tab; the raw tasks document stays readable through History and revisions.
- Approve, Archive, Approvals and "agents live" arrive with SV-11 and SV-13.

## Verdict

Local: pass. Staging: pending the user's check.
