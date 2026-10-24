# Technical plan — Spec 015

Spec: `specs/015-members-guests/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

Migration 0004 adds `invitations.project_id` (nullable) and `invitations.project_role`. Core: a project member
read model (role, guest, joined from the membership's first audit entry or creation, last active); project
invitations (create: organization member → role at once, otherwise pending invitation + email through the
existing outbox; accept → membership without organization membership); visibility for guests (organization
reads restricted to name and slug; project lists limited to their memberships); last-owner rule. API:
`GET/POST .../members`, `PUT/DELETE .../members/{handle}`, `GET/POST .../invitations`, resend and revoke.
App: Members screen and invite dialog in the shell (replaces the placeholder of spec 011).

## 2. Decisions

- **Guest = membership without organization membership**; rejected: a guest role column (duplicates what the
  data already says).
- **Project invitations reuse the invitations table and accept flow**; rejected: a second table and token flow.
- Joined date: `memberships` gets `created_at` in migration 0004 (default now(); existing rows get the
  project's creation time).

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 6 | yes | Migration 0004; audit entries for every change |
| 7 | yes | Guests authorized only through their memberships |

## 4. Test strategy

Core: guest visibility (organization reads, project lists, audit), invitation paths, last-owner rule, last
active. API: endpoints and authorization (non-owners refused). MCP: a guest lists only their projects. App:
members table, role change, remove with confirmation, invite dialog both paths, pending invitations.
Playwright: owner invites an outside email, accepts as a new account through Mailpit, sees only that project.
