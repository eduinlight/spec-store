# Spec 013 — Specs, documents, traceability and constitution in the SDD Vault design

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-7

## 1. Context and goal

The app shows specs, documents, tasks, markers, history and the constitution as the read-only pages of spec 002,
ported with neutral styling. The SDD Vault design (FUNCTIONALITY §3.5–3.7, product-spec §11.2.3–7) turns them
into a specs list of cards, a spec detail with lifecycle steps and tabs (Spec, Plan, Tasks, Traceability,
Research, Validation, Markers, History), a readiness checklist, a traceability matrix from requirements to tasks
and commits, a history of resolved questions, and a document-style constitution page. This spec covers reading;
approving, answering and restoring from the browser come with SV-11.

## 2. Users / actors

- Member (reader, editor, owner) of a project.

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| `routes/project.tsx` specs table, `spec.tsx`, `document.tsx`, `history.tsx`, `revision.tsx`, `diff.tsx` | Read-only views of spec 002 | replace | Design |
| Task and marker parsers in `packages/core/src/markdown` | Indexes derived from markdown | keep, extend | FR index and resolved markers derive from the same documents |

## 4. User stories

### US-1 (P1) — Browse specs
As a member I want the specs of a project as cards, searchable and filterable by status, newest first.
### US-2 (P1) — Read a spec
As a member I want a spec's documents, tasks, traceability, markers and history in tabs, with its lifecycle and readiness.
### US-3 (P2) — Read the constitution
As a member I want the constitution as a document with its principles, version, dates and history.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL list a project's specs as cards with number, created month, title, status, open-questions count, one-line summary, slug, which of the five documents exist, number of requirements, last update, contributors and task progress; newest first by default, with sorting by oldest first and recently updated, search by title, slug or number, status filters with counts (All, Active, Draft, Clarified, Approved, Archived), and pages of 20 with "Load N more".
- **FR-2** (US-2): THE SYSTEM SHALL show a spec's header (number, title, status, branch), lifecycle steps Draft → Clarified → Approved → Archived, and tabs Spec, Plan, Tasks · done/total, Traceability, Research, Validation (disabled when not written), Markers · open count, History.
- **FR-3** (US-2): WHEN a document tab is shown, THE SYSTEM SHALL show its revision line (revision n of m, author, client, relative time), rendered and raw views, a table of contents, each requirement with a tooltip naming the tasks that reference it, and a readiness checklist (open markers resolved, every requirement has a task, plan and tasks written, constitution exists).
- **FR-4** (US-2): THE SYSTEM SHALL show the tasks grouped by phase with counts, filters Pending, Done, Mine (completed by me) and Parallel, and for each task its id, description, parallel flag, requirement chips, done-when and, when done, who completed it, through which client and with which commit.
- **FR-5** (US-2): THE SYSTEM SHALL show a traceability row per requirement: its text, the tasks that reference it (done or pending), a coverage bar, the commits of its done tasks, and a status Covered, In progress, Not started or No tasks; with a note listing tasks without requirements.
- **FR-6** (US-2): THE SYSTEM SHALL list the open markers of a spec and the resolved ones: a marker present in a revision and absent from the next revision of the same document is resolved by that revision's author and client at its time.
- **FR-7** (US-2): THE SYSTEM SHALL show a document's revisions (author, client, message, time) and the diff between two revisions.
- **FR-8** (US-3): THE SYSTEM SHALL show the constitution as a document with its version, first and last revision dates and authors, a lock note for non-owners, its numbered principles and its version history.
- **FR-9**: THE SYSTEM SHALL show archived specs and archived projects as read-only with a banner, and every new text in English and Spanish.

## 9. Out of scope

- Approve, archive, answer markers, restore revisions, amend the constitution from the browser (SV-11).
- Live presence and "agents live" (SV-13); the card shows the open-questions badge only.
- Inherited organization principles (SV-12).
