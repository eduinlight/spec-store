---
description: Validates the active spec FR by FR: which test covers each one and its real result. Writes validation.md with a verdict
argument-hint: [NNN]
---

You are going to demonstrate, with evidence, whether the spec is fulfilled.

Language: communicate and write everything in English, even if the user writes in another language.

Spec: the one given in `$ARGUMENTS` or, if empty, the one with the highest number in `specs/`.

1. Read `specs/NNN-*/spec.md`, `tasks.md`, `.sdd/templates/validation.md` and the tests directory.
2. Run the project's full suite once (command in `CLAUDE.md`). Save the output.
3. Walk the spec FR by FR and the edge cases one by one. For each: covering test (path::name), result (✅ / ❌ / no test), notes. Do not count an FR as covered if the test does not exercise exactly that behaviour.
4. Add the manual validation required by "Completion criteria": steps, observed, result. If it requires the user to do it, leave the row marked as pending and say so.
5. Verdict: is the spec fulfilled? List of FRs uncovered or red. Each defect found is recorded with symptom, cause and proposed fix, and the task to add to tasks.md is suggested.
6. Write `specs/NNN-*/validation.md`. If the verdict is positive, change the spec status to "approved". Show the verdict and the pending tasks, if any.
7. With a positive verdict and the user's agreement, propose merging the branch `NNN-name` into `main` without squash (`git merge --no-ff`). Do not run it without an explicit OK.
