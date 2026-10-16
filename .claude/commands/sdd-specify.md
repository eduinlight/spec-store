---
description: Short interview (max. 6 questions, one at a time) and writes the spec of a feature in EARS notation. Only WHAT and WHY
argument-hint: <description of the feature or change>
---

You are going to create a specification. Do not write code or talk about implementation.

Language: communicate and write everything in English, even if the user writes in another language.

User input: $ARGUMENTS

1. Read `docs/constitution.md` (mandatory; if it does not exist, stop and ask to run `/sdd-constitution`), `.sdd/templates/spec.md` and the previous specs in `specs/` so as not to duplicate or contradict them.
2. If the project has existing code related to the feature, read it before asking. Note what it does today and why, for the "Current state" section.
3. Interview: at most 6 questions, ONE at a time, waiting for the answer. Only about what changes the outcome: scope, edge cases, errors, actors, what is left out. Offer closed options when possible and mark the one you recommend. Do not ask what you can infer from the code or the constitution.
4. Assign a number: the next one after the highest existing in `specs/` with format `NNN` (three digits, starting at 001). Short kebab-case name in English, 2-4 words. Create `specs/NNN-name/`.
   If the project is a git repository: check that the tree is clean (if not, stop and say so), and create and switch to the branch `NNN-name` from `main`. The spec and everything derived from it are worked on in that branch.
5. Write `specs/NNN-name/spec.md` following the full template:
   - Functional requirements in EARS, numbered FR-n, each one associated with a US and with a single measurable criterion. Subjective terms ("fast", "intuitive") without a threshold are forbidden.
   - "Out of scope" mandatory and concrete.
   - Every doubt you could not resolve: `[NEEDS CLARIFICATION: question]` in "Open questions". Never guess silently; if you take a default value, record it in "Assumptions".
   - If there is legacy behaviour, fill in the "Current state" table with the decision per piece (keep / replace / remove) and its justification with a link to code or documentation.
6. Self-review before delivering: does any FR talk about technology, files or algorithms? Move it or remove it. Is any FR not testable? Rewrite it. Does every US have at least one FR?
7. Show the spec and finish by indicating: path, number of FRs, number of open questions, and that the next step is `/sdd-clarify NNN`.
