# Spec 014 — Overview, active specs and live activity

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-8

## 1. Context and goal

A project's page lists its specs and members (spec 002). The SDD Vault design (FUNCTIONALITY §3.3, §3.4, §3.8,
product-spec §11.2.2) makes the Overview the main dashboard (KPIs for a period, tasks completed by agent, what
needs attention, active specs, recent activity), adds an Active specs page (the active spec with its next task
and a ready-to-paste agent prompt, drafts with their open questions) and an Activity page with a live feed. The
feed must update while people watch, across every server instance, so this spec also introduces the platform's
live channel: WebSockets fed by an event bus.

## 2. Users / actors

- Project member: follows progress and activity.
- Agent: its writes appear live.

## 4. User stories

### US-1 (P1) — Overview
As a member I want KPIs, a chart of tasks completed by agent and a list of what needs my attention for 7, 14 or 30 days.
### US-2 (P1) — Active specs
As a member I want the active spec's next task with its done-when and a prompt I can paste into any agent.
### US-3 (P1) — Live activity
As a member I want a feed of what people and agents change, including refused writes, that updates without reloading.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL show for a chosen period of 7, 14 or 30 days: active specs (non-archived specs not yet approved-and-complete), tasks completed in the period with the change against the previous period, requirement coverage of the active spec, open markers of the project, and agents active in the last 15 minutes (distinct MCP clients with writes).
- **FR-2** (US-1): THE SYSTEM SHALL show the tasks completed per day of the period stacked by client.
- **FR-3** (US-1): THE SYSTEM SHALL list what needs the person's attention: specs waiting for approval (owners, status clarified), open questions per spec, requirements of the active spec without work, each with an action that opens the right screen.
- **FR-4** (US-2): THE SYSTEM SHALL show the active spec (status, branch, progress, phase, open markers) with its next pending task (id, parallel flag, requirements, description, done-when) and a "Copy agent prompt" action that copies an instruction naming the project, the spec and the task; and each draft spec with its summary and open questions.
- **FR-5** (US-3): THE SYSTEM SHALL list the project's activity newest first (person, client, action in words, target, result, time), filterable by All, Tasks, Documents and Security (denied writes, member and access changes), showing a conflict followed by a successful write of the same document by the same person as "rejected, re-read and saved".
- **FR-6** (US-3): WHEN a write to a project succeeds or is refused on any server instance, THE SYSTEM SHALL deliver it within 2 seconds to every open Activity and Overview page of that project's members, over a WebSocket, without polling.
- **FR-7** (US-3): IF the live connection drops, THEN THE SYSTEM SHALL reconnect with backoff and reload what was missed; non-members never receive another project's events.
- **FR-8**: THE SYSTEM SHALL show every new text in English and Spanish.

## 6. Non-functional requirements

- **NFR-1**: One server instance holds at least 1,000 open live connections; events reach every instance through the event bus.
- **NFR-2**: A failure of the event bus never fails a write; it only delays live updates until reconnection.

## 9. Out of scope

- Presence of agents working right now and task claiming (SV-13).
- Notifications (SV-15).
