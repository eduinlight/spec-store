# Spec 001 — Store Core

Status: clarified · Created: 2026-10-17 · Constitution: v1.0

## 1. Context and goal

Today every SDD artifact (constitution, spec, plan, tasks, research, validation) lives as a markdown file inside the project repository, and the agent reads and edits them as files. This spec moves those documents into a remote store shared by several projects and several users, and gives the agent the same control it has over the files today through an MCP interface: list projects and specs, find the active spec, read and write documents, create specs, change their status, mark tasks done and see progress. The value is one source of truth per project, outside the code repository, with authorship, history and access control that files in a repo do not give.

## 2. Users / actors

- Server admin: installs the server, creates users, issues and revokes tokens, creates projects and assigns their first owner.
- Project owner: manages the members of a project and archives its specs.
- Editor: creates specs and writes documents of the projects they belong to.
- Reader: reads the documents of the projects they belong to.
- AI agent: calls the MCP interface on behalf of a user, using that user's token; it never has rights of its own.

## 3. Current state

New project, no legacy code. The behaviour being replaced is the file layout described in `.sdd/README.md` (`docs/constitution.md`, `specs/NNN-name/*.md`, active spec = highest number) and the ten commands in `.claude/commands/`.

| Current piece | What it existed for | Decision | Justification (doc/code) |
|---|---|---|---|
| `docs/constitution.md` in the repo | Principles the agent reads before any task | replace | Becomes a per-project document in the store with versions (`.sdd/README.md` Structure) |
| `specs/NNN-name/{spec,plan,tasks,research,validation}.md` | One folder per spec, numbered | replace | Same five kinds and the same numbering, stored per project (`.sdd/README.md` Structure) |
| Active spec = highest `NNN` folder | Default target of every command | keep | Same rule, computed by the store over non-archived specs (`.sdd/README.md` last line) |
| `.sdd/templates/*.md` | Skeleton for each kind | keep | Served by the store as global templates; the files remain the seed |
| Branch `NNN-name` and `Tnn:` commits | Traceability commit → task → FR | keep | Store returns the branch name and records the commit per task (`.sdd/README.md` Git strategy) |
| Commands in `.claude/commands/sdd-*.md` | The workflow itself | keep | Rewriting them to use the store is a later spec; out of scope here |

## 4. User stories (by priority)

### US-1 (P1) — Administer users, tokens, projects and members
As a server admin I want to create users, issue and revoke their tokens, create projects and manage memberships from the server's command line so that a team can start using the store without any other tool.
Independent test: from an empty store, create an admin, a user, a token, a project and a membership, then a call with that token lists exactly that project.

### US-2 (P1) — Read the documents of my projects
As an agent acting for a member I want to list the projects I belong to, their specs, the active spec, and read any document, the constitution and the templates so that every SDD command has its inputs without touching files.
Independent test: a reader token reads every document of a project it belongs to and gets an authorization error on a project it does not belong to.

### US-3 (P1) — Create specs and write documents
As an agent acting for an editor I want to create a spec with the next number, write documents with protection against overwriting someone else's change, update the constitution, change a spec's status and mark a task done with its commit so that the workflow can run end to end against the store.
Independent test: create a spec, write its tasks document, mark one task done with a commit identifier, and read back the document showing that task checked.

### US-4 (P2) — See progress without parsing
As an agent I want the store to tell me the tasks, the pending markers and the overall status of a project so that `/sdd-status` and `/sdd-implement` do not have to parse markdown themselves.
Independent test: after writing a tasks document with three tasks and marking one done, the status reports one done out of three and the next pending task with its "Done when".

### US-5 (P2) — Know who changed what
As an owner I want every write to keep its author, time and message, and to be able to read earlier revisions, so that a wrong change can be understood and recovered.
Independent test: two writes to the same document produce two listed revisions with different authors, and the earlier body can be retrieved.

## 5. Functional requirements (EARS)

Patterns: `THE SYSTEM SHALL <behaviour>` · `WHEN <event>, THE SYSTEM SHALL <response>` ·
`IF <condition>, THEN THE SYSTEM SHALL <response>` · `WHILE <state>, THE SYSTEM SHALL <response>` ·
`WHERE <active option>, THE SYSTEM SHALL <response>`.

