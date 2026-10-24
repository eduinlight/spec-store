# Tasks — Spec 010

Plan: `specs/010-agent-attribution/plan.md` · Date: 2026-10-24

- [x] T01 (FR-1, FR-6) Migration 0002, schema, `Via`, `AuditResult`; audit and revision writers record the caller's client and origin; readers return the new fields — Done when: core tests for attribution, migration list and schema green
- [x] T02 (FR-3, FR-4, FR-5) `Store::audited` decorator and refusal recording, applied to every write; existing tests that count audit rows updated — Done when: core refusal tests green
- [x] T03 (FR-1, FR-2) API: origin layer with `TRUSTED_PROXY_HOPS`, client names for bearer and session callers — Done when: API tests green
- [x] T04 (FR-1, FR-7) MCP: client name from per-request info, `ClientNameResolver` with TTL cache, fallback — Done when: MCP tests green
- [x] T05 (FR-6) API and MCP output fields, `make openapi`, app history and audit columns — Done when: API, MCP and app tests green
- [x] T06 Gate, staging overlays set `TRUSTED_PROXY_HOPS`, `validation.md` — Done when: gate green
