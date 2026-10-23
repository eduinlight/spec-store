# Spec 005 — SaaS Foundation

Status: clarified · Created: 2026-10-22 · Constitution: v2.0

## 1. Context and goal

Specs 001 to 004 deliver spec-store as one self-hosted binary: a single SQLite file, users and static bearer tokens created by an administrator on the server host, a server-rendered read-only web view, and administration commands that open the database file directly. That shape cannot be offered as a hosted service: teams cannot sign themselves up, there is no boundary between one customer's projects and another's, the store cannot run on more than one machine, and the only way to manage anything is a shell on the server. This spec turns spec-store into the foundation of a SaaS product. Organizations become the tenant that owns projects. People sign up and sign in through a hosted identity provider, and agents and the CLI authorize through that same provider instead of copying long-lived tokens. Persistence moves to a database server that several instances can share. The single binary is split into separate deliverables that share one body of business rules: an HTTP API, an MCP server, a CLI, a public landing page and a browser app. The repository gets one entry point for development, linting, formatting and testing, and every merge reaches staging and every release reaches production without manual steps. Everything an agent can do today through MCP keeps working with the same rules. The landing and app screens use neutral styling until the product designs are delivered in a later spec.

## 2. Users / actors

- Visitor: an anonymous person on the public landing page who wants to understand the product and sign up or sign in.
- Organization owner: creates an organization, invites people, manages organization membership and creates projects in it.
- Organization member: belongs to an organization and works in the projects where they hold a project role.
- Project member (owner, editor, reader): the project roles of spec 001, now held inside an organization.
- Platform operator: the people running the service. Replaces the global admin flag of spec 001: reads every organization and its audit trail, but writes only through a membership.
- AI agent: an MCP client acting on behalf of one person, with exactly that person's access.
- CLI user: a person running the command-line tool on their own machine against the hosted service.
- Developer: works in the repository; needs one command per routine task and the same checks locally as in the pipeline.
- Release manager: merges to main and tags releases; needs both to reach their environment without manual steps and to be reversible.

## 3. Current state

| Current piece | What it existed for | Decision | Justification (doc/code) |
|---|---|---|---|
| Single crate and binary `spec-store` (`Cargo.toml`, `src/main.rs`) | One process for MCP, UI and administration (constitution P2) | replace | Split into a shared core package and separate api, mcp and cli deliverables (US-8) |
| Store, markdown, templates and tree logic (`src/store/*`, `src/markdown/*`, `src/templates.rs`, `src/tree.rs`) | Business rules of specs 001 and 003: revisions, optimistic locking, status transitions, task and marker parsing, import/export layout | keep | These rules are moved into the shared core unchanged; the deliverables call them (FR-19, FR-20) |
| SQLite through Diesel with the bundled library, migrations `0001_init`, `0002_web_ui` (`src/db.rs`, `src/schema.rs`, `migrations/`) | Only persistent state in one file (constitution P6, spec 004) | replace | One file cannot be shared by several instances. No store exists in use, so the new database starts empty and no data migration is needed (Clarification, 2026-10-22) |
| Users with an admin flag and static bearer tokens (`src/store/users.rs`, `src/auth.rs`, spec 001 FR-1 to FR-3, FR-7) | Identify every MCP call | replace | People are identified by the identity provider; agents and the CLI receive short-lived access through it (FR-6 to FR-13). The admin flag becomes the platform operator role |
| Administration commands that open the database file on the server host (`src/cli.rs`, spec 001 FR-6) | Bootstrap users, projects, members, tokens and repository URLs | replace | The CLI runs on the user's machine and acts through the API with the signed-in user's rights (US-4) |
| Browser sessions created from a pasted API token (`src/store/sessions.rs`, `src/ui/cookies.rs`, spec 002 FR-1 to FR-5, FR-24) | Sign in to the read-only UI | remove | Browser sign-in goes through the identity provider (FR-6, FR-7); there are no API tokens to show on a profile page |
| Server-rendered UI (`src/ui/*`, spec 002) | Read-only view of projects, specs, documents, tasks, history, diffs and audit | replace | Rebuilt as the browser app with the same read-only behaviour (US-6); markup-level guarantees (NFR-3 of spec 002, no JavaScript) do not carry over to a single-page app |
| Local MCP over standard input/output (`run_stdio` in `src/server.rs`, spec 001 NFR-4) | Local client without network, used by tests | remove | A local process cannot take part in the browser authorization flow; agents connect over the network only (Assumptions) |
| Streamable HTTP MCP endpoint and its tools (`src/mcp.rs`) | Agent interface of spec 001 | keep | Same tools and rules, now served by the mcp deliverable with the new authorization (FR-11, FR-19) |
| Host header allow-list (`--allowed-host`, README) | Protection against DNS rebinding for a self-hosted server | keep | Still required for every public endpoint (NFR-6) |
| Integration and performance tests (`tests/*`) | Behaviour and thresholds of specs 001 to 004 | keep | Re-pointed at the new deliverables and database; they are the guarantee that agent-visible behaviour did not change (FR-19) |

