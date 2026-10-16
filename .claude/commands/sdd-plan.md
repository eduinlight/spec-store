---
description: Generates the technical plan for the active spec: context, constitution check, modules, data, contracts, justified decisions and test strategy. No code
argument-hint: [NNN]
---

You are going to translate the spec into technical decisions. Do not write production code.

Language: communicate and write everything in English, even if the user writes in another language.

Spec: the one given in `$ARGUMENTS` or, if empty, the one with the highest number in `specs/`.

1. Read `docs/constitution.md`, `specs/NNN-*/spec.md` and `.sdd/templates/plan.md`. If the spec has `[NEEDS CLARIFICATION]` markers in FRs or edge cases, stop and ask to run `/sdd-clarify NNN` first.
2. Read the existing code that will be affected. Do not plan on assumptions about how it is built.
3. Fill in "Technical context". Every unknown is marked `[NEEDS RESEARCH]`.
4. Research: resolve each unknown by consulting official documentation, reference source code or the project itself. Write `specs/NNN-*/research.md` with: question, finding, source (URL or path), decision. If the constitution names mandatory reference sources (versions, documentation), use them and cite them.
5. Constitution check (gate): table, principle by principle. If something does not comply, either it is justified in "Deviations" with the rejected simple alternative, or you stop and consult the user.
6. Write the rest of the plan: file structure with each file's responsibility, data model, contracts (input/output/errors), technical decisions ALWAYS with the discarded alternative and the reason, test strategy and coverage table FR → module → test. No FR may be left without a row.
7. Run the constitution check again on the final design.
8. Write `specs/NNN-*/plan.md`. Show a summary: key decisions, deviations, FRs without coverage (must be zero) and risks.
9. Next step: `/sdd-tasks NNN`.
