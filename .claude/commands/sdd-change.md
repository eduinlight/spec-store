---
description: New requirement or change to the active spec. Updates ONLY the spec, shows the diff and points out which parts of the plan and tasks become outdated. Does not touch code
argument-hint: <description of the change> [NNN]
---

The spec changes before the code. Never the other way round.

Language: communicate and write everything in English, even if the user writes in another language.

Requested change: $ARGUMENTS
Spec: the one given by number or, otherwise, the one with the highest number in `specs/`.

1. Read `docs/constitution.md` and `specs/NNN-*/{spec,plan,tasks}.md`.
2. If the change contradicts the constitution, say so and stop; the constitution is changed with `/sdd-constitution`, not here.
3. Decide whether the change fits in this spec or deserves a new one (`/sdd-specify`). Rule: if it touches existing USs or their FRs, it goes here; if it is a new independent capability, new spec. Explain the decision.
4. Update `spec.md`: new or modified FRs in EARS (keep existing numbers; new ones continue the sequence; removed ones are marked `[REMOVED: reason]` instead of deleted), edge cases, out of scope, and an entry in `## Clarifications` with the date and reason for the change. Status goes back to "draft" if there are new open questions.
5. Show the spec diff.
6. List which sections of `plan.md` and which tasks in `tasks.md` are affected, without modifying them. Give the command to regenerate them (`/sdd-plan NNN`, `/sdd-tasks NNN`) and whether `/sdd-clarify NNN` should be run first.
7. Do not write or modify code.