## 4. User stories (by priority)

### US-1 (P1) — Organizations own projects
As an organization owner I want my projects, members and audit trail to live inside my organization so that nobody outside it can see or change them.
Independent test: two people sign up, each creates an organization with a project of the same slug, and neither can list, read or address the other's project through MCP, the API, the CLI or the app.

### US-2 (P1) — Sign up and sign in through the identity provider
As a visitor I want to create an account and sign in on a branded page so that I can use the app without an administrator creating my account.
Independent test: from the landing page, a new person signs up, verifies their email, lands in the app signed in, signs out, and is asked to sign in again on the next visit to a protected page.

### US-3 (P1) — Agents connect by authorizing in the browser
As a project member I want to connect my agent by approving it in my browser so that I never copy a long-lived secret and the agent gets exactly my access.
Independent test: an MCP client with no credentials is told how to authorize, the person approves in the browser, and the client then lists only that person's projects.

### US-4 (P1) — The CLI works against the hosted service
As a CLI user I want to sign in from the terminal and manage organizations, projects and documents with my own rights so that I never need access to the server.
Independent test: the CLI signs in with a device code, creates an organization and a project, imports a spec directory, exports it to an empty directory with identical bytes, and signs out.

### US-5 (P1) — Agent workflow unchanged
As an AI agent I want every MCP operation of spec 001 to behave as before so that the SDD workflow keeps working without changes.
Independent test: the integration tests of spec 001 pass against the new deliverables and database, apart from the authentication cases that this spec replaces.

### US-6 (P1) — Browser app: read documents, manage the organization
As a project member I want to browse projects, specs, documents, tasks, history, diffs and audit in the browser app so that I keep the visibility spec 002 gave me; as an organization owner I want to set up my organization, people and projects there so that I can start without installing the CLI.
Independent test: every page of spec 002 has an equivalent screen showing the same data under the same access rules, and a newly signed-up person creates an organization, invites a second person who accepts, creates a project and gives them the editor role, all from the app.

### US-7 (P2) — Public landing page
As a visitor I want a public page that explains the product and leads me to sign up or sign in so that I can start without help.
Independent test: the landing page loads without a session, shows its content before any script runs, and its sign-up and sign-in actions open the identity provider's pages.

### US-8 (P2) — One repository, one entry point
As a developer I want one command to start the full local stack and one command each to lint, format-check and test everything so that local results match the pipeline.
Independent test: on a fresh clone with the documented prerequisites, the setup command brings up every service, and the lint, format and test commands exit 0.

### US-9 (P2) — Automatic deployments
As a release manager I want merges to main to reach staging and version tags to reach production automatically, each with its own data, so that releases need no manual steps and can be rolled back.
Independent test: a merge to main changes the version served on staging, a version tag changes the version served on production, and reverting the deployment change restores the previous version.

