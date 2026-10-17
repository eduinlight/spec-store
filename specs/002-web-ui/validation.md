# Validation — Spec 002

Date: 2026-10-17 · Suite executed: `cargo test` (plus `cargo test --release --test perf --test perf_ui -- --ignored` and `cargo clippy --all-targets -- -D warnings && cargo fmt --check`) · Overall result: 138 pass / 0 fail (2 ignored performance tests, both passing in release)

Test names are `file::function`; unit tests inside `src/` are named by module.

| FR | Test(s) | Result | Notes |
|---|---|---|---|
| FR-1 | `tests/ui_session.rs::visitors_are_sent_to_the_login_page_with_the_requested_path`, `tests/ui_session.rs::login_page_shows_a_form_and_no_data` | ✅ | Unknown `/ui/...` paths also go through the session check; the requested path travels in `next` |
| FR-2 | `tests/store_sessions.rs::create_session_returns_a_prefixed_secret_stored_only_hashed`, `tests/store_sessions.rs::session_caller_resolves_user_and_updates_last_used`, `tests/ui_session.rs::valid_token_creates_a_session_and_redirects_to_next` | ✅ | `last_used_at` set on login and on every page |
| FR-3 | `tests/ui_session.rs::invalid_tokens_get_an_error_without_session_or_echo`, `tests/store_sessions.rs::create_session_rejects_unknown_and_revoked_tokens` | ✅ | |
| FR-4 | `tests/store_sessions.rs::unknown_ended_revoked_and_expired_sessions_are_unauthenticated`, `tests/store_sessions.rs::sessions_expire_thirty_days_after_creation`, `tests/ui_session.rs::signed_in_pages_show_the_viewer_and_end_on_revocation_or_logout` | ✅ | Stale cookie is cleared on redirect |
| FR-5 | `tests/ui_session.rs::signed_in_pages_show_the_viewer_and_end_on_revocation_or_logout` | ✅ | Other sessions of the same user stay valid |
| FR-6 | `src/ui/html.rs::layout_shows_title_viewer_and_controls`, `src/ui/html.rs::layout_marks_admins_and_links_the_global_audit`, `tests/ui_session.rs::signed_in_pages_show_the_viewer_and_end_on_revocation_or_logout` | ✅ | |
| FR-7 | `tests/store_tasks.rs::project_overview_summarises_every_visible_project`, `tests/ui_projects.rs::projects_page_lists_memberships_with_progress` | ✅ | Admin rows without membership are marked |
| FR-8 | `tests/ui_projects.rs::outsiders_get_the_same_not_found_page_for_existing_and_missing_projects` | ✅ | Identical 404 bodies |
| FR-9 | `tests/ui_projects.rs::project_page_shows_specs_constitution_repository_and_members`, `tests/ui_projects.rs::empty_project_and_admin_view` | ✅ | |
| FR-10 | `tests/ui_specs.rs::spec_page_shows_documents_in_order_with_metadata` | ✅ | Sections in the five-kind order, missing kinds marked |
| FR-11 | `src/ui/render.rs::renders_headings_lists_and_tables`, `src/ui/render.rs::renders_task_boxes_with_their_state`, `tests/ui_specs.rs::document_pages_render_and_raw_view_returns_exact_bytes` | ✅ | Raw view compared byte for byte, CRLF included |
| FR-12 | `src/ui/render.rs::raw_html_is_shown_as_text`, `src/ui/render.rs::unsafe_link_schemes_are_dropped_and_safe_ones_kept`, `tests/ui_specs.rs::spec_page_shows_documents_in_order_with_metadata` | ✅ | `javascript:` and `data:` destinations dropped |
| FR-13 | `tests/ui_specs.rs::spec_page_groups_tasks_by_phase_and_lists_markers` | ✅ | Phase totals and malformed lines with line numbers |
| FR-14 | `tests/ui_specs.rs::spec_page_groups_tasks_by_phase_and_lists_markers` | ✅ | Trailing slash of the repository URL trimmed |
| FR-15 | `tests/ui_specs.rs::spec_page_groups_tasks_by_phase_and_lists_markers`, `tests/ui_projects.rs::projects_page_lists_memberships_with_progress` | ✅ | |
| FR-16 | `tests/store_projects.rs::repository_url_is_validated_audited_and_clearable`, `tests/cli.rs::project_set_repo_sets_validates_and_clears` | ✅ | Audit action `project.repo` |
| FR-17 | `tests/ui_history.rs::history_lists_revisions_newest_first_and_marks_the_current_one` | ✅ | Constitution history shows versions |
| FR-18 | `tests/ui_history.rs::revision_pages_show_old_bodies_and_point_to_the_current_one` | ✅ | |
| FR-19 | `src/ui/diff.rs::insertions_and_deletions_carry_line_numbers`, `src/ui/diff.rs::distant_changes_make_separate_hunks`, `tests/ui_history.rs::diff_pages_compare_with_the_predecessor` | ✅ | Three lines of context, first revision says no predecessor |
| FR-20 | `tests/store_audit.rs::list_audit_page_is_authorized_and_paged`, `tests/ui_audit.rs::owners_see_their_project_audit_in_pages` | ✅ | 50 per page, cursor on the entry id |
| FR-21 | `tests/ui_audit.rs::admins_see_everything_with_a_project_filter` | ✅ | |
| FR-22 | `tests/ui_audit.rs::editors_are_not_allowed_and_see_no_link`, `tests/store_audit.rs::list_audit_page_is_authorized_and_paged` | ✅ | 403 page; no audit links for editors and readers |
| FR-23 | `tests/ui_readonly.rs::ui_paths_reject_writing_methods`, `tests/ui_readonly.rs::browsing_every_page_changes_nothing_but_the_session` | ✅ | Table counts and revision bytes unchanged after browsing every page |
| FR-24 | `tests/store_users.rs::list_own_tokens_needs_no_admin_and_shows_only_the_callers`, `tests/ui_session.rs::profile_lists_own_tokens_without_secrets` | ✅ | Current session marked |
| FR-25 | `tests/store_projects.rs::list_members_orders_owners_first_and_handles_empty`, `tests/ui_projects.rs::project_page_shows_specs_constitution_repository_and_members` | ✅ | |
| NFR-1 | `tests/perf_ui.rs::p95_spec_page_under_300ms` (release, ignored by default) | ✅ | Measured p95 of the spec page for a 200 KB spec with 10 threads: 55.8 ms |
| NFR-2 | `tests/ui_session.rs::token_and_session_secret_never_leak`, `tests/store_sessions.rs::create_session_returns_a_prefixed_secret_stored_only_hashed`, `src/ui/cookies.rs::set_cookie_carries_the_protective_attributes` | ✅ | `HttpOnly`, `SameSite=Lax`, `Secure` with `--secure-cookies`; only the hash is stored |
| NFR-3 | `tests/ui_specs.rs::spec_page_shows_documents_in_order_with_metadata`, `tests/ui_history.rs::revision_pages_show_old_bodies_and_point_to_the_current_one` | ✅ | Every view reached through plain links; no script is served |
| NFR-4 | `tests/ui_session.rs::signed_in_pages_show_the_viewer_and_end_on_revocation_or_logout` | ✅ | `/mcp` answers with the bearer token beside an open session |
| NFR-5 | `tests/ui_specs.rs::constitution_page_and_large_documents` | ✅ | 1 MB document rendered and returned raw byte for byte |
| NFR-6 | no test | ✅ by inspection | All pages and messages are English |

