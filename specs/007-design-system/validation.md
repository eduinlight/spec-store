# Validation — Spec 007

Spec: `specs/007-design-system/spec.md` · Date: 2026-10-24 · Ticket: SV-1

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | `packages/ui/src/styles/tokens.test.ts`: handoff colour, type, radius, layout and elevation tokens with their values; semantic aliases (`--primary`, `--background`, `--foreground`, `--border`, `--destructive`, `--radius`) resolve to TapTap tokens; system sans and Consolas stacks | pass |
| FR-2 | App and landing import `@workspace/ui/globals.css` (tokens included); the Keycloak image copies `tokens.css` (`deploy/keycloak/Dockerfile`); browser check: landing, sign-in page and signed-in app render with the teal primary, the grey page background and the system font; e2e 8/8 | pass |
| FR-3 | One test file per component in `packages/ui/src/taptap/*.test.tsx` covering sizes, variants, states and the value/onChange contracts | pass |
| FR-4 | `Select.test.tsx`: single trigger keeps `0 12px` (large, medium) and `0 8px` (small) with an 8 px gap; `Tooltip.test.tsx`: bubble wraps with `white-space: normal`, `max-width: 260px` | pass |
| FR-5 | `foundation.test.tsx`: lock document glyph, 1.7 stroke, `currentColor` | pass |
| FR-6 | `packages/ui/src/patterns/*.test.tsx` (71 tests): combobox search, pick, keyboard, empty text and actions; faceted filter counts, selected tags and clear; data table selection, sort both ways, column hiding, page size and clamping; spec card name, live and markers pills; sidebar item selected state; kbd tones | pass |
| FR-7 | Keyboard and ARIA cases in every component and pattern test (focus ring, Enter/Space activation, Escape closes overlays, arrow keys in Select, Tabs, Combobox; dialog focus trap and return; `aria-pressed`, `aria-checked`, `aria-selected`, `aria-current`, `aria-sort`) | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check` | pass |
| `make lint` | pass |
| `make test` | pass (UI package 336 tests, app 27, landing 3, every Rust suite) |
| `make e2e` | Playwright 8/8 with the new tokens; Lighthouse landing performance 0.98, accessibility 0.92 on `/` and `/es/` |

## Notes

- Known issue for the owner: white text on the TapTap primary (`#15C5CE`) has a contrast ratio near 2:1, below WCAG AA (4.5:1); Lighthouse's colour-contrast audit fails on the landing's primary buttons although the category score stays above 0.9. It comes from the design system's brand colour and is raised on SV-1 and SV-2.

- The handoff defines light mode only; dark mode stays out of scope.
- Existing screens keep the shadcn components and adopt the TapTap look through the semantic aliases; each screen ticket moves its screens to the TapTap components.

## Verdict

Local: pass. Staging: pending the user's check that the landing, sign-in and app pages show the TapTap colours.