## 5. Functional requirements (EARS)

### Organizations and tenancy (US-1)

- **FR-1** (US-1): WHEN a signed-in person creates an organization with a slug and a name, THE SYSTEM SHALL store it with a slug that is unique across the platform and make that person its owner; a duplicate slug is rejected with an error that names the conflict.
- **FR-2** (US-1): WHEN an organization owner invites an email address, THE SYSTEM SHALL record a pending invitation that expires after 7 days and send that address, within 5 minutes, an email naming the organization and the inviter with a link that opens the acceptance screen of the app; WHEN the person who signs in with that verified email accepts it before it expires, THE SYSTEM SHALL make them an organization member; an expired, revoked or already-used invitation is rejected with an error that names the cause.
- **FR-3** (US-1): WHEN an organization owner changes a member's organization role between owner and member, or removes a member, THE SYSTEM SHALL apply it and remove the removed person's project roles in that organization; IF the change would leave the organization without an owner, THEN THE SYSTEM SHALL refuse it with an error that says so.
- **FR-4** (US-1): WHEN an organization owner creates a project with a slug and a name, THE SYSTEM SHALL store it in that organization with a slug unique within the organization and address it as `<organization slug>/<project slug>` in every interface; project roles are granted only to organization members.
- **FR-5** (US-1): IF a caller addresses an organization or a project of an organization they do not belong to, and they are not a platform operator, THEN THE SYSTEM SHALL answer with the same not-found error whether or not it exists; a platform operator receives read access and an authorization error on any write.

### Human sign-in (US-2)

- **FR-6** (US-2): WHEN a visitor without a session opens any app screen, THE SYSTEM SHALL send them to the identity provider's sign-in page and, after a successful sign-in, return them to the screen they asked for.
- **FR-7** (US-2): WHEN a person signs out of the app, THE SYSTEM SHALL end both the app session and the identity provider session, so that the next protected screen asks for credentials again.
- **FR-8** (US-2): WHEN a person signs in for the first time, THE SYSTEM SHALL create their profile from the identity provider's verified email and name, with a unique handle derived from the provider's username; later sign-ins update the email and name from the provider.
- **FR-9** (US-2): THE SYSTEM SHALL present the sign-in, sign-up, email verification, password reset and error pages of the identity provider with the product name, logo and colour tokens of the landing and app, in English and Spanish, following the language selected in the app or landing.
- **FR-10** (US-2): IF a person is disabled in the identity provider, THEN THE SYSTEM SHALL reject their app, API, MCP and CLI requests within 5 minutes of the change.

### Agent and CLI authorization (US-3, US-4)

- **FR-11** (US-3): WHEN an MCP request arrives without valid credentials, THE SYSTEM SHALL answer with an authentication error that tells the client where to authorize, as defined by the MCP authorization specification, and return no data.
- **FR-12** (US-3): WHEN a person approves an MCP client in the browser, THE SYSTEM SHALL let that client act with exactly that person's organization and project roles, with each access credential valid for at most 5 minutes and renewable without the browser for up to 30 days of inactivity; WHEN the person revokes that client from the app, THE SYSTEM SHALL reject its next renewal and every request after its current credential expires.
- **FR-13** (US-4): WHEN a CLI user runs the sign-in command, THE SYSTEM SHALL show a verification address and a short code, complete the sign-in once the person approves it in a browser, store the credentials only in the user's account on that machine, and refuse with a timeout error after 10 minutes without approval; the sign-out command removes the stored credentials and ends the session.
- **FR-14** (US-4): THE SYSTEM SHALL offer through the CLI, with the signed-in user's rights: organization create, list, invite, invitation list, resend and revoke, member role change and removal; project create, list, member set and remove, repository URL set and clear; audit list; and the import and export of spec 003.
- **FR-15** (US-4): WHEN a CLI user imports or exports a project, THE SYSTEM SHALL meet spec 003 FR-1 to FR-21 with the project addressed as in FR-4, reading and writing files only on the CLI user's machine.

