# spec-store

Remote, multi-project store for Spec Driven Development documents (constitution, spec, plan, tasks, research, validation) exposed to AI agents through an MCP server, so the documents no longer live in each project's repository. One Rust binary serves the Streamable HTTP MCP endpoint, a read-only web UI and a JSON API for the CLI, with a single SQLite file as the only state. Documents are markdown with append-only revisions; the server indexes FRs, tasks, status and markers for queries.

## Commands

- Run: `cargo run -- serve`
- Tests: `cargo test`
- Lint: `cargo clippy --all-targets -- -D warnings && cargo fmt --check`

## Methodology: Spec Driven Development

This project is developed with SDD. The workflow and commands are in `.sdd/README.md`.

Mandatory rules for the agent:

1. Read `docs/constitution.md` and the active spec (highest number in `specs/`) before touching code.
2. Do not implement anything that is not in an approved spec. If it is missing, `/sdd-change` or `/sdd-specify` first.
3. One task per cycle with `/sdd-implement`. Tests first. When done, stop and wait for "next".
4. Every doubt is asked or marked `[NEEDS CLARIFICATION]`. Never guess silently.
5. For any technical doubt, consult the official documentation or the reference source code before deciding, and cite the source in `research.md`.

## Language

1. Always communicate in English. Every reply, question, report, progress update and summary is written in English, even when the user writes in another language.
2. Every artifact is written in English: specs, plans, tasks, research, validation, constitution, documentation, code, identifiers and commit messages.
3. If the user writes in another language, understand the request and answer in English. Do not switch languages unless the user explicitly changes this rule in this file.

## Working rules

1. Never `git commit` or `git push` without an explicit OK for that commit. One authorization covers a single commit.
2. Do not add comments to the code. Use descriptive names; explanations go in the documentation or the commit message.
3. Defects are reported and fixed, with their test.
4. Clean repo: no "before/after" copies, no logs or temporary artifacts.
5. The full suite runs at the end of each task, not between every small change.
6. Explain each step calmly and move forward step by step.
7. When something specific is requested, do not jump to another task.

## Style

- Code, identifiers, commit messages, documentation and messages to the user in English.
- Rust stable toolchain, edition 2024, `rustfmt` defaults.
- Errors are typed with `thiserror` in libraries and `anyhow` only in the binary entry point.
- SQLite is the only persistent state; schema changes go through numbered migrations.
- Every write to a document creates a revision and an audit entry; nothing is updated in place.

## Checklist before considering a task finished

- [ ] The task's "Done when" is met literally
- [ ] Suite executed and result shown
- [ ] Task marked `[x]` in tasks.md
- [ ] No temporary files or added comments
