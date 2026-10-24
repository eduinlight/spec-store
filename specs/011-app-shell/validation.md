# Validation — Spec 011

Spec: `specs/011-app-shell/spec.md` · Date: 2026-10-24 · Ticket: SV-5

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | `apps/app/src/shell/Header.test.tsx`: teal header, organization and project switchers (search, current check, Create organization, All projects, New project), palette, shortcuts and notifications triggers; `e2e/shell.spec.ts` screenshots | pass |
| FR-2 | `Sidebar.test.tsx`: project-mode items and sections; placeholder screens for Active specs, Specs, Activity, Members, Tokens, Preferences, Security, Billing | pass |
| FR-3 | `Sidebar.test.tsx`, `e2e/shell.spec.ts`: account mode with "Back to {project}" returning to the remembered page | pass |
| FR-4 | `UserMenu.test.tsx`: avatar, name, handle; account and organization items; language; sign out; `e2e/sign-in.spec.ts` signs out through the menu | pass |
| FR-5 | `CommandPalette.test.tsx`: ⌘K and Ctrl+K, trigger, groups, filter, ↑/↓/↵/Esc | pass |
| FR-6 | `useShortcuts.test.tsx`, `ShortcutsDialog.test.tsx`: `?`, `n`, `Esc`, g-chords; ignored in text fields and while a dialog is open | pass |
| FR-7 | `confirm.test.tsx`; member removal, leaving an organization, invitation revoke and agent-access revoke ask first; toasts after role change, removal, invitation send/resend/revoke, access revoke, language change | pass |
| FR-8 | `navigation.test.ts`: switching organization or project keeps the same kind of screen when it exists | pass |
| FR-9 | Every shell text in `i18n/en.ts` and `i18n/es.ts` | pass |
| NFR-2 | `e2e/shell.spec.ts` at 390 px: sidebar in a drawer, no horizontal scroll | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint`, `make test` | pass (app 102 tests, perf 65 ms) |
| Playwright | 12/12 |

## Deviations

- Project switcher: the project list API has no active spec per project, so only the current project shows it (SV-6 adds summaries).
- Project Members goes to a placeholder until SV-9; members are managed in project settings meanwhile.
- Organization-level pages get a small sidebar (Projects, Members, Invitations); the spec did not define it.
- Below 640 px the organization switcher moves into the drawer.

## Verdict

Local: pass. Staging: pending the user's check.
