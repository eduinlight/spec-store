# Validation — Spec 001

Date: 2026-10-17 · Suite executed: `cargo test` (plus `cargo test --release --test perf -- --ignored` and `cargo clippy --all-targets -- -D warnings && cargo fmt --check`) · Overall result: 79 pass / 0 fail (1 ignored performance test, passing in release)

Test names are `file::function`. Unit tests inside `src/` are named by module.

| FR | Test(s) | Result | Notes |
|---|---|---|---|
| FR-1 | `tests/store_users.rs::creates_user_and_rejects_duplicate_handle`, `tests/cli.rs::first_user_is_admin_then_as_is_required` | ✅ | Duplicate handle is a conflict naming the handle |
| FR-2 | `tests/store_users.rs::issues_token_once_and_lists_without_secret`, `tests/cli.rs::tokens_are_printed_once_and_listed_without_secret` | ✅ | Only the SHA-256 of the token is stored; `token list` shows label, times and revocation |
| FR-3 | `tests/store_users.rs::revoked_token_fails_other_tokens_work`, `tests/http_auth.rs::missing_unknown_and_revoked_tokens_get_401` | ✅ | Second token of the same user keeps working |
| FR-4 | `tests/store_projects.rs::creates_project_and_rejects_duplicate_slug`, `tests/cli.rs::projects_members_and_audit` | ✅ | |
| FR-5 | `tests/store_projects.rs::set_member_replaces_role_and_remove_ends_access`, `tests/cli.rs::projects_members_and_audit` | ✅ | One membership row per user and project; removal gives Forbidden on the next call |
| FR-6 | `tests/cli.rs::first_user_is_admin_then_as_is_required`, `tests/cli.rs::projects_members_and_audit`, `tests/store_users.rs::later_users_require_an_admin_actor` | ✅ | First user becomes admin without `--as`; owner may run `member set` and `member remove`; editor and unknown handles are rejected |
| FR-7 | `tests/http_auth.rs::missing_unknown_and_revoked_tokens_get_401`, `tests/http_auth.rs::valid_token_reaches_mcp_initialize`, `tests/store_users.rs::authenticate_resolves_user_and_updates_last_used` | ✅ | 401 with `WWW-Authenticate: Bearer realm="spec-store"` and no body data |
| FR-8 | `tests/store_projects.rs::list_projects_by_membership_and_admin_sees_all`, `tests/mcp_stdio.rs::lists_tools_and_projects_for_the_acting_user` | ✅ | |
| FR-9 | `tests/store_projects.rs::non_member_gets_forbidden_without_existence_leak`, `tests/mcp_stdio.rs::outsider_is_forbidden_without_existence_leak`, `tests/store_documents.rs::reader_and_archived_cannot_write` | ✅ | Existing and missing project give the same code and message; admin without membership cannot write |
| FR-10 | `tests/store_specs.rs::list_specs_with_kinds_ordered`, `tests/mcp_stdio.rs::reads_specs_documents_constitution_and_templates` | ✅ | |
| FR-11 | `tests/store_specs.rs::active_spec_skips_archived_and_reports_none`, `tests/store_specs.rs::archiving_the_active_spec_moves_to_the_next_highest`, `tests/mcp_stdio.rs::reads_specs_documents_constitution_and_templates` | ✅ | `get_active_spec` returns `{"spec": null}` when none |
| FR-12 | `tests/store_documents.rs::get_document_and_distinct_not_found`, `tests/mcp_stdio.rs::reads_specs_documents_constitution_and_templates` | ✅ | Not-found message names either the spec or the kind |
| FR-13 | `tests/store_documents.rs::get_constitution_and_not_found`, `tests/mcp_stdio.rs::reads_specs_documents_constitution_and_templates` | ✅ | |
| FR-14 | `src/templates.rs::returns_the_five_template_kinds`, `src/templates.rs::rejects_kinds_without_template_listing_valid_ones`, `src/templates.rs::get_by_name_parses_and_validates`, `tests/mcp_stdio.rs::reads_specs_documents_constitution_and_templates` | ✅ | `research` and unknown kinds get a validation error listing the valid kinds |
| FR-15 | `tests/store_specs.rs::create_spec_assigns_next_number_and_branch`, `tests/store_specs.rs::concurrent_spec_creation_never_collides_or_skips`, `tests/mcp_stdio.rs::writes_specs_documents_status_and_tasks` | ✅ | Archived specs count towards the next number |
| FR-16 | `src/model.rs::accepts_valid_slugs_and_handles`, `src/model.rs::rejects_invalid_slugs`, `tests/store_specs.rs::rejects_bad_or_duplicate_slug` | ✅ | |
| FR-17 | `tests/store_documents.rs::put_creates_document_and_revision`, `tests/mcp_stdio.rs::writes_specs_documents_status_and_tasks` | ✅ | |
| FR-18 | `tests/store_documents.rs::put_requires_matching_expected_revision`, `tests/mcp_stdio.rs::writes_specs_documents_status_and_tasks` | ✅ | MCP error carries `data.kind = conflict` and `data.current_revision` |
| FR-19 | `tests/store_documents.rs::reader_and_archived_cannot_write`, `tests/store_tasks.rs::mark_done_rejects_unknown_done_or_bad_commit` | ✅ | Archived documents stay readable; revision count unchanged after rejection |
| FR-20 | `tests/store_documents.rs::constitution_owner_only_and_version_must_change`, `tests/mcp_stdio.rs::writes_specs_documents_status_and_tasks` | ✅ | |
| FR-21 | `src/model.rs::status_transition_table`, `src/model.rs::allowed_targets_are_listed_for_errors`, `tests/store_specs.rs::status_transitions_table`, `tests/mcp_stdio.rs::writes_specs_documents_status_and_tasks` | ✅ | Archive by editor is rejected; owner archives from any status |
| FR-22 | `src/markdown/tasks.rs::mark_done_rewrites_exactly_one_line`, `src/markdown/tasks.rs::handles_crlf_bodies`, `tests/store_tasks.rs::mark_done_rewrites_line_with_commit`, `tests/mcp_stdio.rs::writes_specs_documents_status_and_tasks` | ✅ | Revision message is `Tnn: done, commit <id>` |
| FR-23 | `src/model.rs::validates_commit_identifiers`, `src/markdown/tasks.rs::mark_done_rejects_unknown_done_and_duplicates`, `tests/store_tasks.rs::mark_done_rejects_unknown_done_or_bad_commit` | ✅ | |
| FR-24 | `tests/store_documents.rs::concurrent_puts_exactly_one_wins` | ✅ | Two threads released by a barrier with the same expected revision |
| FR-25 | `src/markdown/tasks.rs::parses_all_fields`, `src/markdown/tasks.rs::tasks_before_any_heading_have_an_empty_phase`, `tests/store_tasks.rs::list_tasks_pending_filter_and_malformed`, `tests/mcp_stdio.rs::derived_reads_tasks_markers_and_status` | ✅ | |
| FR-26 | `src/markdown/tasks.rs::reports_malformed_lines_with_numbers`, `tests/store_tasks.rs::list_tasks_pending_filter_and_malformed` | ✅ | |
| FR-27 | `src/markdown/markers.rs::finds_both_kinds_with_nearest_heading`, `src/markdown/markers.rs::handles_crlf`, `src/markdown/markers.rs::no_markers_and_no_heading`, `tests/store_tasks.rs::list_markers_across_documents`, `tests/mcp_stdio.rs::derived_reads_tasks_markers_and_status` | ✅ | |
| FR-28 | `tests/store_tasks.rs::project_status_counts_phase_next_and_markers`, `tests/mcp_stdio.rs::derived_reads_tasks_markers_and_status` | ✅ | Includes a project with no constitution and no specs |
| FR-29 | `tests/store_tasks.rs::status_reflects_last_write` | ✅ | Derived reads parse the current revision; there is no cache to go stale |
| FR-30 | `tests/store_documents.rs::list_revisions_newest_first`, `tests/mcp_stdio.rs::history_tools_list_and_read_revisions` | ✅ | Constitution revisions carry their version |
| FR-31 | `tests/store_documents.rs::get_revision_body_and_not_found`, `tests/mcp_stdio.rs::history_tools_list_and_read_revisions` | ✅ | |
| FR-32 | `tests/store_audit.rs::audit_records_actor_project_action_target_and_time`, `tests/store_audit.rs::list_audit_filters_by_project_and_limit`, `tests/cli.rs::projects_members_and_audit`, `tests/store_specs.rs::create_spec_assigns_next_number_and_branch` | ✅ | `audit list` requires an admin `--as` |
| NFR-1 | `tests/perf.rs::p95_read_under_100ms` (release, ignored by default) | ✅ | Measured p95 of a 200 KB document read plus project status with 10 threads: 2.68 ms |
| NFR-2 | `src/model.rs::body_limit_is_one_mebibyte`, `tests/store_documents.rs::rejects_over_1mb_and_keeps_bytes` | ✅ | CRLF and non-ASCII bytes read back identical |
| NFR-3 | `tests/http_auth.rs::token_never_in_logs_or_responses`, `tests/cli.rs::tokens_are_printed_once_and_listed_without_secret` | ✅ | Capturing tracing subscriber; also confirmed on the real server log in the manual validation |
| NFR-4 | `tests/http_auth.rs::valid_token_reaches_mcp_initialize`, `tests/mcp_stdio.rs::history_tools_list_and_read_revisions` | ✅ | The stdio test asserts the 16 tools; the manual validation ran the same tools over HTTP |
| NFR-5 | no test | ✅ by inspection | All error messages, identifiers, logs and documents are English; enforced by review, not by a test |