### Persistence, revisions and audit (US-5)

- **FR-16** (US-5): THE SYSTEM SHALL keep every document write as an append-only revision with an audit entry and optimistic locking, as spec 001 FR-17, FR-18, FR-24 and FR-32 require, also when two writes with the same expected revision reach two different instances at the same time.
- **FR-17** (US-1): WHEN any organization, invitation, membership, project or document change succeeds, THE SYSTEM SHALL record an audit entry with actor, organization, project when it applies, action, target and time; organization owners read their organization's entries and platform operators read every entry.
- **FR-18** (US-9): WHEN two or more instances start at the same time against a database that needs schema changes, THE SYSTEM SHALL apply each change exactly once, and every instance SHALL start serving without error.

### Agent workflow (US-5)

- **FR-19** (US-5): THE SYSTEM SHALL meet spec 001 FR-8 to FR-32 through MCP, with "admin" read as platform operator, projects addressed as in FR-4, and project listings limited to the caller's organizations.
- **FR-20** (US-5): THE SYSTEM SHALL apply the same rules and return the same results and errors for an operation whether it is called through MCP, the API or the CLI.

### Browser app (US-6)

- **FR-21** (US-6): WHEN a signed-in person opens the app, THE SYSTEM SHALL show the organizations they belong to, let them switch between them, and show the selected organization's projects as spec 002 FR-7 describes.
- **FR-22** (US-6): THE SYSTEM SHALL meet spec 002 FR-8 to FR-22 and FR-25 in the app, with "admin" read as platform operator and projects addressed as in FR-4.
- **FR-36** (US-6): THE SYSTEM SHALL let an organization owner, from the app, do every organization, invitation, membership, project and repository URL operation listed in FR-14 except import and export, under the same rules and errors as FR-1 to FR-5 and FR-20, and SHALL let an invited person accept or decline an invitation from the link of FR-2.
- **FR-37** (US-6): WHEN a signed-in person belongs to no organization and has no pending invitation, THE SYSTEM SHALL show them a screen to create their first organization instead of an empty project list; WHEN they have pending invitations, THE SYSTEM SHALL list them on that screen.
- **FR-38** (US-6): THE SYSTEM SHALL never create, change or archive a spec, document, constitution or spec status in response to a browser request; those writes stay with agents and the CLI.
- **FR-23** (US-6): THE SYSTEM SHALL show every date and time in the app in the viewer's browser time zone and the selected language, with relative wording for times less than 7 days old and an absolute date with time beyond that.
- **FR-24** (US-6): THE SYSTEM SHALL offer English and Spanish in the app and the landing, use the browser's preferred language on the first visit (English when neither matches), keep the person's choice across visits, and fail the build when a text key exists in one language and not the other.

### Landing (US-7)

- **FR-25** (US-7): WHEN any visitor requests the landing page, THE SYSTEM SHALL return its full text content in the first HTML response without a session, with sign-up and sign-in actions that open the identity provider's sign-up and sign-in pages.

### Developer workflow (US-8)

- **FR-26** (US-8): THE SYSTEM SHALL provide one documented command that starts the local database, identity provider with its realm, theme and a seeded test person, and every app, and one command that removes all of it including its data.
- **FR-27** (US-8): THE SYSTEM SHALL provide one command each that lints, format-checks and tests every package and app in the repository, each exiting non-zero when any package fails.
- **FR-28** (US-8): WHEN a change is pushed to main, THE SYSTEM SHALL run the commands of FR-27 in the pipeline and mark the run as failed when any of them fails.

### Deployments (US-9)

