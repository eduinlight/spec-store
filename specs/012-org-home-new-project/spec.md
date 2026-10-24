# Spec 012 — Organization home, new project and Get started

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-6

## 1. Context and goal

The organization page lists projects in a table and new projects are created with a slug and a name only. The
SDD Vault design ("Org home" and "New project" screens, FUNCTIONALITY §3.12, product-spec §11.2.1) shows each
project as a card with its role, constitution, active spec and progress, last activity and open questions, with
filters My projects / All / Archived; a New project dialog that can start empty, from an imported SDD tree or as
a copy of another project; and a Get started checklist that leads a new project to its first agent call and its
first spec.

## 2. Users / actors

- Organization member: sees the projects they can open.
- Organization owner: creates, copies, imports and archives projects.

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| `GET /api/v1/orgs/{org}/projects` (slug, name, role) | Project list (spec 005) | extend | Cards need summaries |
| `routes/organization.tsx` table, `routes/project-new.tsx` form | Spec 005 screens | replace | Design |
| `POST /api/v1/projects/{org}/{project}/import` with dry run | CLI import (spec 003, 005) | keep | The browser builds the same tree from a chosen folder |

## 4. User stories

### US-1 (P1) — Project cards
As a member I want to see my organization's projects as cards with their state, and filter mine, all and archived.
### US-2 (P1) — New project
As an owner I want to create a project empty, from an SDD tree on my disk, or as a copy of another project.
### US-3 (P2) — Get started
As an owner of a new project I want a checklist that takes me to my first agent call and first spec.
### US-4 (P2) — Archive
As an owner I want to archive a project so it becomes read-only and leaves the main list.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL return for each project of an organization: slug, name, the caller's role (or none), archived flag, constitution version and its last change time, the active spec (number, slug, status, tasks done and total), the last activity (person, client, time) and the number of open markers of the active spec.
- **FR-2** (US-1): THE SYSTEM SHALL show the projects as cards with those fields, filtered by My projects (projects where the caller is a member, not archived), All (not archived) and Archived, with an empty state per filter.
- **FR-3** (US-2): WHEN an owner opens New project, THE SYSTEM SHALL ask for a name, propose a slug from it (editable, monospaced, with a preview of how agents address it, `<org>/<slug>`), an optional repository URL, and a start option: Empty, Import SDD tree, or Copy project.
- **FR-4** (US-2): WHERE the start option is Import SDD tree, THE SYSTEM SHALL let the owner choose a folder, run the import as a dry run, show every problem found, and import only when there is none (spec 003 rules).
- **FR-5** (US-2): WHERE the start option is Copy project, THE SYSTEM SHALL create the project with the current constitution and every spec and current document of the chosen project of the same organization, as new revisions authored by the owner, with an audit entry.
- **FR-6** (US-3): WHEN a project has been created, THE SYSTEM SHALL show a Get started checklist on its Overview with the steps Create project (done), Connect your agent (with the connect command), First agent call (done when any write in the project came from an MCP client), and Write spec 001 (done when spec 001 exists), each derived from the project's data.
- **FR-7** (US-4): WHEN an owner archives a project, THE SYSTEM SHALL mark it archived with an audit entry and refuse every write to it with an authorization error until an owner restores it.
- **FR-8**: THE SYSTEM SHALL show every new text in English and Spanish.

## 6. Non-functional requirements

- **NFR-1**: The project summaries of an organization with 50 projects load in under 300 ms at p95 on the reference machine.

## 8. Edge cases

- Slug already taken in the organization: the dialog shows the error and keeps the inputs.
- Import folder without `docs/constitution.md` or `specs/`: the dry run reports it.
- Copy of a project with no constitution: copies the specs only.
- A reader sees cards but no New project action.

## 9. Out of scope

- "Inherit organization constitution" (SV-12): the option is shown disabled with a note.
- Project deletion (SV-17).
- Importing from a repository URL (Git provider app, roadmap).

## 10. Completion criteria

Every FR covered by a green test.
