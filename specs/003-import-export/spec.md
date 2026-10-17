# Spec 003 — Import and Export

Status: approved · Created: 2026-10-17 · Constitution: v1.0

## 1. Context and goal

Projects that already follow SDD keep their artifacts as files: `docs/constitution.md` and `specs/NNN-name/{spec,plan,tasks,research,validation}.md`. Spec 001 gave the store an agent interface and spec 002 a browser view, but there is no way to bring an existing tree into a project or to get the documents back out as files. Without import, every team must recreate its history by hand before switching; without export, nobody can leave the store, keep an offline copy or hand the documents to a tool that reads files. This spec adds an administrator-side import of a project directory into a store project, preserving spec numbers and the exact bytes of every document, and an export that writes the current documents back to the same layout, so the two operations round-trip.

## 2. Users / actors

- Project owner: migrates an existing repository's SDD tree into the project they own.
- Project member (owner, editor, reader) or server admin: exports a project, or one spec, to files for backup, review or migration.

## 3. Current state

| Current piece | What it existed for | Decision | Justification (doc/code) |
|---|---|---|---|
| Administration CLI acting on the database file with `--as HANDLE` (`src/cli.rs`) | User, token, project, member and audit administration on the server host | keep | Import and export follow the same model: they run on the host that holds the database file, act as a named user and obey that user's project role |
| `create_spec` assigns the next number (`src/store/specs.rs`) | Agents never pick numbers | keep | Import is the only path that sets a number explicitly, because existing numbers are referenced by branches and commits; later specs still get highest + 1 |
| Status changes follow the transition table (`src/model.rs::transition_allowed`) | Agents move specs through the workflow step by step | keep | Import restores a status that already exists in the files, so it sets it directly instead of walking transitions |
| Constitution versions are given by the writer (`put_constitution`) | The agent states the version when it amends | keep | Import reads the version from the file's `Version` line or from an explicit option |

## 4. User stories (by priority)

### US-1 (P1) — Import an existing tree
As a project owner I want to import my repository's `docs/constitution.md` and `specs/` folders into an empty store project so that the store becomes the source of truth without retyping anything.
Independent test: import this repository's own `docs/` and `specs/` into a new project and read every document back with the same bytes, numbers, slugs and statuses.

### US-2 (P1) — Export a project to files
As a project member I want to write the project's current documents to a directory in the usual layout so that I keep an offline copy or move the project elsewhere.
Independent test: export a project with a constitution and two specs into an empty directory and compare every file with the stored current revision.

### US-3 (P2) — Round trip
As a project owner I want an import followed by an export to reproduce my files exactly so that I can trust the migration.
Independent test: import this repository's tree, export it to an empty directory and compare both trees file by file.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): WHEN a project owner imports a directory into a project, THE SYSTEM SHALL create one spec for every folder `specs/<name>/` whose name equals the branch name the store derives from a number and a valid slug, with that number and that slug.
- **FR-2** (US-1): WHEN a spec folder contains `spec.md`, `plan.md`, `tasks.md`, `research.md` or `validation.md`, THE SYSTEM SHALL store each one as the document of that kind whose current revision body is byte-identical to the file.
- **FR-3** (US-1): WHERE the directory contains `docs/constitution.md`, THE SYSTEM SHALL store it as the project constitution with a byte-identical body and the version given by `--constitution-version` or, without that option, by the first line that starts with `Version ` in the file.
- **FR-4** (US-1): IF an imported constitution has no `Version ` line and no `--constitution-version` is given, THEN THE SYSTEM SHALL refuse the import naming the file.
- **FR-5** (US-1): WHEN a spec is imported, THE SYSTEM SHALL set its status to the first word after `Status:` on the first such line of its `spec.md` when that word is `draft`, `clarified`, `approved` or `archived`, and otherwise to `draft` with a notice on the error stream naming the spec.
- **FR-6** (US-1): IF the import finds any problem among: a folder in `specs/` whose name is not a valid branch name, a number or slug already used in the project, two folders with the same number or slug, a file that is not valid UTF-8, a file larger than the document size limit, a constitution in the directory while the project already has one, or neither a constitution nor any spec folder, THEN THE SYSTEM SHALL refuse the import, list every problem found on the error stream and leave the store unchanged.
- **FR-7** (US-1): IF the import fails after it started writing, THEN THE SYSTEM SHALL leave the store unchanged.
- **FR-8** (US-1): WHEN the import meets a file in `specs/` that is not a folder, or a file or folder inside a spec folder that is not one of the five document files, THE SYSTEM SHALL skip it and name it on the error stream without failing the import.
- **FR-9** (US-1): WHEN a document is imported, THE SYSTEM SHALL record one revision authored by the acting user with the message `import <path relative to the directory>` and the audit entries `spec.import` for each spec, `document.put` for each spec document and `constitution.put` for the constitution.
- **FR-10** (US-1): IF the acting user is not an owner of the project, THEN THE SYSTEM SHALL refuse the import and leave the store unchanged.
- **FR-11** (US-1): WHERE `--dry-run` is given, THE SYSTEM SHALL perform every check, print the same report as a real import and leave the store unchanged.
- **FR-12** (US-1): WHEN an import succeeds, THE SYSTEM SHALL print one line for the constitution with its version, one line per spec with its branch name, status and imported kinds, and a final line with the number of specs and documents imported.
- **FR-13** (US-1): WHEN a spec is created after an import, THE SYSTEM SHALL give it the number one above the highest number in the project.
- **FR-14** (US-2): WHEN a member of the project or an admin exports a project to a directory, THE SYSTEM SHALL write `docs/constitution.md` and `specs/<branch name>/<kind>.md` for the constitution and every existing spec document, each byte-identical to its current revision, creating missing folders.
- **FR-15** (US-2): WHERE `--spec <number>` is given, THE SYSTEM SHALL export only the documents of that spec, and IF that spec does not exist, THEN THE SYSTEM SHALL refuse the export.
- **FR-16** (US-2): IF a target file exists with content different from the document and `--force` is not given, THEN THE SYSTEM SHALL refuse the export, list every such file on the error stream and write no file.
- **FR-17** (US-2): WHEN a target file already holds exactly the document's bytes, THE SYSTEM SHALL leave it untouched and report it as unchanged; WHERE `--force` is given, THE SYSTEM SHALL overwrite differing files; THE SYSTEM SHALL never delete a file in the directory.
- **FR-18** (US-2): IF the status of an exported spec differs from the `Status:` word of its stored `spec.md`, THEN THE SYSTEM SHALL write the stored bytes unchanged and print a notice on the error stream naming the spec and both statuses.
- **FR-19** (US-2): IF the acting user is neither a member of the project nor an admin, THEN THE SYSTEM SHALL refuse the export and write no file.
- **FR-20** (US-2): WHEN an export succeeds, THE SYSTEM SHALL print one line per file with `written` or `unchanged` and a final line with the counts, and SHALL create no revision and no audit entry.
- **FR-21** (US-3): WHEN a directory is imported into an empty project and the project is exported to an empty directory, THE SYSTEM SHALL produce every imported file with identical bytes at the same relative path.