- **FR-29** (US-9): WHEN a change is merged to main, THE SYSTEM SHALL build each changed deliverable and serve it on staging within 30 minutes without manual steps.
- **FR-30** (US-9): WHEN a version tag `v<major>.<minor>.<patch>` is pushed, THE SYSTEM SHALL build every deliverable from the tagged commit and serve it on production within 30 minutes without manual steps.
- **FR-31** (US-9): THE SYSTEM SHALL give staging and production separate databases, separate identity provider realms and separate secrets, so that no account, session or data of one environment is accepted in the other.
- **FR-32** (US-9): WHEN a deployment change is reverted in the deployment repository, THE SYSTEM SHALL serve the previous version of the affected deliverable in that environment within 10 minutes.
- **FR-39** (US-9): THE SYSTEM SHALL serve production at `spec-store.eduindev.com` (landing), `app-spec-store.eduindev.com` (app), `api-spec-store.eduindev.com` (API), `mcp-spec-store.eduindev.com` (MCP) and `auth-spec-store.eduindev.com` (identity provider), and staging at the same names with a `staging-` prefix, each configured per environment without a code change.
- **FR-33** (US-9): THE SYSTEM SHALL expose on every server deliverable a liveness check that answers without authentication and a readiness check that fails while its database or identity provider is unreachable.
- **FR-34** (US-9): THE SYSTEM SHALL back up the production platform database and identity provider database once every 24 hours to storage outside the cluster, keep each backup for 30 days, and report a failed backup as a failed scheduled job.
- **FR-35** (US-9): THE SYSTEM SHALL restore the latest production backup into a scratch database every 7 days, fail the check when the restore fails or the restored data has fewer organizations, projects or revisions than the backup recorded, and document the manual restore procedure.

## 6. Non-functional requirements

- **NFR-1**: Spec 001 NFR-1 holds: a document or derived read of a spec whose documents total up to 200 KB completes in under 100 ms at the 95th percentile with 10 concurrent callers, measured against the API with a local database on the reference development machine.
- **NFR-2**: Spec 001 NFR-2 holds: document bodies above 1 MB are rejected; smaller bodies are stored and returned byte for byte, including line endings.
- **NFR-3**: Spec 003 NFR-1 holds: importing 100 specs with five 20 KB documents each completes in under 30 seconds through the CLI against a local stack.
- **NFR-4**: Access tokens, refresh tokens, device codes and client secrets never appear in logs, audit entries, error messages, URLs or browser storage readable by page scripts.
- **NFR-5**: The API and MCP deliverables run with at least 2 instances per environment, and stopping one instance makes no request fail other than those in flight on it.
- **NFR-6**: Every public endpoint is served over HTTPS only and rejects requests whose host is not one of the environment's configured hosts.
- **NFR-7**: The landing page scores at least 90 for performance and accessibility in Lighthouse on mobile; the app's first screen after sign-in is interactive in under 3 seconds on a desktop broadband connection.
- **NFR-8**: Code, identifiers, documentation, API and MCP messages are in English; only text shown to people in the app, landing and identity provider pages is translated.
- **NFR-9**: Production loses at most 24 hours of committed data after the loss of the cluster's volumes (recovery point), and a documented restore brings the service back within 4 hours (recovery time).

## 7. Key entities

- Organization: the tenant; unique slug and name; owns projects, organization memberships, invitations and its audit trail.
- Organization membership: one role (owner, member) of one person in one organization.
- Invitation: a pending offer to join an organization, bound to an email address, with expiry and state (pending, accepted, revoked, expired).
- Person (user): profile linked to one identity provider account by its stable identifier; handle, email, name and the platform operator flag.
- Project: as in spec 001, now belonging to exactly one organization, with a slug unique inside it.
- Project membership: as in spec 001, limited to organization members.
- Authorized client: an MCP client or CLI session that a person approved; revocable by that person.
- Spec, Document, Revision, Task, Marker, Template: unchanged from spec 001.
- Audit entry: as in spec 001, with the organization it belongs to.

## 8. Edge cases

