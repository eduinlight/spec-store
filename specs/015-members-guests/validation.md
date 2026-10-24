# Validation — Spec 015

Spec: `specs/015-members-guests/spec.md` · Date: 2026-10-24 · Ticket: SV-9

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | Core `project_members.rs` (role, guest flag, joined, last active, owners first, organization owners listed as owners from the organization); API `http_projects.rs`, `http_project_members.rs`; app `project-members.test.tsx` | pass |
| FR-2 | Core last-owner rule; API refusal; app role select, removal with confirmation and toast, server refusal shown as an error toast | pass |
| FR-3 | Core and API: inviting an organization member gives the role at once (`{added}`) | pass |
| FR-4 | Core and API: other emails get a pending invitation (7 days) and an email naming the project, inviter and role (Mailpit); accepting creates a guest membership; e2e `members.spec.ts` registers the invited person from the email and accepts | pass |
| FR-5 | Core and API list, resend, revoke; app pending invitations with Resend and Revoke | pass |
| FR-6 | Core `guests.rs`, API `http_guests.rs`, MCP `mcp_tenancy.rs`, CLI `cli_commands.rs`: guests see only their projects and the organization's name; members, invitations, audit and other projects not found; e2e: the guest sees one project and the Guest tag | pass |
| FR-7 | Core: a guest added to the organization keeps their project roles | pass |
| FR-8 | English and Spanish keys | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint`, `make test` | see the ticket |
| Playwright | see the ticket |

## Defects found and fixed

- A new project's Members table was empty: its creator owns it as an organization owner, with no membership row. Organization owners are now listed as owners "from the organization", without role or remove controls.

## Decisions

- Without verified organization domains (SV-19), the guest rule is: organization members invited to a project get the role at once; everyone else joins as a guest through an invitation.
- The last-owner rule counts project memberships; organization owners keep owning every project of their organization.

## Verdict

Local: pass. Staging: pending the user's check (invitation email through Amazon SES).
