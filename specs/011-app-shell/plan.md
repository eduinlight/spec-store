# Technical plan — Spec 011

Spec: `specs/011-app-shell/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

`routes/root.tsx` becomes the shell: header, sidebar (project or account mode), user menu, command palette,
shortcuts dialog, `MessageHost` and a confirmation dialog provider. It is built from `@workspace/ui/taptap`
(Button, IconButton, Icon, Avatar, Dialog, Drawer, Tooltip, message) and `@workspace/ui/patterns`
(Combobox, SidebarItem, Kbd). Shell state lives in small hooks under `apps/app/src/shell/`. Existing pages
render inside the shell's content area unchanged; routes that do not exist yet (Active specs, Specs list,
Activity, Tokens, Preferences, Security, Billing) get placeholder pages with an empty state.

## 2. Technical context

React 19, React Router 7 data mode, react-i18next, Vitest + Testing Library. No new dependency.

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 2 | yes | TapTap components and patterns only |
| 3 | yes | Tests first for shell, palette, shortcuts, confirm |
| 4 | yes | `useShortcuts` (one keyboard listener with a `g` chord state) and `ConfirmProvider` (one dialog for every destructive action) justified because several screens share them |

## 4. Files

```
apps/app/src/shell/
  Shell.tsx            header + sidebar + content + user menu layout (drawer sidebar under 880 px)
  Header.tsx           logo, OrganizationSwitcher, ProjectSwitcher, palette trigger, shortcuts, notifications
  Sidebar.tsx          project mode / account mode, back link with remembered origin
  UserMenu.tsx         avatar, name, handle; account and organization items, language, sign out
  CommandPalette.tsx   actions, specs (from the current project's spec list), navigation; ↑/↓/↵/Esc
  ShortcutsDialog.tsx  list of shortcuts with Kbd
  useShortcuts.ts      ?, n, Esc, g-chords; ignores inputs, textareas, selects, contenteditable
  confirm.tsx          ConfirmProvider + useConfirm()
  navigation.ts        route builders and "same kind of screen" mapping for switches
apps/app/src/routes/placeholder.tsx   empty states for upcoming screens
apps/app/src/routes/root.tsx         loader adds projects of the current organization; renders Shell
apps/app/src/router.tsx              new routes (placeholders) under the shell
apps/app/src/i18n/{en,es}.ts         shell texts
```

## 5. Data

Root loader: `GET /api/v1/orgs` (existing) and, when an organization is in the URL, its projects
(`GET /api/v1/orgs/{org}/projects`, existing) with role and active spec; the project switcher shows them.

## 6. Decisions

- **Keep URLs** (`/o/:org`, `/p/:org/:project/...`); the shell reads `:org` and `:project`.
- **Palette data** from loaders already run; no new search endpoint in this ticket (spec 011 §9).
- Rejected: a global state library; React context for the confirm dialog and route params for the rest.

## 7. Test strategy

Vitest: header switchers (search, pick, current check, actions), sidebar modes and back link, user menu items,
palette open with ⌘K/Ctrl+K, filter, keyboard navigation and Esc, shortcuts and g-chords ignored in inputs,
confirm dialog resolves true/false, phone drawer; Playwright: signed-in shell screenshot and keyboard path.