- Same project slug in two organizations: allowed; every interface addresses projects with the organization slug (FR-4).
- Invitation sent to an email that already has an account: accepted on that account's next sign-in, still within 7 days.
- Invitation accepted by a person whose verified email differs from the invited one: rejected (FR-2).
- Last owner tries to leave, step down or be removed: refused (FR-3).
- Member removed from an organization while their agent is connected: the next call to that organization's projects gets the not-found error of FR-5.
- Outbound mail service unreachable when an invitation is created: the invitation is still recorded, sending is retried for up to 1 hour, and the owner can resend it; a failed send is logged without the recipient's address in clear.
- Owner resends a pending invitation: the same invitation gets a new email and keeps its original expiry.
- Identity provider unreachable: sign-in and token renewal fail with an error that names the provider as unavailable; readiness fails (FR-33); requests with still-valid access keep working.
- MCP access expires in the middle of a session: the client is told to re-authorize (FR-11) and nothing is written by the rejected call.
- CLI sign-in never approved: timeout after 10 minutes (FR-13).
- Two concurrent writes on different instances with the same expected revision: one wins, one conflicts (FR-16).
- A backup runs while writes are in flight: the backup is consistent at a single point in time and contains no partial revision.
- Rolling deploy where old and new instances run at once against an already-changed schema: both keep serving (FR-18).
- Person's browser language is neither English nor Spanish: English (FR-24).
- Platform operator without memberships tries to write: authorization error (FR-5).

## 9. Out of scope

- Final visual design and content of the landing page and the app; this spec ships neutral styling and placeholder landing copy.
- Plans, pricing, subscriptions, payments, usage limits and billing.
- Writing specs, documents, constitutions or spec statuses from the browser app (FR-38); import and export from the browser.
- Migrating data from SQLite stores; there are none in use.
- Social or enterprise sign-in providers (Google, GitHub, SAML) and multi-factor policies beyond the identity provider's defaults.
- Static personal access tokens for headless or CI agents.
- Local MCP over standard input/output.
- Desktop or mobile apps.
- Organization deletion and data export for account closure.
- Emails other than the organization invitation and the identity provider's verification and password reset messages (no notifications, digests or marketing mail).
- Preview environments per pull request.

## 10. Completion criteria

