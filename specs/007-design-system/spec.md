# Spec 007 — TapTap design system in the UI package

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-1

## 1. Context and goal

The app, the landing page and the Keycloak theme use the neutral shadcn styling of spec 005. The SDD Vault
design handoff (2026-10-24) builds every screen from the TapTap design system: its tokens (teal brand, neutral
ramps, seven type styles, four elevations), about twenty-five of its components, and a few patterns made of
them. Every later screen ticket (SV-2 to SV-20) needs these pieces, so they are built once, typed and tested,
in the shared UI package, and every deliverable reads the same tokens.

## 2. Users / actors

- Front-end developer of a later ticket: composes screens from typed components and tokens.
- Visitor and signed-in person: see the TapTap look (colours, type, radii) on every page.

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| `packages/ui/src/styles/tokens.css` (shadcn oklch tokens, Inter font) | Neutral styling of spec 005 | replace | Same semantic names now resolve to TapTap values, so existing screens change look without code changes |
| shadcn `button`, `card`, `input`, `label` | Used by spec 005 screens | keep | Each screen ticket migrates its screens to the TapTap components |
| Keycloak `tokens.css` copy | Theme colours | replace | Generated from the same TapTap tokens |

## 4. User stories (by priority)

### US-1 (P1) — One set of tokens
As a developer I want the TapTap tokens available everywhere, so every deliverable looks the same.
Independent test: the token files carry the handoff's values and the app, the landing page and the Keycloak theme load them.

### US-2 (P1) — Typed components
As a developer I want the TapTap components the designs use, typed and tested, so I build screens without re-implementing them.
Independent test: each component renders its states in a unit test.

### US-3 (P1) — Patterns
As a developer I want the patterns of the designs (combobox, faceted filter, data table, spec card, sidebar item, kbd chip) ready to use.
Independent test: each pattern's behaviour (search, select, sort, paginate, select rows) is covered by a unit test.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL define in `packages/ui` every token of the handoff's `tokens/colors.css`, `typography.css` and `layout.css` with the same names and values, and SHALL resolve the semantic names used by existing screens (`--primary`, `--background`, `--foreground`, `--muted`, `--border`, `--destructive`, `--radius`, `--font-sans`, `--font-mono`) to TapTap tokens.
- **FR-2** (US-1): THE SYSTEM SHALL load those tokens in the app, the landing page and the Keycloak login and email themes.
- **FR-3** (US-2): THE SYSTEM SHALL provide Button, IconButton, Input, Select, Checkbox, CheckableTag, Toggle, Tabs, Steps, Breadcrumb, Pagination, Tag, StatusTag, Avatar, AvatarGroup, Tooltip, Table, Alert, Message (toast), Dialog, Drawer, BarChart, ChartLegend and Icon with the props, sizes, variants and states of the handoff's components.
- **FR-4** (US-2): WHEN a single-value Select is rendered, THE SYSTEM SHALL keep its horizontal padding (12 px, 8 px small); WHEN a Tooltip title is longer than its width, THE SYSTEM SHALL wrap it within 260 px.
- **FR-5** (US-2): THE SYSTEM SHALL provide a "document with padlock" glyph (1.7 px stroke, `currentColor`) for locked items.
- **FR-6** (US-3): THE SYSTEM SHALL provide a Combobox (searchable list, check on the current item, footer actions), a FacetedFilter (dashed trigger, searchable checkbox list with counts, selected tags, Clear filters), a DataTable (row and page selection, column visibility, sortable column, rows per page, page X of Y), a SpecCard, a SidebarItem (3 px primary indicator when selected) and a Kbd chip.
- **FR-7** (US-2, US-3): WHERE a component is interactive, THE SYSTEM SHALL operate it with the keyboard (Tab focus, Enter/Space to activate, Escape to close overlays, arrow keys in lists) and expose its role and state to assistive technology.

## 6. Non-functional requirements

- **NFR-1**: No runtime patch file ships; the two bugs of `ds-fixes.js` are fixed in the components.
- **NFR-2**: The landing page keeps Lighthouse ≥ 90 for performance and accessibility on mobile.

## 7. Key entities

- Token, component, pattern.

## 8. Edge cases

- Empty lists in Combobox, FacetedFilter and DataTable show their empty text.
- A DataTable page beyond the last after a filter returns to the last page.
- A disabled component ignores pointer and keyboard activation.

## 9. Out of scope

- Dark mode (the handoff defines light only; noted in SV-1).
- Migrating existing screens to the new components (each screen ticket does it).
- Components the designs do not use (Cascader, DatePicker, Transfer, Upload, Slider, Result, Popconfirm, AreaChart, LineChart, DonutChart, kit aliases).

## 10. Completion criteria

- Every FR covered by at least one green test.
- Landing, app and Keycloak pages render with TapTap tokens (checked in the browser).

## 11. Assumptions

- Components are ported to TypeScript from the handoff's bundle with inline styles on CSS variables, as in the source.
