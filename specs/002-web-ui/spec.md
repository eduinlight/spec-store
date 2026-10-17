# Spec 002 — Web UI

Status: approved · Created: 2026-10-17 · Constitution: v1.0

## 1. Context and goal

Spec 001 moved the SDD documents into a shared store that agents reach through MCP and admins reach through the command line. People on the team have no way to look at the documents without one of those two: a reviewer who wants to read a spec, an owner who wants to see progress, or anyone who wants to know who changed what has to ask an agent or run commands on the server host. This spec adds a read-only browser view of everything the store already knows: projects, specs, documents rendered from markdown, tasks with their commits, open markers, revision history with diffs, and the audit trail for those allowed to see it. The value is visibility for humans without giving the browser any power to change the documents, so the agent workflow stays the only writer.

## 2. Users / actors

- Visitor: a browser without a session; may only reach the login page.
- Member (reader, editor or owner): signs in with their own API token and reads the projects they belong to.
- Owner: additionally sees the audit trail of their projects.
- Admin: reads every project and the global audit trail.
- Server admin: sets a project's repository URL from the command line so commits become links.

## 3. Current state

Spec 001 (`specs/001-store-core/spec.md`) delivered the store and the MCP interface. The server answers `POST /mcp` behind a bearer token and `GET /healthz`; nothing answers a browser.

| Current piece | What it existed for | Decision | Justification (doc/code) |
|---|---|---|---|
| HTTP server with `/mcp` and `/healthz` (`src/server.rs`) | MCP endpoint for agents | keep | The UI is served by the same process and address; the MCP endpoint and its authentication do not change (constitution 2, single binary) |
| Bearer-token middleware (`src/auth.rs`) | Authenticates MCP calls per request | keep | Browsers cannot send a bearer header on navigation; the UI adds a session created from the same token, the MCP path keeps using the header (FR-7 of spec 001) |
| Store read operations: projects, specs, active spec, documents, constitution, tasks, markers, status, revisions (`src/store/*.rs`) | Read path of the MCP tools | keep | The UI shows exactly what the tools return; one read path, two presentations (constitution 5) |
| Audit listing without caller or paging (`Store::list_audit`, used by `audit list --as`) | Admin reads the trail on the command line | replace | The browser needs the listing authorized per caller (owner of the project or admin) and in pages |
| Project attributes: slug and name only (`projects` table, `project add`) | Identify a project | replace | A repository URL per project is needed to link commits; set from the command line like the other project attributes |
| `README.md` sections on running and connecting | Operator documentation | keep | A section on the browser view is added |

## 4. User stories (by priority)

### US-1 (P1) — Sign in with my token
As a member I want to paste my API token once in the browser and stay signed in so that I can read my projects without an agent or server access.
Independent test: with a valid token the login page leads to the projects page; with an unknown or revoked token it shows an error and no data; the profile page lists the user's tokens without secrets; after logout the next page request asks for the token again.

### US-2 (P1) — Browse my projects and specs
As a member I want to see my projects, each project's specs with status and progress, and each spec's documents rendered so that I can review a spec as easily as a file in a repository.
Independent test: a reader of one project sees that project only, opens its active spec and reads the rendered spec document and its raw text.

### US-3 (P1) — See tasks, commits and markers
As an owner I want each spec's tasks with their done state and commit links, and the open markers, so that I know where the work stands without reading the tasks document line by line.
Independent test: a tasks document with three tasks, one done with a commit, shows one checked row with a link to that commit in the project's repository and two pending rows; a document with an open clarification marker lists it with its heading.

### US-4 (P2) — Follow the history of a document
As an owner I want to list the revisions of a document, open an earlier one and see what changed against the previous revision so that a wrong change can be understood and recovered.
Independent test: two writes to the same document produce two listed revisions; opening the second shows a diff with the added and removed lines against the first.

### US-5 (P2) — See who did what
As an owner I want the audit trail of my project in the browser, and as an admin the trail of every project, so that administration and writes can be reviewed without the command line.
Independent test: an owner sees the project's entries newest first; an editor of the same project sees no audit link and gets an authorization error when requesting the page directly; an admin sees entries of every project with a project filter.