- Every FR covered by at least one green test, or by a recorded manual check where a pipeline or browser makes an automated test impractical (FR-9, FR-29 to FR-32, FR-35's first run).
- Manual validation: on staging, sign up from the landing page, verify the email, create an organization and a project, connect Claude Code to the MCP endpoint by approving it in the browser, create a spec and mark a task done through the agent, see the spec, task and commit in the app, sign in with the CLI and export the project to an empty directory, then tag a release and see the same flow work on production.
- Constitution v2.0 is approved before `/sdd-plan 005` and amends P2 (PostgreSQL through `diesel`, the deliverables of this spec instead of a single binary), P6 (the database server as the only persistent state), P7 (authentication through the identity provider) and P8 (reference sources for Keycloak, PostgreSQL and the frontend stack); this spec is not approved before that.

## 11. Assumptions

- The user's chosen stack is a constraint for the plan, not a requirement to test: a Rust core package used by Rust api (axum), mcp and cli apps; PostgreSQL; Keycloak with a custom theme; a React Router 7 landing page; a React and Vite single-page app; Tailwind CSS, shadcn/ui, dayjs and react-i18next; layout `packages/core`, `apps/api`, `apps/cli`, `apps/mcp`, `apps/landing`, `apps/app`.
- Repository conventions follow the lightchat project (`../lightchat`): a Makefile as the single entry point, workspace-level rustfmt and clippy settings, Docker Compose for local services, slim runtime images, GitHub Actions with a reusable deployment bump. Frontend tooling follows the portfolio project (`../portfolio`): React Router 7, Biome for linting and formatting, i18next, Tailwind CSS 4, Vitest.
- Deployment follows the lightchat-cd project (`../lightchat-cd`): a separate `spec-store-cd` repository with `base/`, `staging/` and `prod/` Kustomize overlays, one ArgoCD AppProject and an Application per environment, container images in GitHub Container Registry, k3s behind the existing WireGuard and Traefik ingress, main → staging and `v*` tag → production. PostgreSQL and Keycloak run inside the cluster in each environment's namespace, and the theme ships in a custom Keycloak image.
- No local reference repository uses Keycloak; its theme and realm conventions are taken from the official Keycloak documentation and cited in `research.md`.
- Self-registration with email verification is enabled in the identity provider; there is no waiting list.
- Organization owners hold owner rights on every project of their organization.
- The handle is the identity provider username, which the realm keeps unique and unchangeable.
- The app talks only to the API; the MCP deliverable and the API share the core and the database, not each other.
- Backups go to the homelab Garage S3 store that already holds lightchat media, outside the k3s cluster, in a bucket per environment; staging is not backed up.
- One outbound SMTP service per environment is used both by the platform for invitations and by the identity provider for its own mail; invitation emails are sent in the inviter's selected language (English or Spanish).
- Organization slugs follow the project slug rules of spec 001 FR-16 (one to four lowercase words of letters and digits joined by single hyphens).
- Every server deliverable writes structured JSON logs to standard output with a request identifier; metrics and tracing export are left to a later spec.
- The 5-minute credential lifetime of FR-12 also bounds FR-10: a disabled person's credentials stop being renewed at once and the last one expires within 5 minutes.
- NFR-3 relaxes spec 003 NFR-1 from 10 to 30 seconds because the import now crosses the network between the CLI and the API.
- The pipeline runs on push to main and on tags, not as a pull request gate, as in lightchat.

## 12. Open questions

None.

## Clarifications

- 2026-10-22 · Scope: one foundation spec; landing and app screens come in later specs with the designs · §1, §9.
- 2026-10-22 · Tenancy: organizations own projects; billing out of scope · US-1, FR-1 to FR-5.
- 2026-10-22 · Agent and CLI authorization: MCP authorization flow with the identity provider, CLI device sign-in, static tokens removed · FR-11 to FR-13.
- 2026-10-22 · Legacy data: no store is in use; start empty, no data migration · §3, §9.
- 2026-10-22 · Browser app: port the read-only views of spec 002 with neutral styling · US-6, FR-21 to FR-24.
- 2026-10-22 · Deployment: same pattern as lightchat and lightchat-cd, with PostgreSQL and Keycloak in the cluster · US-9, §11.
- 2026-10-22 · Q: Constitution v1.1 conflicts with P2, P6, P7 and P8; how is it resolved? A: Amend to v2.0 with `/sdd-constitution` after clarification and before `/sdd-plan`; approval of this spec is gated on v2.0 · Header, §10, §12.
- 2026-10-22 · Q: How does an invited person learn about the invitation? A: The system emails an accept link through an SMTP provider shared with the identity provider · FR-2, FR-14, §8, §9, §11.
- 2026-10-22 · Q: What backup and recovery guarantee does production have? A: Daily backups of both databases to storage outside the cluster, kept 30 days, with a restore check every 7 days; at most 24 hours of data loss · FR-34, FR-35, NFR-9, §8, §11.
- 2026-10-22 · Q: Should the browser app manage organizations, invitations and members? A: Yes: organization, invitation, membership, project and repository URL management in the app; documents stay read-only in the browser · US-6, FR-22, FR-36 to FR-38, §9, §12.
- 2026-10-22 · Q: Which hostnames do the environments use? A: The lightchat pattern under `eduindev.com` with a `staging-` prefix · FR-39, §11, §12.
- 2026-10-22 · Resolved by the reviewer without a question: FR-12 lifetime made consistent with FR-10 (5-minute credentials, 30-day renewal), organization slug rules, logging, NFR-3 threshold · FR-12, §11.