Administration (US-1)

- **FR-1** (US-1): WHEN an admin creates a user with a handle, THE SYSTEM SHALL store it with a unique handle and an admin flag; a second user with the same handle is rejected with an error that names the conflict.
- **FR-2** (US-1): WHEN an admin issues a token for a user, THE SYSTEM SHALL return the token in clear exactly once and keep only a form from which the token cannot be recovered; listing tokens shows label, creation time and last use, never the token.
- **FR-3** (US-1): WHEN an admin revokes a token, THE SYSTEM SHALL reject every later call presenting it with an authentication error, while the user's other tokens keep working.
- **FR-4** (US-1): WHEN an admin creates a project with a slug and a name, THE SYSTEM SHALL store it with a unique slug; a duplicate slug is rejected with an error that names the conflict.
- **FR-5** (US-1): WHEN an admin or an owner of the project assigns a member with a role among owner, editor and reader, THE SYSTEM SHALL store exactly one role per user and project, replacing any previous role; removing the membership ends the user's access to that project on the next call.
- **FR-6** (US-1): THE SYSTEM SHALL execute the administration commands only from the server's command line, without a token, each one naming the acting user by handle; the acting handle must be an existing admin, or for the member commands an owner of the target project, otherwise the command is rejected; the only exception is the creation of the first user on an empty store, which becomes an admin.

Reading (US-2)

- **FR-7** (US-2): WHEN a call reaches the MCP interface, THE SYSTEM SHALL authenticate it with a bearer token; a call with no token, an unknown token or a revoked token receives an authentication error and no data.
- **FR-8** (US-2): WHEN an authenticated caller lists projects, THE SYSTEM SHALL return only the projects where the caller has a membership, with their slug, name and the caller's role; an admin receives every project.
- **FR-9** (US-2): IF a caller addresses a project where they have no membership and are not an admin, THEN THE SYSTEM SHALL answer with an authorization error that does not reveal whether the project exists; an admin without membership may read any project but every write by them is authorized by their membership role alone.
- **FR-10** (US-2): WHEN a member lists the specs of a project, THE SYSTEM SHALL return for each spec its number, slug, status, branch name and which of the five document kinds exist, ordered by number.
- **FR-11** (US-2): WHEN a member asks for the active spec of a project, THE SYSTEM SHALL return the non-archived spec with the highest number, or an explicit "no active spec" result when none exists.
- **FR-12** (US-2): WHEN a member reads a document by project, spec number and kind, THE SYSTEM SHALL return its current body, its revision identifier, author and time; a missing spec or a kind not yet written receives a not-found error that says which of the two is missing.
- **FR-13** (US-2): WHEN a member reads the constitution of a project, THE SYSTEM SHALL return its current body and version; a project without a constitution receives a not-found error that says so.
- **FR-14** (US-2): WHEN a member asks for a template by kind (constitution, spec, plan, tasks, validation), THE SYSTEM SHALL return the global template body for that kind; an unknown kind receives a validation error listing the valid kinds.

Writing (US-3)

