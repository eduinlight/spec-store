# Technical plan — Spec 007

Spec: `specs/007-design-system/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

The handoff's token files are copied into `packages/ui/src/styles/taptap/`, and `tokens.css` maps the
semantic names used by existing screens onto them. The components the designs use are ported from the
handoff's bundle (`_ds_bundle.js`, one section per source file) to TypeScript React components under
`packages/ui/src/taptap/`, one file per component family, keeping the source's inline styles on CSS variables.
Patterns are composed from those components under `packages/ui/src/patterns/`. Everything is exported from
`@workspace/ui/taptap` and `@workspace/ui/patterns`. The package gains its own Vitest suite.

## 2. Technical context

- React 19, TypeScript 6, Vitest 5 + Testing Library + jsdom (same versions as `apps/app`).
- No new runtime dependency.
- The Keycloak theme's `tokens.css` is produced from the same files by `deploy/keycloak/Dockerfile`'s copy step.

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 2 | yes | TapTap design system admitted by v3.0; no new dependency |
| 3 | yes | Tests first for every component and pattern |
| 4 | yes | One abstraction: `useInteraction` (hover/press/focus state machine) shared by every component, as in the source; justified because twenty components need the same state rules |
| 9 | yes | English identifiers |

## 4. Module / file structure

```
packages/ui/src/styles/taptap/{colors,typography,layout,base}.css   tokens copied from the handoff
packages/ui/src/styles/tokens.css        semantic aliases → TapTap tokens
packages/ui/src/taptap/
  interaction.ts      useInteraction(disabled) → [state, handlers] with focus
  field.ts            fieldShell, inputReset, Chevron (from fieldStyle.js)
  icon-data.ts        glyph table (from icon-data.js)
  Icon.tsx, LockDocument.tsx
  Button.tsx (Button, IconButton), Input.tsx, Select.tsx, Checkbox.tsx, Toggle.tsx,
  Tag.tsx (Tag, CheckableTag, StatusTag), Avatar.tsx (Avatar, AvatarGroup), Tooltip.tsx (Floating, Tooltip),
  Tabs.tsx, Steps.tsx, Breadcrumb.tsx, Pagination.tsx, Table.tsx, Alert.tsx, Message.tsx (toast API),
  Dialog.tsx, Drawer.tsx, BarChart.tsx (BarChart, ChartLegend, CHART_COLORS)
  index.ts
packages/ui/src/patterns/
  Combobox.tsx, FacetedFilter.tsx, DataTable.tsx, SpecCard.tsx, SidebarItem.tsx, Kbd.tsx, index.ts
packages/ui/vitest.config.ts, src/test/setup.ts
```

## 5. Data

None.

## 6. Contracts

Each component's props follow the handoff source (names, defaults, variants: `size` large/medium/small;
Button `variant` primary/outline/ghost/link/danger…). Overlays (Dialog, Drawer) render in a portal, trap focus
and close on Escape. `message.success|error|info|warning(text)` shows a toast through a `MessageHost` mounted once.

## 7. Decisions

- **Port to TypeScript with inline styles** (as the source). Rejected: loading `_ds_bundle.js` as a global
  script (no types, no tree-shaking, needs `window.React`); rewriting in Tailwind classes (risk of drifting
  from the handoff and more work for no visual gain).
- **Keep shadcn components alongside** until each screen moves. Rejected: deleting them now (every existing
  screen would have to move in this ticket).
- **Semantic aliases** (`--primary: var(--primary-600)` …) so the existing Tailwind classes adopt the TapTap
  look at once.

## 8. Security

Icon glyph markup is static data from the handoff, injected only from the bundled table.

## 9. Test strategy

Vitest per component (render each variant and state, keyboard operation, ARIA role/state) and per pattern
(search, select, sort, paginate, selection); token test comparing the CSS files with the handoff values;
existing app and landing suites and e2e stay green; Lighthouse on the landing page.
