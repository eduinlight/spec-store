# Spec NNN — <Name>

Status: draft | clarified | approved · Created: <YYYY-MM-DD> · Constitution: v<X.Y>

## 1. Context and goal

<One paragraph: what problem it solves and what value it brings. Only WHAT and WHY.>

## 2. Users / actors

- <Role>: <what they need>

## 3. Current state (if legacy code or behaviour exists)

<What the system does today, why it was done that way, and what of it still makes sense.
Link to the code or documentation that justifies it. Omit in new projects.>

| Current piece | What it existed for | Decision | Justification (doc/code) |
|---|---|---|---|
| | | keep / replace / remove | |

## 4. User stories (by priority)

### US-1 (P1) — <title>
As a <role> I want <action> so that <benefit>.
Independent test: <how this story is verified on its own>.

## 5. Functional requirements (EARS)

Patterns: `THE SYSTEM SHALL <behaviour>` · `WHEN <event>, THE SYSTEM SHALL <response>` ·
`IF <condition>, THEN THE SYSTEM SHALL <response>` · `WHILE <state>, THE SYSTEM SHALL <response>` ·
`WHERE <active option>, THE SYSTEM SHALL <response>`.

- **FR-1** (US-1): WHEN ..., THE SYSTEM SHALL ...
- **FR-2** (US-1): IF ..., THEN THE SYSTEM SHALL ...

## 6. Non-functional requirements

- **NFR-1**: <performance, security, platforms, language... with a measurable threshold>

## 7. Key entities (if applicable)

- <Entity>: <what it represents, relevant attributes, relationships>. No schema details.

## 8. Edge cases

- <Empty data, duplicates, concurrency, external failures, permissions...>

## 9. Out of scope

- <What is explicitly NOT done in this iteration>

## 10. Completion criteria

- Every FR covered by at least one green test.
- <Concrete manual validation>

## 11. Assumptions

- <Decisions taken by default in the absence of information>

## 12. Open questions

- [NEEDS CLARIFICATION: <specific question>]

## Clarifications

<Added by /sdd-clarify: date, question, answer, affected section.>
