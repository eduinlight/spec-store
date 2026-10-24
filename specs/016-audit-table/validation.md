# Validation — Spec 016

Spec: `specs/016-audit-table/spec.md` · Date: 2026-10-24 · Ticket: SV-10

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | Core `store_audit_table.rs` (text over target, client and origin; actor, client, action, result and period filters and combinations; facet counts with the other filters applied); API `http_audit_table.rs`; app audit tests; e2e filters Result = Denied | pass |
| FR-2 | Core sort and paging with totals; app rows per page, page X of Y, first/previous/next/last | pass |
| FR-3 | App table: time, actor, via, action, target, result pills, origin; column toggles; row and page selection; footer counts; e2e hides a column and selects a row | pass |
| FR-4 | Core export by ids or by query, recorded as `audit.export`; CSV header and RFC 4180 quoting; formula-like cells (`= + - @`, tab, carriage return) kept as text (unit tests); e2e downloads the selected row | pass |
| FR-5 | Core and API: project owners, organization owners, operators; others refused | pass |
| FR-6 | English and Spanish keys | pass |
| NFR-1 | `perf_audit_table` (release): 100,000 entries, page with facets p50 26 ms, p95 101 ms (threshold 500 ms) at load average 15–48 | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint`, `make test` | pass (83 Rust test binaries, UI 337, landing 24, app 295) except `perf_read`: 257–433 ms on this branch and 321–642 ms on main's code side by side under the same machine load (average 33–41) after a Docker restart on this machine; no regression; main's CI runs it on a clean runner |
| Playwright | 18/18 |

## Notes

- Column visibility is kept per page view; filters, sort, page and page size live in the address.
- The organization view adds a Project column.
- The design system's `TriUp`/`TriDown` glyphs appear swapped (newest first shows ▲); left for a design-system fix.

## Verdict

Local: pass. Staging: pending the user's check.
