# Tasks — Spec 007

Plan: `specs/007-design-system/plan.md` · Date: 2026-10-24

## Phase 1 — Foundation
- [x] T01 (FR-1, FR-2) Vitest setup in `packages/ui`; token test; copy TapTap tokens, alias the semantic names, load them in app, landing and Keycloak — Done when: token test green and existing suites green
- [x] T02 (FR-3, FR-5, FR-7) `interaction.ts`, `field.ts`, `icon-data.ts`, `Icon`, `LockDocument` with tests — Done when: tests green

## Phase 2 — Components (parallel, disjoint files)
- [x] T03 [P] (FR-3, FR-7) Button, IconButton, Tag, CheckableTag, StatusTag, Avatar, AvatarGroup with tests — Done when: tests green
- [x] T04 [P] (FR-3, FR-4, FR-7) Input, Select, Checkbox, Toggle with tests (Select keeps padding) — Done when: tests green
- [x] T05 [P] (FR-3, FR-4, FR-7) Floating, Tooltip (wraps at 260 px), Tabs, Steps, Breadcrumb, Pagination with tests — Done when: tests green
- [x] T06 [P] (FR-3, FR-7) Table, Alert, Message, Dialog, Drawer, BarChart, ChartLegend with tests — Done when: tests green

## Phase 3 — Patterns
- [ ] T07 (FR-6, FR-7) Combobox, FacetedFilter, DataTable, SpecCard, SidebarItem, Kbd with tests — Done when: tests green
- [ ] T08 (FR-1 to FR-7) Exports, gate (`make fmt-check lint test e2e`), browser check of app, landing and Keycloak, `validation.md` — Done when: gate green