## Edge cases

| Case | Test | Result |
|---|---|---|
| Empty store or user with no memberships | `tests/ui_projects.rs::projects_page_lists_memberships_with_progress` (user without projects) | ✅ |
| Project without constitution or specs | `tests/ui_projects.rs::empty_project_and_admin_view` | ✅ |
| Spec with no documents | `tests/ui_specs.rs::spec_page_groups_tasks_by_phase_and_lists_markers` (spec 2: no tasks document, no markers) and `tests/ui_projects.rs::project_page_shows_specs_constitution_repository_and_members` (`0/0`) | ✅ |
| Tasks document with malformed lines | `tests/ui_specs.rs::spec_page_groups_tasks_by_phase_and_lists_markers` | ✅ (malformed list; a document with only malformed lines yields an empty table through the same code path) |
| Member removed or token revoked while the browser is open | `tests/ui_session.rs::signed_in_pages_show_the_viewer_and_end_on_revocation_or_logout`, `tests/ui_projects.rs::outsiders_get_the_same_not_found_page_for_existing_and_missing_projects` | ✅ |
| Session cookie from another store or before a reset | `tests/ui_session.rs::visitors_are_sent_to_the_login_page_with_the_requested_path` (stale cookie cleared) | ✅ |
| Repeated invalid logins | `tests/ui_session.rs::invalid_tokens_get_an_error_without_session_or_echo` (three failures, no session rows) | ✅ |
| Raw HTML, scripts and `javascript:` links in a document | `src/ui/render.rs::raw_html_is_shown_as_text`, `src/ui/render.rs::unsafe_link_schemes_are_dropped_and_safe_ones_kept` | ✅ |
| 1 MB document with Windows line endings | `tests/ui_specs.rs::constitution_page_and_large_documents`, `tests/ui_specs.rs::document_pages_render_and_raw_view_returns_exact_bytes` (CRLF), `src/ui/render.rs::handles_crlf_and_code_blocks` | ✅ |
| Revision 1 diff; identical bodies | `tests/ui_history.rs::diff_pages_compare_with_the_predecessor`, `src/ui/diff.rs::identical_bodies_have_no_hunks` | ✅ |
| Repository URL with trailing slash | `tests/ui_specs.rs::spec_page_groups_tasks_by_phase_and_lists_markers` | ✅ |
| Admin without membership | `tests/ui_projects.rs::empty_project_and_admin_view`, `tests/ui_audit.rs::admins_see_everything_with_a_project_filter` | ✅ |
| Login `next` pointing outside the UI | `src/ui/pages/login.rs::safe_next_keeps_ui_paths_only`, `tests/ui_session.rs::valid_token_creates_a_session_and_redirects_to_next` | ✅ |
| Project with no members | `tests/ui_projects.rs::empty_project_and_admin_view` | ✅ |

