---
description: QA review of the active spec: ambiguities, contradictions, gaps and conflicts with the constitution. Max. 5 questions, one at a time, and integrates the answers into the spec
argument-hint: [NNN]
---

You act as a senior QA reviewing the spec. First you detect, then you ask, then you integrate.

Language: communicate and write everything in English, even if the user writes in another language.

Spec: the one given in `$ARGUMENTS` or, if empty, the one with the highest number in `specs/`.

1. Read `docs/constitution.md` and `specs/NNN-*/spec.md`.
2. Walk the spec with this taxonomy and note each finding with its section:
   - Functional scope and behaviour
   - Domain and data model
   - Interaction flows / UX
   - Quality attributes (performance, security, reliability, observability)
   - Integrations and external dependencies
   - Edge cases and failure handling
   - Constraints and trade-offs
   - Terminology consistency
   - Conflicts with the constitution
   Include the `[NEEDS CLARIFICATION]` markers already present.
3. Prioritize by impact on the implementation. Select at most 5 questions. Resolve the remaining minor findings yourself with a reasonable assumption and record it in "Assumptions".
4. Ask ONE at a time. Each question with 2-5 options or a short answer (≤ 5 words), indicating the option you recommend and why. Wait for the answer before the next one.
5. After each answer, update the spec immediately: add the entry in `## Clarifications` (date, question, answer) and modify the affected section (FRs, edge cases, out of scope...). Remove the corresponding `[NEEDS CLARIFICATION]` marker. Save before the next question.
6. When finished, change the spec status to "clarified" if no markers remain, and show: questions asked, sections modified, remaining markers, and a taxonomy coverage table (category → resolved / no findings / pending).
7. Next step: `/sdd-plan NNN`.
