---
description: Proposes the project constitution (6-8 non-negotiable principles) and waits for approval before writing it
argument-hint: [notes about stack, limits or principles that must appear]
---

You are going to draft the project constitution. It is a short, verifiable document, not a style guide.

Language: communicate and write everything in English, even if the user writes in another language.

1. Read `.sdd/templates/constitution.md` and, if they exist, `docs/constitution.md`, `CLAUDE.md`, `README*` and the project's manifests or dependency files to infer the real stack.
2. Propose between 6 and 8 principles, 25 lines at most in total. Each principle is one imperative, checkable sentence. Mandatory: the spec rules, tests as a gate, simplicity, language (English for all artifacts and all communication), what requires human approval. Add the domain ones that apply (reference versions, documentation that must be consulted, sources of truth).
3. Incorporate the user's notes: $ARGUMENTS
4. Show the full proposal on screen and ask whether it is approved or what should change. Do NOT write the file until you receive explicit approval.
5. With approval, write `docs/constitution.md` with version 1.0 and today's date. If it already existed, bump the version (major if a principle is removed or redefined, minor if one is added or refined) and update the amendment date.
6. Confirm the written path and remind the user that the next step is `/sdd-specify`.
