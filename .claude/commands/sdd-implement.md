---
description: Implements the tasks in tasks.md in order (the given one, or from the first pending one), tests first, suite at the end of each task, commit per task; stops only for manual validations, failures that change the plan, or decisions outside the spec
argument-hint: [Tnn] [NNN] [--single] [--delegate]
---

Chained implementation cycle. With `--single` it implements one task and stops; without it, it chains tasks until the current phase and all following ones are finished, stopping only when indicated below.

Language: communicate and write everything in English, even if the user writes in another language.

Arguments: `$ARGUMENTS` may contain the task id (Tnn) and/or the spec number (NNN). Without a task, start at the first `- [ ]` in tasks.md. Without a spec, the one with the highest number in `specs/`.

For each task:

1. Read `docs/constitution.md`, `CLAUDE.md`, `docs/coding-standards.md` if it exists, `specs/NNN-*/tasks.md`, `plan.md`, the FRs in `spec.md` that the task references, and `research.md`. If the task migrates existing behaviour, read the corresponding original code first and reproduce its behaviour except where the spec decides otherwise.
2. Verify that the tasks it depends on are marked `[x]`. If not, stop and say so.
3. If it is a test task: write only the tests, run them and confirm they are red for the expected reason. If it is an implementation task: write the minimum code that makes the referenced tests pass, following the plan and the standards. If the plan does not work as is, stop and explain before deviating.
4. Run the project suite (command in `CLAUDE.md`) and the lint. If something unrelated to the task fails, report it and do not hide it.
5. Check the "Done when" literally. Only if it is met, mark `[x]` in tasks.md and commit with the message `Tnn: description` (authorization for per-task commits is granted by the constitution when the branch's tasks are approved). Without a commit, do not move on to the next one.
6. Leave a short progress note when closing each phase: tasks done, tests, and what comes next.

Stop and wait for the user only when:
- the task is a manual validation that needs their browser, their credentials or their judgement;
- a task fails in a way that requires changing the plan or the spec (`/sdd-change`);
- a decision appears that the spec does not cover;
- all tasks are finished: then summarize and propose `/sdd-validate NNN`.

## Delegating to the implementer agent

If the user asked to delegate the implementation (or `$ARGUMENTS` includes `--delegate`), do not implement yourself: for each task launch the `sdd-implementer` agent (defined in `.claude/agents/`) with the assignment "Implement task Tnn of spec NNN" and wait for its report.

- If the report has no blockers and the task was marked and committed, check `git log -1` and move on to the next task.
- If the report has blockers, take over that task yourself in the main session with the full context: diagnose, fix or, if the plan or the spec needs to change, stop and consult the user.
- Manual validations are never delegated.
- You give the progress notes to the user, summarizing the reports.