## 5. Functional requirements (EARS)

Patterns: `THE SYSTEM SHALL <behaviour>` · `WHEN <event>, THE SYSTEM SHALL <response>` ·
`IF <condition>, THEN THE SYSTEM SHALL <response>` · `WHILE <state>, THE SYSTEM SHALL <response>` ·
`WHERE <active option>, THE SYSTEM SHALL <response>`.

Session (US-1)

- **FR-1** (US-1): WHEN a visitor without a session requests any UI page other than the login page, THE SYSTEM SHALL answer with the login page and no project data, remembering the requested page so that it is shown after a successful login.
- **FR-2** (US-1): WHEN a visitor submits an existing, non-revoked API token on the login page, THE SYSTEM SHALL create a browser session bound to that token's user, mark the token as used, and show the requested page or the projects page.
- **FR-3** (US-1): IF the submitted token is unknown or revoked, THEN THE SYSTEM SHALL show the login page with an error that says the token is not valid, create no session, and never echo the submitted value.
- **FR-4** (US-1): WHILE a session exists, THE SYSTEM SHALL treat every request as the session's user; IF the token behind the session has been revoked, or the session is older than its lifetime, THEN THE SYSTEM SHALL end the session and answer as for a visitor (FR-1).
- **FR-5** (US-1): WHEN a signed-in user logs out, THE SYSTEM SHALL end the session so that a later request carrying the same session identifier is treated as a visitor.
- **FR-6** (US-1): THE SYSTEM SHALL show on every page the handle of the signed-in user, their admin flag when set, a link to their profile page and a logout control.
- **FR-24** (US-1): WHEN a signed-in user opens their profile page, THE SYSTEM SHALL list their own tokens with label, creation time, last use and revoked state, marking the one behind the current session, and SHALL never show any token value or any other user's tokens.

Projects (US-2)

- **FR-7** (US-2): WHEN a signed-in user opens the projects page, THE SYSTEM SHALL list the projects of their memberships with name, slug, role, constitution version or its absence, number of specs, and for the active spec its number, slug, status and tasks done over total; an admin additionally sees every other project, marked as read-only access without membership; a user with no projects sees an explicit empty message.
- **FR-8** (US-2): IF a signed-in user requests a project, spec, document, revision or audit page of a project where they have no membership and are not an admin, THEN THE SYSTEM SHALL answer with the same not-found page whether or not the project exists.
- **FR-9** (US-2): WHEN a member opens a project page, THE SYSTEM SHALL show the project's name, slug, repository URL when set, the constitution version with a link to the constitution document or its absence, and every spec with number, slug, status, branch name, which document kinds exist, and tasks done over total, ordered by number with the active spec marked.
- **FR-25** (US-2): WHEN a member or an admin opens a project page, THE SYSTEM SHALL list the project's members with handle and role, owners first, so that every member can see who reads, writes and owns.
- **FR-10** (US-2): WHEN a member opens a spec page, THE SYSTEM SHALL show number, slug, status, branch name, and one section per document kind in the order spec, plan, tasks, research, validation, each either rendered with its current revision's author, time and message, or marked as not written.
- **FR-11** (US-2): WHEN a document or the constitution is shown, THE SYSTEM SHALL render its markdown as HTML with headings, paragraphs, lists, task boxes in their checked state, tables, links, emphasis and code blocks, and SHALL offer a raw view that shows the stored text byte for byte.
- **FR-12** (US-2): IF a document contains HTML tags or scripts, THEN THE SYSTEM SHALL show them as literal text in the rendered view and never execute or interpret them.

Tasks, commits and markers (US-3)

- **FR-13** (US-3): WHEN a spec with a tasks document is shown, THE SYSTEM SHALL show a tasks table with identifier, phase, parallel flag, referenced FRs, description, "Done when", done state and commit identifier, in document order, grouped by phase with each phase's done over total, plus a separate list of malformed lines with their line numbers when any exist.
- **FR-14** (US-3): WHERE the project has a repository URL, THE SYSTEM SHALL show every commit identifier of a done task as a link to that commit in the repository; without a repository URL the identifier is plain text.
- **FR-15** (US-3): WHEN a spec is shown, THE SYSTEM SHALL list every `[NEEDS CLARIFICATION: ...]` and `[NEEDS RESEARCH: ...]` marker across its documents with kind, document kind, nearest heading and text, and SHALL show the count on the spec page and on the project page next to the active spec.
- **FR-16** (US-3): WHEN an admin or an owner of the project sets the project's repository URL from the command line, THE SYSTEM SHALL store it when it is an absolute `http` or `https` URL, allow clearing it, record an audit entry, and reject any other value with a validation error naming the rule.