- **FR-15** (US-3): WHEN an editor or owner creates a spec with a slug, THE SYSTEM SHALL assign the next number after the highest existing number in that project (archived specs included, starting at 001), set status draft, derive the branch name `NNN-slug` and return number, slug and branch name.
- **FR-16** (US-3): IF the slug of a new spec is not one to four lowercase words of letters and digits joined by single hyphens, or is already used in the project, THEN THE SYSTEM SHALL reject the creation with a validation error and assign no number.
- **FR-17** (US-3): WHEN an editor or owner writes a document with a body and a message, THE SYSTEM SHALL store it as a new revision with author and time, make it the current revision, and return the new revision identifier; the first write of a kind for a spec creates the document.
- **FR-18** (US-3): IF a write targets an existing document and the expected revision identifier is missing or differs from the current one, THEN THE SYSTEM SHALL store nothing and answer with a conflict error that includes the current revision identifier.
- **FR-19** (US-3): IF a write targets a spec whose status is archived, or is made by a reader, THEN THE SYSTEM SHALL store nothing and answer with an authorization error.
- **FR-20** (US-3): WHEN an owner writes the constitution with a body, a version and a message, THE SYSTEM SHALL store it as a new revision only if the version differs from the current one; the same version is rejected with a conflict error, and a write by an editor or reader is rejected with an authorization error.
- **FR-21** (US-3): WHEN an editor or owner changes a spec status, THE SYSTEM SHALL accept only draft→clarified, clarified→approved, clarified→draft, approved→draft, and any status→archived requested by an owner; any other transition is rejected with a validation error naming the allowed targets.
- **FR-22** (US-3): WHEN an editor or owner marks a task done with a task identifier and a commit identifier of 7 to 40 hexadecimal characters, in a spec of any status other than archived, THE SYSTEM SHALL rewrite that task line in the tasks document with its box checked and the suffix `· commit: <identifier>` appended, and store the result as a new revision whose message names the task.
- **FR-23** (US-3): IF the task identifier does not exist in the tasks document, or the task is already done, or the commit identifier is malformed, THEN THE SYSTEM SHALL change nothing and answer with an error naming the cause.
- **FR-24** (US-3): WHEN two writes target the same document at the same time with the same expected revision, THE SYSTEM SHALL accept exactly one and reject the other with the conflict error of FR-18.

Derived reads (US-4)

- **FR-25** (US-4): WHEN a member lists the tasks of a spec, THE SYSTEM SHALL return for every task line of the tasks document its identifier, phase, parallel flag, referenced FRs, description, "Done when" text, done flag and commit identifier if any, in document order, with an optional filter for pending tasks only.
- **FR-26** (US-4): IF a line of the tasks document starts like a task but does not follow the task line format of the template, THEN THE SYSTEM SHALL report it in a separate list of malformed lines with its line number instead of dropping it.
- **FR-27** (US-4): WHEN a member lists the markers of a spec, THE SYSTEM SHALL return every `[NEEDS CLARIFICATION: ...]` and `[NEEDS RESEARCH: ...]` occurrence across the spec's documents with document kind, nearest heading and text.
- **FR-28** (US-4): WHEN a member asks for the status of a project, THE SYSTEM SHALL return the constitution version or its absence, the list of specs as in FR-10, and for the active spec: tasks done and total, the first phase with a pending task, the next pending task with its "Done when", and the count of markers.
- **FR-29** (US-4): WHEN any write completes, THE SYSTEM SHALL make every derived read issued afterwards reflect that write; a derived read never returns data from a superseded revision.

History (US-5)

- **FR-30** (US-5): WHEN a member lists the revisions of a document or of the constitution, THE SYSTEM SHALL return every revision with identifier, author, time and message, newest first.
- **FR-31** (US-5): WHEN a member reads a specific revision by identifier, THE SYSTEM SHALL return the body as it was stored; an unknown identifier receives a not-found error.
- **FR-32** (US-5): WHEN any write or administration command succeeds, THE SYSTEM SHALL record an audit entry with actor, project, action, target and time, readable by an admin from the command line.

## 6. Non-functional requirements

- **NFR-1**: A document read or derived read of a spec whose documents total up to 200 KB completes in under 100 ms at the 95th percentile with 10 concurrent callers on the reference development machine.
- **NFR-2**: A document body larger than 1 MB is rejected with a validation error; smaller bodies are stored byte for byte, including line endings.
- **NFR-3**: Tokens never appear in logs, audit entries, error messages or any response other than the one that issues them.
- **NFR-4**: The MCP interface is reachable by remote clients over the network and by a local client without network for tests; both expose the same operations.
- **NFR-5**: All messages, errors, identifiers and documents produced by the system are in English.

## 7. Key entities

- User: a person with a unique handle and an admin flag; owns tokens and memberships.
- Token: a credential issued to one user, with label, creation, last use and revocation; identifies the caller of every MCP call.
- Project: a unit of collaboration with a unique slug; owns one constitution, its specs and its memberships.
- Membership: one role (owner, editor, reader) of one user in one project.
- Spec: a numbered, slugged unit of work inside a project with a status and a branch name; owns up to five documents.
- Document: a markdown body of one kind (spec, plan, tasks, research, validation) for a spec, or the constitution for a project; always points to its current revision.
- Revision: an immutable body with author, time and message; revisions of a document form its history.
- Task and Marker: read-only views derived from the current revision of the documents.
- Audit entry: who did what, where and when, for every write and administration command.
- Template: the global skeleton of one document kind.