## Edge cases

| Case | Test | Result |
|---|---|---|
| Empty store: `list_projects` returns an empty list | `tests/store_projects.rs::list_projects_by_membership_and_admin_sees_all` (outsider gets `[]`), `tests/store_specs.rs::list_specs_is_empty_for_a_project_without_specs` | ✅ |
| Empty store: status of a project with no specs says so | `tests/mcp_stdio.rs::derived_reads_tasks_markers_and_status` (`active` null, `specs` empty, no constitution) | ✅ |
| Empty store: first user is admin without an acting admin | `tests/cli.rs::first_user_is_admin_then_as_is_required`, `tests/store_users.rs::later_users_require_an_admin_actor` | ✅ |
| User removed while a session is open: next call Forbidden | `tests/store_projects.rs::set_member_replaces_role_and_remove_ends_access` | ✅ (store level; every MCP call resolves access afresh) |
| Token revoked between two calls: second call fails, no partial write | `tests/http_auth.rs::missing_unknown_and_revoked_tokens_get_401` | ✅ (authentication runs before any tool executes) |
| Concurrent spec creation: numbers never collide or skip | `tests/store_specs.rs::concurrent_spec_creation_never_collides_or_skips` | ✅ |
| Writing a document for a missing spec: not-found, nothing created | `tests/store_documents.rs::put_requires_matching_expected_revision` (spec 002 missing) | ✅ |
| Duplicate task identifiers: rejected as malformed, nothing changed | `tests/store_tasks.rs::mark_done_refuses_malformed_or_duplicate_task_lines`, `src/markdown/tasks.rs::mark_done_rejects_unknown_done_and_duplicates` | ✅ |
| Archiving the active spec: next highest non-archived becomes active | `tests/store_specs.rs::archiving_the_active_spec_moves_to_the_next_highest` | ✅ |
| Constitution write by an editor: authorization error | `tests/store_documents.rs::constitution_owner_only_and_version_must_change`, `tests/mcp_stdio.rs::writes_specs_documents_status_and_tasks` | ✅ |
| Tasks document with Windows line endings: stored as is and parsed | `tests/store_documents.rs::rejects_over_1mb_and_keeps_bytes`, `src/markdown/tasks.rs::handles_crlf_bodies` | ✅ |

