# Spec 015 — Project members, roles and guests

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-9

## 1. Context and goal

Project owners set and remove members in project settings, and only people who already belong to the
organization can join a project; invitations exist only for the organization. The SDD Vault design
(FUNCTIONALITY §3.9, product-spec §7.1) gives each project a Members screen (table with role dropdown, joined,
last active, guest tag, remove with confirmation) and an invite dialog where people outside the organization
join that one project as guests, without seeing the rest of the organization. Contractors, clients and
reviewers can then work on one project.

## 2. Users / actors

- Project owner: invites, changes roles, removes.
- Guest: a person with a role in one or more projects of an organization they do not belong to.
- Project member: sees who is in the project.

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| `memberships(project, user, role)` | Project roles (spec 001) | keep | Guests are memberships without an organization membership |
| `set_member` requires an organization member | Spec 005 | change | Guests come only through accepted project invitations |
| `invitations` (organization only) | Spec 005 FR-2 | extend | Optional project and project role |

## 4. User stories

### US-1 (P1) — Members screen
As a member I want to see the project's members with role, guest tag, joined and last active.
### US-2 (P1) — Invite to a project
As an owner I want to invite an email with a role; organization members get the role at once, other people get an invitation and join as guests.
### US-3 (P1) — Guests see only their projects
As an organization I want guests to see only the projects they were invited to.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL list a project's members with handle, name, role, guest flag (no organization membership), joined date and last active time (latest audit entry of that person in the project), owners first.
- **FR-2** (US-1): WHEN an owner changes a member's role or removes a member, THE SYSTEM SHALL apply it with an audit entry after a confirmation for removals; IF the change would leave the project without an owner, THEN THE SYSTEM SHALL refuse it with a message naming the rule.
- **FR-3** (US-2): WHEN an owner invites an email that belongs to a member of the organization, THE SYSTEM SHALL give that person the chosen project role at once with an audit entry.
- **FR-4** (US-2): WHEN an owner invites any other email, THE SYSTEM SHALL record a pending project invitation (role, project) that expires after 7 days and email an accept link naming the project and the inviter; accepting it gives the project role without an organization membership (guest).
- **FR-5** (US-2): THE SYSTEM SHALL let owners list, resend and revoke a project's pending invitations.
- **FR-6** (US-3): WHILE a person is a guest of an organization, THE SYSTEM SHALL show them only the projects they belong to, the organization's name and slug, and nothing else of the organization (members, invitations, audit, other projects), in the app, the API, the MCP server and the CLI.
- **FR-7** (US-3): WHEN an organization owner adds a guest to the organization, THE SYSTEM SHALL keep their project roles and remove the guest flag.
- **FR-8**: THE SYSTEM SHALL show every new text in English and Spanish.

## 8. Edge cases

- Inviting an email that is already a project member: refused with a message.
- A guest removed from their last project of an organization no longer sees the organization.
- Accepting a project invitation with a different verified email: refused (as spec 005).

## 9. Out of scope

- Verified organization domains and SSO (SV-19); "readers are free" billing (SV-20) is shown as a note only.
