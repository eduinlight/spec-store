# Technical plan — Spec 016

Spec: `specs/016-audit-table/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

Core gains an audit query model: `AuditQuery { scope: Project | Organization, text, actors, clients, actions,
results, period, sort, page, page_size }` built into one Diesel query with dynamic filters (boxed query), a
`count(*)` for the total, and facet counts (one grouped query per facet with every other filter applied).
CSV export streams the same query without paging (or by selected ids) and records an `audit.export` entry.
Indexes in migration 0005 support the filters. The API exposes `GET .../audit/table` and
`GET .../audit/export.csv` for projects and organizations; the app replaces the audit pages with the design's
data table built from `@workspace/ui/patterns` DataTable and FacetedFilter.

## 2. Decisions

- **Server-side filtering, sorting, paging and facet counts**; rejected: client-side over a capped download
  (wrong counts once the trail grows).
- **Facet counts with the other filters applied** (the shadcn faceted-filter convention), each facet's own
  selection ignored for its counts.
- **CSV** (RFC 4180, UTF-8 with header) for export; the export is itself audited.
- Migration 0005: indexes `(project_id, created_at DESC)`, `(organization_id, created_at DESC)` and
  `(project_id, action)`.

## 3. Test strategy

Core: each filter, combinations, facet counts, sort, paging totals, export rows and its audit entry, access
rules. API: JSON and CSV endpoints, authorization. Perf: 100,000 entries, facet page under 500 ms (release
test). App: table interactions (text filter, facets, reset, column toggles, selection, rows per page,
pagination, export). Playwright: filter by result Denied after a refused write, export the selection.