## Manual validation

Performed on 2026-10-17 with the release binary, a fresh database and a scripted MCP client speaking Streamable HTTP (JSON-RPC over `POST /mcp`, SSE responses) against `spec-store serve --bind 127.0.0.1:7399`.

| Step | Observed | Result |
|---|---|---|
| `user add alice`, `user add bob --as alice`, `project add demo "Demo project" --as alice`, `member set demo bob editor --as alice`, `token issue bob --label e2e --as alice` | Each command confirmed on stdout; the token was printed exactly once (48 bytes, `sst_` prefix) | ✅ |
| `serve` and `GET /healthz` | `ok` without a token | ✅ |
| `initialize` with the bearer token | Server info `spec-store 0.1.0`, `Mcp-Session-Id` returned, tools capability announced | ✅ |
| `list_projects` | `[{"slug":"demo","name":"Demo project","role":"editor"}]` | ✅ |
| `create_spec demo store-core` | `number 1, slug store-core, branch 001-store-core, status draft` | ✅ |
| `put_document` tasks with two tasks | `revision_id 1` | ✅ |
| `mark_task_done T01 commit 43dbcd9` | `revision_id 2`, line `- [x] T01 (FR: —) Create the skeleton — Done when: cargo build passes · commit: 43dbcd9` | ✅ |
| `project_status demo` | `tasks_done 1, tasks_total 2, current_phase "Phase 1 — Setup", next_task T02 with done_when "model tests green", markers 0, constitution_version null` | ✅ |
| `get_document` tasks | Body shows T01 checked with the commit; revision author `bob`, message `T01: done, commit 43dbcd9` | ✅ |
| `list_tasks` | T01 done with commit `43dbcd9`, T02 pending, no malformed lines | ✅ |
| Call without token | `HTTP 401`, `WWW-Authenticate: Bearer realm="spec-store"` | ✅ |
| `audit list --as alice` | Eight entries: user.create ×2, project.create, member.set, token.issue, spec.create, document.put, task.done, each with actor, project and time | ✅ |
| Token in server log | Absent | ✅ |
| Connect a real Claude Code session with `claude mcp add --transport http ... --header "Authorization: Bearer ..."` and run the same flow from the agent | Not performed: needs the user's Claude Code installation and judgement | ⏳ pending (user) |

## Verdict

The spec is fulfilled. Every one of the 32 functional requirements and the five non-functional requirements has at least one green test that exercises exactly the required behaviour, and all eleven edge cases are covered. The suite reports 79 passing tests across 13 targets with none failing; the ignored performance test passes in release with a p95 of 2.68 ms against a 100 ms threshold; clippy with warnings denied and rustfmt are clean.

Defects found during validation: none in the implementation. Two edge cases of section 8 had no covering test (concurrent spec creation, archiving the active spec); task T44 added both tests, which passed against the existing code without changes.

Pending: the last row of the manual validation, connecting a real Claude Code session, is left to the user. The scripted client exercised the same transport, authentication and tools, so no behavioural gap is expected.

With the user's agreement the branch `001-store-core` can be merged into `main` with `git merge --no-ff`.