## 8. Edge cases

- Empty store: listing projects returns an empty list, not an error; status of a project with no specs says so; the first user created is an admin without naming an acting admin.
- A user removed from a project while an agent session is open: the next call fails with the authorization error of FR-9.
- Token revoked between two calls: the second call fails with FR-7; no partial write survives.
- Creating a spec while another is being created in the same project: numbers never collide and never skip.
- Writing a document for a spec number that does not exist: not-found, nothing created.
- Marking a task done when the tasks document has duplicate task identifiers: rejected as malformed, nothing changed.
- Status change to archived on a spec that was the active one: the next active spec is the next highest non-archived number.
- A constitution write by an editor: rejected with an authorization error (FR-20).
- A tasks document with Windows line endings: stored as is (NFR-2) and still parsed by FR-25.

## 9. Out of scope

- Web UI of any kind; import of an existing `specs/` tree and export back to files; a user-facing CLI beyond the administration commands of US-1.
- Section-level edits (replace or append a section), adding clarifications as a separate operation, FR-to-task coverage tables and revision diffs.
- OAuth or any login flow other than per-user bearer tokens; token expiry; rate limiting.
- Per-project templates; editing templates through the interface.
- Any git operation: the store records branch names and commit identifiers but never runs git.
- Rewriting the `.claude/commands/sdd-*.md` commands and the client-side project identity (marker file in the repo); these are a later spec.
- Hard deletion of users, projects, specs or documents; backups; multi-node deployment.

## 10. Completion criteria

- Every FR covered by at least one green test.
- Manual validation: start the server, create an admin, a user, a token and a project from the command line, connect a Claude Code session with that token, list projects, create a spec, write its tasks document, mark a task done with a commit identifier, and read the status showing one task done and the commit identifier.

## 11. Assumptions

- Administration commands are authorized by access to the server's data store on the host; no token is involved (FR-6).
- Global templates are seeded from `.sdd/templates/` at first start; the research kind has no template and is written free-form.
- Creating a spec creates no document; the first write of each kind creates it.
- The expected revision identifier is mandatory when writing an existing document, optional when creating it.
- Constitution versions are free-form strings compared for inequality only; the agent follows the major/minor rule of `/sdd-constitution`.
- The task line format is the one in `.sdd/templates/tasks.md`: `- [ ] Tnn [P] (FR-x, FR-y) Description — Done when: criterion`, optionally followed by `· commit: <identifier>` once done.
- Specs created through the interface get the branch name `NNN-slug`; whether the client creates that branch is the client's job.
- Project slugs and user handles follow the same format rule as spec slugs (FR-16); projects are addressed by slug in every call.
- Two concurrent first writes of the same document kind, both without an expected revision, are resolved like FR-24: exactly one creates the document.
- The server writes operational logs without document bodies or tokens; log format and level are plan decisions.
- Templates are served read-only; the research kind has none and asking for it returns the validation error of FR-14.

## 12. Open questions

None after /sdd-clarify on 2026-10-17.

## Clarifications

- 2026-10-17 · Who may write the constitution? · Owners only. Affected: FR-20, Edge cases.
- 2026-10-17 · Block task marking while the spec is draft? · No; any non-archived status allows it, the approval gate stays in the workflow. Affected: FR-22.
- 2026-10-17 · Audit actor for command-line administration? · An explicit, required acting admin handle; only the first user on an empty store is exempt. Affected: FR-6, FR-32, Edge cases.
- 2026-10-17 · Where does the commit identifier of a done task live? · Appended to the task line in the markdown, so the index stays a pure derivation. Affected: FR-22, FR-25, Assumptions.
- 2026-10-17 · Admin rights inside a project without membership? · Read everything, write only through a membership role. Affected: FR-9.
- 2026-10-17 · /sdd-analyze A1: FR-5 lets owners assign members but FR-6 only accepted admins on the command line. · FR-6 now accepts an owner of the target project for the member commands. Affected: FR-6.
