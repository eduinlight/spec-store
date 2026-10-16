---
description: Consistency analysis across the spec, plan and tasks of the active spec. Read-only. Report with severities and coverage
argument-hint: [NNN]
---

Non-destructive analysis. Do not modify any file.

Language: communicate and write everything in English, even if the user writes in another language.

Spec: the one given in `$ARGUMENTS` or, if empty, the one with the highest number in `specs/`.

1. Read `docs/constitution.md` and `specs/NNN-*/{spec,plan,tasks}.md` (and `research.md` if it exists). If any is missing, say so and stop.
2. Check:
   - **Duplicates**: nearly identical FRs.
   - **Contradictions**: between FRs, between spec and plan, between plan and tasks, impossible task order.
   - **Terminology drift**: the same concept with different names across the three documents.
   - **Ambiguity**: adjectives without a threshold, `[NEEDS CLARIFICATION]` markers, `TODO`, `???`.
   - **Coverage**: FR without a task, task without an FR (except infrastructure), completion criteria with no task producing them, plan entities that are not in the spec or vice versa.
   - **Constitution**: principles with "must"/"never" violated by plan or tasks; mandatory sections or gates missing.
   - **Tasks**: no "Done when", no path, `[P]` tasks sharing a file, tests after the implementation.
3. Classify each finding:
   - **CRITICAL**: constitution violation, missing artifact, core FR without coverage.
   - **HIGH**: duplicated or contradictory FRs, ambiguous security/performance, non-testable criterion.
   - **MEDIUM**: terminology, NFR without a task, underspecified edge case.
   - **LOW**: wording, minor redundancy.
4. Deliver a report with: findings table (id, severity, location, description, suggestion), coverage table FR → plan → tasks, metrics (total FRs, covered, tasks, findings by severity).
5. If there are CRITICAL findings, recommend not implementing until they are resolved and say which command fixes each one (`/sdd-clarify`, `/sdd-plan`, `/sdd-tasks`, `/sdd-change`). Ask whether the user wants you to apply the suggested fixes.