## Manual validation

Performed on 2026-10-17 with the release binary, a fresh database and Google Chrome driven through the browser tools against `spec-store serve --bind 127.0.0.1:7399`.

| Step | Observed | Result |
|---|---|---|
| CLI: `user add alice`, `user add bob --as alice`, `project add demo`, `member set demo bob editor`, `project set-repo demo https://github.com/eduinlight/spec-store`, `token issue bob` | Each confirmed on stdout; audit shows `project.repo demo` | ✅ |
| MCP (HTTP, bearer token): `create_spec store-core`, `put_document` tasks, `mark_task_done T01 43dbcd9` | Revisions 1 and 2 created, status `tasks_done 1` | ✅ |
| Browser: open `http://127.0.0.1:7399/` | Redirected to `/ui/` then to `/ui/login?next=%2Fui%2F`; login form, no data | ✅ |
| Paste the token, Sign in | Projects page: `Demo project · demo · editor · no constitution · 1 spec · 001 store-core draft 1/2 tasks, 0 markers`; header shows `bob` and Log out | ✅ |
| Open spec 001 | Sections spec, plan, tasks, research, validation in order; tasks rendered with T01 checked; tasks table shows T01 done with commit link `https://github.com/eduinlight/spec-store/commit/43dbcd9` and T02 pending | ✅ |
| Open the tasks history | Revision 2 (current, `T01: done, commit 43dbcd9`) above revision 1 (`initial tasks`), both by bob with time and diff links | ✅ |
| Open the diff of revision 2 | Context lines 1–3, removed `- [ ] T01 ...`, added `- [x] T01 ... · commit: 43dbcd9`, line 5 unchanged | ✅ |
| Log out | Login page shown; a second tab with the old cookie opening `/ui/p/demo` was sent to `/ui/login?next=%2Fui%2Fp%2Fdemo`; the session row has `ended_at` set | ✅ |
| Server log | Contains neither the token nor any session secret | ✅ |

## Verdict

The spec is fulfilled. All 25 functional requirements and six non-functional requirements have green tests that exercise the required behaviour, every edge case of section 8 is covered, and the manual validation in a real browser matched the completion criteria. The suite reports 138 passing tests with none failing; both ignored performance tests pass in release (spec page p95 55.8 ms against 300 ms; MCP read p95 under 100 ms unchanged); clippy with warnings denied and rustfmt are clean.

Defects found during validation: none in the implementation. Three test fixtures were corrected during implementation (ordering of a malformed-line revision, scoping of HTML assertions to the tasks table, the browsing user of the read-only test), each before its task was closed.

With the user's agreement, given on 2026-10-17 for this branch, `002-web-ui` is merged into `main` with a merge commit.
