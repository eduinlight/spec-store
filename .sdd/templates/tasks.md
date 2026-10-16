# Tasks — Spec NNN

Plan: `specs/NNN-name/plan.md` · Date: <YYYY-MM-DD>

Format: `- [ ] Tnn [P] (FR-x, FR-y) Description with path — Done when: verifiable criterion`
`[P]` = can run in parallel with other `[P]` tasks of the same phase (no shared files).
Each task < 30 minutes. Tests before implementation.

## Phase 1 — Setup
- [ ] T01 (FR: —) <skeleton, configuration> — Done when: <the suite runs with 0 tests and no errors>

## Phase 2 — Foundation (blocks the stories)
- [ ] T02 (FR-1) <model/entity> — Done when: <tests for ... are green>

## Phase 3 — US-1 (P1)
- [ ] T03 [P] (FR-1, FR-2) <tests for ...> — Done when: <tests written and red>
- [ ] T04 (FR-1, FR-2) <implementation> — Done when: <T03 green>

## Phase 4 — US-2 (P2)

## Final phase — Polish and cross-cutting
- [ ] Txx (NFR-1) <performance, docs, cleanup> — Done when: <...>

## Dependencies
- Phase 2 blocks all stories. Stories are independent of each other.
- <Tnn depends on Tmm>

## Coverage
| FR | Tasks |
|---|---|
