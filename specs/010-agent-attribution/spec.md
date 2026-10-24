# Spec 010 — Acting agent, origin and refused writes

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-4

## 1. Context and goal

Revisions and audit entries record the person who wrote, but not the agent acting for them, nor where the request came from, and a refused write (an authorization error or an optimistic-locking conflict) leaves no trace because its transaction is rolled back. The SDD Vault design shows the person and the agent on every action ("Priya Shah · via Cursor"), an audit trail with Via, Origin and a result of OK, Denied or Conflict, and an activity feed that tells "write on r12 rejected · current r13 · re-read and merged · saved as r14". Most later screens (overview, activity, audit, presence, billing usage) read this data, so it is recorded first.

## 2. Users / actors

- Agent (MCP client such as Claude Code or Cursor), CLI user, browser user: their writes are attributed.
- Owner reading history and the audit trail: sees who, through which client, from where, and with which result.

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| `audit_log(actor, organization, project, action, target, time)` | Audit of successful writes (spec 005 FR-17) | extend | Add client, origin, result |
| `revisions(author, message, time)` | Append-only history | extend | Add client |
| Writes inside one transaction with their audit row | Atomic audit | keep | Refusals are recorded in a separate transaction after the rollback |
| `Caller { user_id, handle, is_operator }` | Who calls | extend | Carries the client and origin of the call |

## 4. User stories

### US-1 (P1) — Attributed writes
As an owner I want each revision and audit entry to name the client that made it and the origin of the request.
Independent test: a write through MCP, through the CLI and through the app each shows its client in history and audit.

### US-2 (P1) — Refused writes are kept
As an owner I want denied writes and conflicts to appear in the audit trail, so I can see agents trying to break the rules or racing each other.
Independent test: an editor's constitution write and a stale document write each leave an audit entry with result Denied or Conflict, and change nothing else.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): WHEN a write succeeds, THE SYSTEM SHALL record with its revision (if any) and its audit entry the client that made it: for MCP the client name given by the request, otherwise the name the client registered with the identity provider, otherwise "MCP client"; "sddvault CLI" for the CLI; "Web app" for browser sessions; and the origin IP address of the request.
- **FR-2** (US-1): THE SYSTEM SHALL take the origin from the socket peer, or, WHERE a number of trusted proxy hops is configured, from the `X-Forwarded-For` entry that many hops from the right.
- **FR-3** (US-2): IF a member's write is refused with an authorization error, THEN THE SYSTEM SHALL record an audit entry with result Denied, the action and target of the attempted write, and no other change.
- **FR-4** (US-2): IF a write is refused because its expected revision is not the current one, THEN THE SYSTEM SHALL record an audit entry with result Conflict naming the expected and the current revision, and no other change.
- **FR-5** (US-2): IF a caller who is not a member addresses a project, THEN THE SYSTEM SHALL keep answering not found and SHALL NOT record the attempt in that project's audit trail.
- **FR-6** (US-1, US-2): THE SYSTEM SHALL return client, origin and result with every audit entry, and client with every revision, through the API, the MCP revision tools and the app's history and audit pages; successful entries have result OK.
- **FR-7** (US-1): THE SYSTEM SHALL keep MCP tool results, MCP errors and API errors unchanged apart from the added fields.

## 6. Non-functional requirements

- **NFR-1**: Resolving the client name adds no identity-provider call to more than one request per client every 10 minutes per instance.
- **NFR-2**: Spec 001 NFR-1 and spec 005 NFR-1 keep holding.

## 7. Key entities

- Client: name of the program acting for a person.
- Audit entry: actor, client, origin, organization, project, action, target, result, time.

## 8. Edge cases

- A client that sends no name and registered no name: "MCP client".
- No `X-Forwarded-For` header while hops are configured: the socket peer.
- Validation errors (malformed body, bad slug) are not recorded: they are input mistakes, not refused writes.
- A refused write whose audit insert itself fails still returns the original error to the caller.

## 9. Out of scope

- Showing the new fields in the redesigned screens (SV-7, SV-8, SV-10 do it); this spec adds columns to the current history and audit pages only.
- Approximate location from the IP address.
- Linking a conflict to the later successful write (derived when the activity feed is built, SV-8).

## 10. Completion criteria

- Every FR covered by a green test.

## 11. Assumptions

- Staging and production sit behind two proxies (the VPS and Traefik); the trusted hop count is set per environment.