History (US-4)

- **FR-17** (US-4): WHEN a member opens the history of a document or of the constitution, THE SYSTEM SHALL list every revision with identifier, author, time, message and, for the constitution, version, newest first, marking the current one.
- **FR-18** (US-4): WHEN a member opens a specific revision, THE SYSTEM SHALL show its body rendered and raw as in FR-11, labelled as not current when it is not, with a link to the current revision; an unknown revision identifier receives the not-found page.
- **FR-19** (US-4): WHEN a member opens the diff of a revision, THE SYSTEM SHALL show a line-by-line comparison against the previous revision of the same document with added and removed lines marked and unchanged context around them; the first revision shows that it has no predecessor.

Audit (US-5)

- **FR-20** (US-5): WHEN an owner opens the audit page of their project, THE SYSTEM SHALL list the project's audit entries with actor, action, target and time, newest first, in pages of 50 with links to older and newer pages.
- **FR-21** (US-5): WHEN an admin opens the global audit page, THE SYSTEM SHALL list the entries of every project and the entries without project, newest first, in pages of 50, with a filter by project slug.
- **FR-22** (US-5): IF a signed-in user who is neither an admin nor an owner of the project requests an audit page, THEN THE SYSTEM SHALL answer with an authorization error page and no entries, and SHALL show no audit link to them.

Read-only guarantee (all)

- **FR-23** (US-2): THE SYSTEM SHALL accept from the browser only requests that read data, create or end a session, and SHALL never create, change or archive a project, spec, document, constitution, membership, user or token in response to a UI request.

## 6. Non-functional requirements

- **NFR-1**: A project, spec or document page of a spec whose documents total up to 200 KB is produced in under 300 ms at the 95th percentile with 10 concurrent sessions on the reference development machine.
- **NFR-2**: API tokens never appear in pages, URLs, cookies, logs or audit entries; the session identifier cannot be turned back into the token and is not readable by page scripts.
- **NFR-3**: Every page and view, including the raw view, revision pages and diffs, is reachable by following links with JavaScript disabled.
- **NFR-4**: The UI is served by the same process and address as the MCP endpoint, which keeps answering agents unchanged while the UI is in use.
- **NFR-5**: A document of the maximum size allowed by spec 001 (1 MB) renders without error in both the rendered and the raw view.
- **NFR-6**: All pages, messages and errors are in English.

## 7. Key entities

- Session: a browser's signed-in state bound to one user through one API token; has a creation time, a lifetime and an end (logout, expiry or token revocation). The only state the UI creates.
- Repository URL: an optional attribute of a project used to turn commit identifiers into links.
- Rendered document: the HTML presentation of a revision's markdown; derived, never stored.
- Diff: the line comparison of a revision with its predecessor in the same document; derived, never stored.
- Audit page: a window of 50 audit entries of a project, or of all projects for an admin.
- Profile: the signed-in user's handle, admin flag and own tokens without secrets.

## 8. Edge cases

