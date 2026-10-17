# Research — Spec 002

Sources are the crate sources installed in the local cargo registry (`~/.asdf/installs/rust/1.97.1/registry/src/index.crates.io-1949cf8c6b5b557f/`) and the official documentation. Paths below are relative to that registry directory.

## R1 — Markdown rendering

Question: how to render CommonMark with tables and task lists to HTML without executing embedded HTML.

Finding: `pulldown-cmark` 0.13.4 parses with `Parser::new_ext(text, Options)` (`pulldown-cmark-0.13.4/src/parse.rs:255`); `Options::ENABLE_TABLES`, `ENABLE_TASKLISTS` and `ENABLE_STRIKETHROUGH` are flags (`src/lib.rs:224-247`); `html::push_html(&mut String, iter)` writes HTML from any event iterator (`src/html.rs:550`), so events can be rewritten before rendering. Raw HTML arrives as `Event::Html` and `Event::InlineHtml` (`src/lib.rs:564,571`) and task boxes as `Event::TaskListMarker(bool)` (`src/lib.rs:611`); links and images carry `dest_url` in `Tag::Link`/`Tag::Image` (`src/lib.rs:266-278`). Default features are `getopts` and `html`; `html` alone pulls only `pulldown-cmark-escape` (`Cargo.toml:43-51`).

Decision: depend on `pulldown-cmark = { version = "0.13", default-features = false, features = ["html"] }`. Map `Html` and `InlineHtml` events to `Event::Text` so tags render as literal text (FR-12); drop `dest_url` of links and images whose scheme is not `http`, `https`, `mailto` or relative. Rejected alternative: a hand-written renderer (not justified by simplicity, the templates use tables and nested lists) and `comrak` (larger dependency tree, GFM extensions not needed).

## R2 — Line diff

Question: how to compute a line diff of a revision against its predecessor with context.

Finding: `similar` 3.2.0 offers `TextDiff::from_lines(old, new)` (`similar-3.2.0/src/text/mod.rs:799`), `grouped_ops(context)` to get hunks with `n` lines of context (`:974`), `iter_changes(op)` per hunk and `iter_all_changes()` (`:982`); each `Change` has `tag()` returning `ChangeTag::{Equal, Delete, Insert}`, `old_index()`, `new_index()` and `value()` (`src/types.rs:61-126`). No default features needed beyond `text`.

Decision: `similar = { version = "3", default-features = false, features = ["text"] }`, hunks via `grouped_ops(3)`, rendered as a table with one row per line and a class per tag. Rejected alternative: the `diff` crate (no grouping or context helpers) and writing Myers by hand.

## R3 — Forms, queries, redirects and HTML responses in axum

Question: what axum 0.8.9 already provides for the UI.

Finding: default features include `form`, `query`, `original-uri` and `tokio` (`axum-0.8.9/Cargo.toml:81-91`). `Form<T>` (`src/form.rs:71`) deserializes `application/x-www-form-urlencoded` bodies, `Query<T>` (`src/extract/query.rs:51`) the query string, `Html<T>` (`src/response/mod.rs:36`) sets `text/html; charset=utf-8`, `Redirect::to` answers 303 See Other and `Redirect::temporary` 307 (`src/response/redirect.rs:37-47`). `middleware::from_fn_with_state` and `route_layer` are already used in `src/server.rs`.

Decision: no new web dependency. Login is `POST` with `Form`, redirects use `Redirect::to`, pages are `Html<String>`.

## R4 — Cookies

Question: how to read and set the session cookie.

Finding: the `http` crate exposes `header::COOKIE` and `header::SET_COOKIE`; a request may carry several `Cookie` headers, each `name=value` pair separated by `; ` (RFC 6265 §5.4, https://www.rfc-editor.org/rfc/rfc6265#section-5.4). Attributes `HttpOnly`, `SameSite=Lax`, `Secure`, `Path` and `Max-Age` are set on `Set-Cookie` (https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie). `axum-extra`'s `CookieJar` and `tower-cookies` wrap the `cookie` crate.

Decision: parse and build the one cookie by hand in `src/ui/cookies.rs` (about twenty lines, unit tested). Rejected alternative: `axum-extra` with the `cookie` feature; it adds three crates for one cookie, against constitution principle 2.

## R5 — Session storage

Question: where sessions live and how the identifier is protected.

Finding: constitution 6 makes SQLite the only state. The token path already stores `sha256(token)` as hex (`src/auth.rs::hash_token`) and generates 32 random bytes with `rand::fill` (`src/auth.rs::generate_token`).

Decision: table `sessions` created by migration `0002_web_ui.sql`, storing the SHA-256 of a random 32-byte secret, the token id, the user id, creation and expiry times and an end time. The cookie carries the secret; the database never does (NFR-2). Lookups join `api_tokens` so a revoked token ends the session on the next request (FR-4). Rejected alternative: in-memory sessions (lost on restart, not the single SQLite state) and signed stateless cookies (cannot be ended on logout or revocation without a store anyway).

## R6 — HTML templating

Question: template engine or hand-written HTML.

Finding: the UI has twelve page kinds, all tables and lists over the store's structs; `askama` 0.16 and `maud` 0.27 would each add a proc-macro crate and a build dependency.

Decision: a small `src/ui/html.rs` with `escape(&str)`, a `layout(title, user, body)` function and page builders that write into a `String`. Everything interpolated from the store goes through `escape`, tested in unit tests. Rejected alternative: `askama` (justified only above a few dozen templates) and `maud` (same, plus nightly-flavoured syntax).

## R7 — Secure cookie behind a proxy

Question: how the server knows whether to mark the cookie `Secure`.

Finding: the binary serves plain HTTP (`axum::serve` over a `TcpListener`, `src/server.rs`); TLS is a reverse proxy's job. A `Secure` cookie is never sent by browsers over `http://` except to `localhost`.

Decision: `serve --secure-cookies` flag (`ServerConfig::secure_cookies`), off by default, documented in the README for deployments behind HTTPS. Rejected alternative: trusting `X-Forwarded-Proto` (spoofable when the proxy does not strip it; needs a trusted-proxy list).

## R8 — Return-to-page after login without open redirects

Question: how to send the user to the page they asked for (FR-1) safely.

Finding: an attacker can craft `?next=//evil.example` or `?next=https://...`; browsers treat `//host` as scheme-relative.

Decision: accept `next` only when it starts with `/ui/` and its second character is not `/`; otherwise fall back to `/ui/`. Unit tested.

## R9 — Rendering tasks with commit links

Question: how to show task state in the tasks section without a second parser.

Finding: `crate::markdown::tasks::parse_tasks` already yields every field including `commit` and `line` (`src/markdown/tasks.rs`), and `Store::list_tasks`/`list_markers`/`project_status` return the same data the MCP tools return.

Decision: the UI calls the store functions and builds tables; the commit link is `format!("{repo}/commit/{sha}")` after trimming one trailing `/` from the repository URL (spec assumption; GitHub, GitLab and Gitea all serve that path).
