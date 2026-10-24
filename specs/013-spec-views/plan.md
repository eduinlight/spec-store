# Technical plan — Spec 013

Spec: `specs/013-spec-views/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

Core gains derived read models, all computed from the markdown and the revision history (principle 5): a
requirement parser (`FR-n` lines of the spec document), spec summaries for the list, a traceability matrix
(requirements × tasks × commits), a readiness checklist, task completion attribution (the revision that turned a
task done: author, client), and resolved markers (marker diff between consecutive revisions of a document). The
API exposes them; the app replaces the spec list, spec detail and constitution pages with the design's screens
built from `@workspace/ui/taptap` and `@workspace/ui/patterns` (SpecCard, Tabs, Steps, StatusTag, Tooltip, Table).

## 2. Technical context

Rust core (Diesel), axum + utoipa, React app. No new dependency.

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 4 | yes | Read models in `store/views/*.rs` (one module per view), pure parsers in `markdown/` |
| 5 | yes | Every index derives from markdown and revisions; nothing stored |
| 6 | yes | No schema change |

## 4. Files

```
packages/core/src/markdown/requirements.rs    parse FR-n lines (id, text) from a spec body
packages/core/src/store/views/spec_list.rs    spec summaries (title from the spec's first heading, summary line,
                                              created_at, updated_at, contributors, documents present, FR count,
                                              tasks done/total, open markers)
packages/core/src/store/views/traceability.rs requirement rows (tasks, done flags, commits, status), tasks without FRs
packages/core/src/store/views/readiness.rs    checklist items
packages/core/src/store/views/task_history.rs per task: completing revision author, client, time
packages/core/src/store/views/markers.rs      open and resolved markers (marker diff across revisions)
apps/api/src/routes/views.rs                  GET .../specs?view=cards, .../specs/{n}/traceability,
                                              .../specs/{n}/readiness, .../specs/{n}/markers, .../specs/{n}/tasks
apps/app/src/routes/specs.tsx                 specs list (cards, search, status chips, sort, load more)
apps/app/src/routes/spec.tsx                  header, lifecycle, tabs; document tab with ToC, FR tooltips, readiness
apps/app/src/routes/spec-tasks.tsx, spec-traceability.tsx, spec-markers.tsx, spec-history.tsx (tab panels)
apps/app/src/routes/constitution.tsx          document-style constitution
```

## 5. Decisions

- **Resolved markers by diff** of consecutive revisions; rejected: storing marker answers (would duplicate the
  markdown and need a writer).
- **Task completion attribution** from the revision whose task line changed from `[ ]` to `[x]`; rejected:
  reading audit targets (string parsing of `task.done` targets misses document edits).
- One request per tab, loaded when the tab opens.

## 6. Test strategy

Core unit tests for the parsers and each view on seeded specs; API tests for each endpoint's JSON and access
(non-members not found); app tests per screen; Playwright: open a spec, switch tabs, see traceability.
