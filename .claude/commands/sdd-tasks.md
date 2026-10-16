---
description: Breaks the plan down into tasks < 30 min, ordered by dependency, with associated FR, parallelism marker and "Done when" criterion
argument-hint: [NNN]
---

You are going to generate the list of executable tasks.

Language: communicate and write everything in English, even if the user writes in another language.

Spec: the one given in `$ARGUMENTS` or, if empty, the one with the highest number in `specs/`.

1. Read `specs/NNN-*/spec.md`, `plan.md`, `research.md` if it exists, and `.sdd/templates/tasks.md`. If there is no plan, stop and ask for `/sdd-plan NNN`.
2. Organize by phases: Setup → Foundation (what blocks all stories) → one phase per US in priority order → Polish and cross-cutting. Each US must be deliverable and verifiable on its own when its phase closes.
3. Strict format per line:
   `- [ ] Tnn [P] (FR-x, FR-y) Description with file path — Done when: verifiable criterion`
   - `Tnn` sequential with two digits.
   - `[P]` only if the task shares no files with and does not depend on another task of the same phase.
   - `(FR: —)` for infrastructure without an FR.
   - "Done when" is a single line, objective, normally "tests X green" or a reproducible check. "Works correctly" is forbidden.
4. Within each story: first the task of writing tests (done when: written and red), then the implementation (done when: those tests green). Models before services, services before interface.
5. Tasks under 30 minutes. If one does not fit, split it.
6. Explicit "Dependencies" section and "Coverage" table FR → tasks. No FR without a task, no task without an FR except infrastructure.
7. Write `specs/NNN-*/tasks.md`. Show: total tasks, tasks per phase, how many are `[P]`, and which phase constitutes the MVP.
8. Next step: `/sdd-analyze NNN` and then `/sdd-implement T01`.