- Empty store or user with no memberships: the projects page shows an explicit empty message, not an error; an admin on an empty store sees the empty message too.
- Project without constitution or without specs: the project page says so instead of showing empty tables.
- Spec with no documents: the spec page shows every kind as not written and no tasks or markers sections' data.
- Tasks document with only malformed lines: the tasks table is empty and the malformed list shows every line.
- Member removed from the project or token revoked while the browser is open: the next request gets the not-found page (FR-8) or the login page (FR-4), with nothing shown.
- Session cookie from another store or from before a database reset: treated as a visitor.
- Repeated invalid logins: each one fails with FR-3; there is no lockout.
- Document with raw HTML, script tags or markdown link to `javascript:`: shown as text or rendered inert (FR-12).
- Document of 1 MB with Windows line endings: renders and the raw view returns the exact bytes (NFR-5, NFR-2 of spec 001).
- Revision 1 diff: shows "no predecessor"; a constitution revision whose body equals the previous one shows an empty diff with the version change in the metadata.
- Repository URL set to a value with a trailing slash or query: the stored value is used as given; commit links are built from it without duplicating slashes.
- Admin without membership opens a project: everything is readable, the audit page is readable, and the page states the access is read-only without membership.
- Login with a remembered requested page that points outside the UI: the remembered value is ignored and the projects page is shown, so the login form can never redirect to another site.
- Project with no members (all removed by an admin): the members list says so.

## 9. Out of scope

- Any write from the browser: editing documents, creating specs, changing status, marking tasks, managing members, users or tokens.
- Login with username and password, OAuth, or any credential other than the existing API tokens; token issuance from the browser.
- Diff between two arbitrary revisions; only a revision against its predecessor.
- Search across documents, notifications, live updates, comments.
- Links from branch names to the repository; only commits are linked.
- Changes to the MCP tool surface, including exposing the repository URL to agents.
- Visual design requirements beyond readable, plain pages; mobile layout guarantees; themes.
- Rate limiting of logins; session management across several devices beyond independent sessions.

## 10. Completion criteria

- Every FR covered by at least one green test.
- Manual validation: start the server with the store created during the validation of spec 001, open the UI in a browser, sign in with the editor's token, see the project and its spec, read the tasks table with the done task linking to its commit after setting the repository URL from the command line, open the history of the tasks document and the diff of its second revision, log out and confirm the login page is shown again.

## 11. Assumptions

- A session lasts 30 days from creation unless ended earlier by logout or revocation of its token; the lifetime is a server constant, not configurable in this spec.
- Markdown is rendered with CommonMark plus tables and task lists, which is what the templates use; other extensions are not required.
- The commit link is the repository URL followed by `/commit/` and the identifier, which is valid for GitHub, GitLab and Gitea; other hosts are not considered.
- The repository URL is unset for every existing project after upgrade; nothing else changes for them.
- Rendering and diffs are computed on each request from the stored markdown; nothing derived is persisted (constitution 5).
- The login page is reachable by anyone who can reach the server, as `/healthz` is today; it reveals nothing but the form.
- Audit pages show the same entries the command line shows; no new audit actions are introduced except the repository URL change.
- The UI uses the same `--allowed-host` protection as the MCP endpoint.
- TLS is terminated outside the binary; the operator tells the server when it is served over HTTPS so that the session cookie is restricted to HTTPS, and the default without that setting suits loopback testing over plain HTTP.
- The UI is always on; it cannot be switched off separately from the MCP endpoint in this spec.
- Members are listed to every member because membership is not secret inside a team; audit stays restricted because it exposes administration and token events.

## 12. Open questions

None after /sdd-clarify on 2026-10-17.

## Clarifications

- 2026-10-17 · Which feature for spec 002? · Read-only web UI; SDD commands on the store and CLI import/export stay for later specs. Affected: whole spec.
- 2026-10-17 · How does a person authenticate in the browser? · Paste the API token once; a server session cookie follows until logout or revocation. Affected: US-1, FR-1 to FR-6.
- 2026-10-17 · How far does revision history go? · List, open any revision, and a line diff against the predecessor. Affected: US-4, FR-17 to FR-19.
- 2026-10-17 · Who sees the audit trail? · Owners for their projects, admins for everything. Affected: US-5, FR-20 to FR-22.
- 2026-10-17 · Link commits to a repository? · Yes, through a repository URL set per project from the command line. Affected: FR-14, FR-16, Current state.
- 2026-10-17 · How are documents shown? · Rendered markdown with a raw view. Affected: FR-11, FR-12.
- 2026-10-17 · Should a signed-in user see their own tokens? · Yes, a read-only profile page without secrets. Affected: US-1, FR-6, FR-24, Key entities.
- 2026-10-17 · Should the project page list members and roles? · Yes, visible to every member and to admins. Affected: US-2, FR-25, Edge cases, Assumptions.
