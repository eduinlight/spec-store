# Validation — Spec 014

Spec: `specs/014-overview-activity-live/spec.md` · Date: 2026-10-24 · Ticket: SV-8

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | Core `store_dashboard.rs` (active specs, tasks completed and previous period, coverage, open markers, agents active in 15 minutes); API `http_dashboard.rs`; app Overview tests (period 7/14/30 in the address, KPI tiles with tooltips) | pass |
| FR-2 | Core completed-per-day by client; app stacked bar chart | pass |
| FR-3 | Core attention items (approval waiting for owners, open questions, requirements without tasks); app list with actions to the spec, markers and traceability tabs | pass |
| FR-4 | Core active view (next task, agent prompt, drafts with markers); app Active specs (Copy agent prompt to clipboard with toast, drafts); e2e copies the prompt | pass |
| FR-5 | Core activity (filters, conflict resolved by a later save); app Activity with sentences per action, result pills, Load more | pass |
| FR-6 | Core: events published after commit for writes and refusals, not on rollback; API: write reaches a subscribed socket within 2 s, refusal too, MCP write through Redis reaches a socket of another instance, two hubs on one Redis both deliver; e2e: a write appears on the open Activity page without reloading | pass |
| FR-7 | API: unauthenticated upgrade refused, foreign project subscription refused, session end closes with 1008, lost access drops the subscription; resync on hub reconnect (`http_live_resync.rs`, Redis `CLIENT KILL`); app: reconnect with backoff, resubscribe, refetch on resync and on subscription confirmation | pass |
| FR-8 | English and Spanish keys | pass |
| NFR-2 | `an_unreachable_redis_never_fails_a_write`; publishing through a bounded queue on a background task | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint`, `make test` | pass (Rust 390+, UI 337, landing 24, app 258); `perf_read` 50 ms and 71 ms once the machine load dropped (106 ms under load) |
| Playwright | 16/16 |

## Defects found and fixed

- The phone drawer listed every organization above the navigation without a height limit; with many organizations the navigation could not be reached. The list is now capped at 40% of the screen and scrolls (found by `e2e/shell.spec.ts` once the test account had 24 organizations).

## Decisions

- Redis pub/sub carries notifications only; pages refetch through the authorized API.
- A confirmed subscription and a hub reconnection both trigger one refetch, so no write is missed between page load and subscription or during a bus outage.
- "Requirements without work" lists requirements with no task; requirements whose tasks are pending show in the coverage KPI and the traceability tab.

## Verdict

Local: pass. Staging: pending the user's check of live updates across the two API replicas of production (staging runs one).
