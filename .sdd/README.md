# SDD — Spec Driven Development

Reusable structure for developing with AI agents from an agreed specification,
not from improvised prompts. It combines the best of Spec Kit (GitHub) and hello-sdd
(Brais Moure).

## Workflow

```
/sdd-constitution → /sdd-specify → /sdd-clarify → /sdd-plan → /sdd-tasks → /sdd-analyze
       → /sdd-implement Tn (one task, tests first, stop) ×N → /sdd-validate
       → /sdd-change (if a new requirement appears: spec first, then code)
```

| Command | What it does | Produces |
|---|---|---|
| `/sdd-constitution` | Proposes 6-8 non-negotiable principles and waits for approval | `docs/constitution.md` |
| `/sdd-specify <idea>` | Interview (max. 6 questions, one at a time) and writes the spec in EARS | `specs/NNN-name/spec.md` |
| `/sdd-clarify [NNN]` | QA review: ambiguities, contradictions, gaps. Max. 5 questions | `## Clarifications` section in the spec |
| `/sdd-plan [NNN]` | Technical plan with constitution check and justified decisions | `plan.md`, `research.md` if needed |
| `/sdd-tasks [NNN]` | Tasks < 30 min, ordered, with FR and "Done when" | `tasks.md` |
| `/sdd-analyze [NNN]` | Spec/plan/tasks consistency, coverage, severities. Read-only | on-screen report |
| `/sdd-implement [Tn]` | Implements ONLY that task, tests first, and stops | code + `[x]` in tasks.md |
| `/sdd-validate [NNN]` | Walks the spec FR by FR: covering test and result | `validation.md` |
| `/sdd-change <req>` | New requirement: updates the spec, shows the diff, does not touch code | updated spec |
| `/sdd-status` | Active spec, tasks done/pending, open questions | on-screen summary |

## Rules

1. The spec rules. If the code and the spec disagree, fix the spec first or the code afterwards, never ignore it.
2. Only WHAT and WHY in the spec. The HOW goes in the plan.
3. Requirements in EARS notation, each one testable with a single measurable criterion.
4. Every doubt is marked `[NEEDS CLARIFICATION: question]`. Never guess silently.
5. One task per cycle. Tests before implementation. When done, stop.
6. "Out of scope" is mandatory in every spec.
7. Every technical decision in the plan carries the discarded alternative and why.
8. English only. All artifacts and all communication with the user are in English, even when the user writes in another language.

## Git strategy

1. `main` only contains what is stable: original delivery, infrastructure, SDD template, constitution and
   validated specs. Never develop directly on `main`.
2. One branch per spec, with the same name as its folder: `NNN-name`. `/sdd-specify` creates it. Spec, plan,
   tasks, code and validation live in it.
3. One commit per task, with message `Tnn: description`. Spec, plan and tasks are committed when approved.
   No commit without the user's explicit OK.
4. Merge into `main` without squash when `/sdd-validate` gives a positive verdict, to preserve the
   commit → task → FR → spec traceability.

## Structure

```
.claude/commands/sdd-*.md   slash commands (this workflow)
.sdd/templates/             templates for constitution, spec, plan, tasks, validation
docs/constitution.md        project principles
specs/NNN-name/             spec.md, plan.md, tasks.md, research.md, validation.md
CLAUDE.md                   agent context: forces reading the constitution and the active spec
```

The active spec is the one with the highest number in `specs/`, unless the command receives another number.