## 6. Non-functional requirements

- **NFR-1**: Importing 100 specs with five 20 KB documents each completes in under 10 seconds with a release build on the development machine.
- **NFR-2**: Import and export keep the command-line contract of spec 001: exit code 0 on success, 1 on failure with nothing on standard output and an explanation on the error stream.
- **NFR-3**: Export writes only inside the target directory; no stored name can produce a path outside it.

## 7. Key entities (if applicable)

- Project directory: a folder holding an optional `docs/constitution.md` and an optional `specs/` folder with one folder per spec.
- Import report: the list of documents that will be or were imported, skipped entries, status fallbacks and problems.

## 8. Edge cases

- Empty `specs/` folder and a constitution: only the constitution is imported.
- Spec folder with no document files: the spec is created with no documents and reported with an empty kind list.
- Folder names like `0001-name`, `01-name`, `001-Name` or `001-a-b-c-d-e`: invalid branch names, the import is refused.
- Byte order mark, CRLF line endings, trailing whitespace and missing final newline: kept byte for byte.
- A file of exactly the document size limit is accepted; one byte more is refused.
- Malformed task lines in `tasks.md`: stored as written; the index reports them as malformed, as for any other write.
- Re-running the same import: refused because the numbers already exist; nothing changes.
- Exporting a project with no constitution and no specs: succeeds with zero files.
- Export into a directory that does not exist: it is created.
- Archived specs: exported like any other.
- Symbolic links inside the directory: followed as the file system reports them; no special treatment.

## 9. Out of scope

- Import or export through the HTTP server or the MCP interface; both run on the host that holds the database file.
- Importing git history as revisions, or exporting the revision history; only current bodies move.
- Incremental import into a project that already has the same spec numbers, merging documents, or updating existing specs from files.
- Deleting files on export or mirroring a directory.
- Templates, `.sdd/` configuration, `CLAUDE.md` and any spec subfolders such as `contracts/`.
- Machine-readable output formats.

## 10. Completion criteria

- Every FR covered by at least one green test.
- This repository's own `docs/constitution.md` and `specs/` imported into a scratch project and exported to an empty directory, with `diff -r` reporting no difference for the imported files.

## 11. Assumptions

- The CLI is the only interface, as for the other administration commands, and it acts on the local database file with `--as HANDLE`.
- Import needs the owner role because the constitution can only be written by owners; an admin who is not an owner makes themself owner first with `member set`.
- Export is allowed to every member and to admins, the same audience that can read the documents today.
- A spec folder name is valid when it equals the branch name derived from its number and slug (three digits with leading zeros up to 999, then plain digits), so export reproduces the same folder.
- Status and constitution version are read from the existing template lines `Status: ...` and `Version ...`; nothing in the bodies is rewritten on import or export.
- Unknown entries are skipped with a notice instead of failing, because spec-kit style trees carry extra folders such as `contracts/`.

## 12. Open questions

None.

## Clarifications

- 2026-10-17 · Which feature for spec 003? · CLI import and export of a `specs/` tree, the next candidate named in specs 001 and 002; rewriting the SDD commands on top of the store stays for a later spec. Decided during the autonomous run requested with "continue until merged with main". Affected: whole spec.
- 2026-10-17 · What happens on a conflict with existing data? · The whole import is refused and nothing is written; there is no merge. Affected: FR-6, FR-7, Out of scope.
- 2026-10-17 · Does export overwrite files? · Only with `--force`; identical files are left untouched and nothing is ever deleted. Affected: FR-16, FR-17.
- 2026-10-17 · What if the stored status and the `Status:` line disagree on export? · Bytes stay exact and a notice is printed. Affected: FR-18.
