# Validation — Spec 012

Spec: `specs/012-org-home-new-project/spec.md` · Date: 2026-10-24 · Ticket: SV-6

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | `packages/core/tests/store_summaries.rs` (9 tests: role, constitution, active spec with tasks, markers, last activity with client, visibility); `apps/api/tests/http_projects.rs` list fields | pass |
| FR-2 | App tests for the cards page (filters My projects / All / Archived, empty states, role tag, constitution line, active spec block, last activity, markers); `e2e/org-home.spec.ts` cards and Archived tab | pass |
| FR-3 | App tests for the dialog (slug from name, editable, `<org>/<slug>` preview, start options, errors kept); e2e creates a project | pass |
| FR-4 | `apps/app/src/lib/sdd-tree.ts` tests (CLI scan rules); dialog dry-run problems test; `apps/api/tests/http_readonly.rs` browser session may import; e2e chooses a folder, sees "specs: 1, documents: 3", imports and opens the project with spec 001 | pass |
| FR-5 | `packages/core/tests/store_archive.rs` copy tests (constitution with version, specs, documents as new revisions, audit `project.copy`); API copy tests; e2e copies a project | pass |
| FR-6 | Get started checklist tests (1 of 4 done after creation; first agent call from `last_activity.client`; spec 001); the connect command uses `mcp_url` from `/api/v1/me` (API test); e2e | pass |
| FR-7 | `store_archive.rs`: every write to an archived project is refused and recorded as denied, reads keep working, restore; MCP test (`mcp_attribution.rs`); e2e archives through settings with confirmation | pass |
| FR-8 | English and Spanish keys for every new text | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint`, `make test` | pass except `perf_read`, which failed on every run while this machine's load average was between 20 and 800; measured side by side under the same load, main gave 133–136 ms and this branch 89–149 ms, so the branch adds no cost; main's CI runs the check on a clean runner |
| Playwright | 13/13 |

## Decisions taken during implementation

- Spec 005 kept import and export out of the browser; spec 012 FR-4 asks for browser import, so browser sessions may now import (dry run first). Export from the browser stays refused until SV-17.
- The app learns the MCP address from `/api/v1/me` (`mcp_url`, from `MCP_RESOURCE`) instead of a build-time variable, so one app image serves staging and production.

## Verdict

Local: pass. Staging: pending the user's check.
