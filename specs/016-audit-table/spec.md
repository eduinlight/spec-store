# Spec 016 — Audit trail as a data table

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-10

## 1. Context and goal

Owners read a project's audit trail, and organization owners their organization's, as pages of 50 entries
(spec 002, spec 005) with the client, origin and result columns added by spec 010. The SDD Vault design
(FUNCTIONALITY §3.10) turns it into a data table: text filter, faceted filters with counts (Actor, Via, Action,
Result, Period), column visibility, row and page selection, sortable time, rows per page, pagination, and an
export of the selected or filtered entries. Security reviewers and owners answer "who did what, through which
agent, from where" quickly and take the evidence with them.

## 4. User stories

### US-1 (P1) — Find entries
As an owner I want to filter the audit trail by text, actor, client, action, result and period, with counts, and sort by time.
### US-2 (P1) — Take evidence
As an owner I want to export the selected entries, or every entry matching the filters, as CSV.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL filter a project's or organization's audit entries by a text that matches target, client or origin, and by any combination of actors, clients, actions, results and a period (last 24 hours, 7 days, 30 days, all), and SHALL return with each page the number of entries per value of each facet under the other filters.
- **FR-2** (US-1): THE SYSTEM SHALL sort the entries by time, newest or oldest first, and page them with 10, 25, 50 or 100 rows per page, returning the total number of matching entries.
- **FR-3** (US-1): THE SYSTEM SHALL show the entries in a table with time, actor, via (client), action, target, result and origin, letting the person hide Via, Target and Origin, select rows and all rows of the page, and show "N of M row(s) selected", "Page X of Y" and the pagination controls; result pills OK, Denied and Conflict.
- **FR-4** (US-2): WHEN an owner exports, THE SYSTEM SHALL download a CSV of the selected entries, or of every entry matching the filters when none is selected, with columns time, actor, client, action, target, result, detail, origin, and SHALL record the export in the audit trail.
- **FR-5**: THE SYSTEM SHALL keep audit access as before: project owners for their project, organization owners for their organization, operators for everything; everyone else gets the authorization error.
- **FR-6**: THE SYSTEM SHALL show every new text in English and Spanish.

## 6. Non-functional requirements

- **NFR-1**: A page with facet counts over 100,000 audit entries of a project answers in under 500 ms at p95 on the reference machine.

## 9. Out of scope

- SIEM streaming (SV-19 / enterprise); the global operator audit keeps its current page.
