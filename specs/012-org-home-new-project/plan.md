# Technical plan — Spec 012

Spec: `specs/012-org-home-new-project/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

Core gains a project summary query, project copy, and project archiving (migration 0003: `projects.archived_at`);
writes to archived projects return `Forbidden` (so spec 010 records them as denied). The API extends the
organization's project list with the summary fields and adds `POST .../copy`, `POST .../archive`,
`POST .../restore`. The app replaces the organization page with project cards and filters, the new-project page
with the design's dialog (folder import through `<input type="file" webkitdirectory>` building the same tree JSON
the CLI sends to the import endpoint), and shows the Get started checklist on the project Overview.

## 2. Technical context

Rust core + axum API (utoipa), React app with the TapTap components and patterns. No new dependency.

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 1, 3 | yes | Approved spec, tests first |
| 4 | yes | Summary as one query module (`store/summaries.rs`) returning a read model; no new abstraction |
| 6 | yes | Migration 0003; copy writes new revisions; archive is a flag with an audit entry, no delete |
| 7 | yes | Archived projects refuse writes for everyone |

## 4. Files

```
packages/core/migrations/0003_project_archive/up.sql
packages/core/src/store/summaries.rs        project_summaries(caller, org) -> Vec<ProjectSummary>
packages/core/src/store/projects.rs         copy_project, archive_project, restore_project; archived check in access_in writes
apps/api/src/routes/projects.rs             list with summaries, copy, archive, restore; make openapi
apps/app/src/routes/organization.tsx        cards (SpecCard-like project card), filters (Tabs type card), empty states
apps/app/src/routes/project-new.tsx         dialog-style form, slug preview, start options, folder import dry run
apps/app/src/routes/project.tsx             Get started checklist on Overview
apps/app/src/lib/sdd-tree.ts                FileList → import tree JSON (docs/constitution.md, specs/NNN-name/*.md)
```

## 5. Data

```sql
ALTER TABLE projects ADD COLUMN archived_at timestamptz;
```

## 6. Decisions

- **Folder import in the browser** with the existing endpoint; rejected: a zip upload parsed by the API (new
  dependency and a second import path).
- **Archived means read-only** through the same authorization point as roles; rejected: hiding archived projects
  from reads (history must stay readable).
- **Get started derived from data** (audit client, spec 001); rejected: a stored checklist table.

## 7. Test strategy

Core: summaries (role, active spec, markers, last activity), copy (revisions, audit), archive (writes refused
and recorded as denied, restore). API: list fields, copy/archive/restore authorization. App: cards and filters,
dialog (slug, preview, options, errors), folder-to-tree conversion, dry run problems, checklist states.
Playwright: create a project, see it in the cards, open Get started.
