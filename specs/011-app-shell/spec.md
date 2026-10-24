# Spec 011 — App shell, command palette and shortcuts

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-5

## 1. Context and goal

The app shows a neutral header with a language select and the spec 005 pages in a centred column. The SDD Vault design (`SDD Vault App.dc.html`, FUNCTIONALITY §3.1–3.2, product-spec §11.4) wraps every screen in one chrome: a 60 px teal header with organization and project switchers, a command palette, shortcuts and notifications; a 220 px sidebar that switches between project navigation and account navigation; a user menu; and global feedback (toasts, confirmation dialogs, tooltips). Every later screen ticket renders inside this shell, so it comes first among the app screens.

## 2. Users / actors

- Signed-in person: navigates between organizations, projects and screens with mouse or keyboard.

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| `routes/root.tsx` header with organization select, language select, sign out | Spec 005 navigation | replace | Design shell |
| Routes `o/:org/*`, `p/:org/:project/*`, `profile` | Spec 005 screens | keep | Screens move into the shell; their redesign comes with their tickets |

## 4. User stories

### US-1 (P1) — Navigate with the shell
As a person I want to switch organization and project from the header and reach every screen from the sidebar.
### US-2 (P1) — Keyboard first
As a person I want a command palette and shortcuts to jump anywhere without the mouse.
### US-3 (P2) — Consistent feedback
As a person I want toasts after actions and a confirmation before anything destructive.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL show on every signed-in screen a 60 px header in `--primary-600` with the logo, an organization combobox (searchable, check on the current one, "Create organization"), a project combobox (searchable, each project's active spec and role, "All projects" and "New project"), a "Search or jump to… ⌘K" trigger, a keyboard shortcuts button and a notifications button.
- **FR-2** (US-1): WHILE a project is selected, THE SYSTEM SHALL show a 220 px sidebar with Overview; SDD: Active specs, Specs, Constitution; Project: Activity, Members, Audit, Settings; screens that do not exist yet show an empty state naming what is coming.
- **FR-3** (US-1): WHEN a person opens an item of the user menu, THE SYSTEM SHALL switch the sidebar to account mode: "← Back to {project}" (returning to the page they came from), Account: Profile, My tokens, Preferences; Organization · {org}: Security, Billing & usage, Audit.
- **FR-4** (US-1): THE SYSTEM SHALL pin a user menu at the bottom of the sidebar (avatar, name, handle) that opens upward with the account items, the organization items, the language choice and Sign out.
- **FR-5** (US-2): WHEN a person presses ⌘K or Ctrl+K or uses the trigger, THE SYSTEM SHALL open a command palette with suggested actions and searchable specs and navigation targets of the current organization, moving with ↑/↓, opening with ↵ and closing with Esc.
- **FR-6** (US-2): THE SYSTEM SHALL support the shortcuts `?` (list), `n` (notifications), `Esc` (close), and `g` followed by `o` Overview, `e` Active specs, `l` Specs, `c` Constitution, `a` Activity, `m` Members, `u` Audit, `p` Profile, and SHALL ignore them while the focus is in a text field.
- **FR-7** (US-3): THE SYSTEM SHALL confirm every destructive action in a dialog and report the outcome of actions in a toast.
- **FR-8** (US-1): WHEN a person switches organization or project, THE SYSTEM SHALL keep them on the same kind of screen when it exists in the new context.
- **FR-9**: THE SYSTEM SHALL keep all texts of the shell in English and Spanish.

## 6. Non-functional requirements

- **NFR-1**: Keyboard only: every control reachable and operable; visible focus; WCAG AA except the brand teal decision raised on SV-1.
- **NFR-2**: At phone width the sidebar becomes a drawer opened from the header; no horizontal scroll.

## 9. Out of scope

- The redesigned content of each screen (their tickets); the notifications drawer content (SV-15); tokens screen (SV-14); search across tasks and requirements (needs SV-7's index).
