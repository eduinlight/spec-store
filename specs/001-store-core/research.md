# Research — Spec 001

Date: 2026-10-17. Versions checked with `cargo search` on this date.

## R1. Which MCP SDK and version, and how is a server built with it?

- Finding: `rmcp` 3.5.1 is the official Rust SDK. Features needed: `server`, `macros` (default), `schemars`, `transport-io` (stdio), `transport-streamable-http-server`. A server is a struct holding a `ToolRouter<Self>`; tools are methods in an `#[tool_router] impl` annotated `#[tool(description = "...")]`, taking `Parameters<T>` where `T: Deserialize + JsonSchema`, and optionally `RequestContext<RoleServer>`. `ServerHandler` is implemented with `#[tool_handler]` and `get_info` returning capabilities. Stdio: `handler.serve(stdio()).await?.waiting().await?`.
- Source: https://docs.rs/rmcp/latest/rmcp/ · https://github.com/modelcontextprotocol/rust-sdk/blob/main/README.md · https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/examples/servers/src/common/counter.rs
- Decision: `rmcp = "3.5"` with the features above; one handler struct `SpecStoreMcp` with 16 tools.

## R2. How is the Streamable HTTP transport mounted, and how does a tool see the HTTP request?

- Finding: `StreamableHttpService::new(|| Ok(handler), LocalSessionManager::default().into(), StreamableHttpServerConfig::default()...)` is mounted with `axum::Router::new().nest_service("/mcp", service)`. The service consumes the body and injects the remaining `http::request::Parts` into the request `Extensions`, readable in a tool as `ctx.extensions.get::<axum::http::request::Parts>()`. `StreamableHttpServerConfig` has `with_allowed_hosts` (defaults to loopback only, DNS rebinding guard), `with_allowed_origins`, `with_max_request_body_bytes`, `with_cancellation_token`, `with_json_response`.
- Source: https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/examples/servers/src/counter_streamhttp.rs · https://docs.rs/rmcp/latest/rmcp/transport/streamable_http_server/tower/struct.StreamableHttpService.html · https://docs.rs/rmcp/latest/rmcp/transport/streamable_http_server/tower/struct.StreamableHttpServerConfig.html
- Decision: an axum middleware authenticates the bearer token, resolves the user and inserts a `Caller` value into `request.extensions_mut()` before `next.run`; tools read it through `Parts.extensions`. `allowed_hosts` is configured from the CLI (`--allowed-host`, repeatable) because the server is remote. `max_request_body_bytes` set to 2 MiB to bound NFR-2's 1 MB bodies with JSON overhead.

## R3. How does the SDK's own example authenticate incoming HTTP requests?

- Finding: `complex_auth_streamhttp.rs` uses `axum::middleware::from_fn_with_state(store, validate_token_middleware)` as a `route_layer` on `Router::new().nest_service("/mcp", mcp_service)`. The middleware reads `Authorization`, strips `Bearer `, validates against a store, and returns `401` with `WWW-Authenticate: Bearer ...` otherwise.
- Source: https://raw.githubusercontent.com/modelcontextprotocol/rust-sdk/main/examples/servers/src/complex_auth_streamhttp.rs
- Decision: same shape, without OAuth metadata (out of scope): `401` with `WWW-Authenticate: Bearer` for missing, unknown or revoked tokens (FR-7).

## R4. MCP transport requirements the server must meet

- Finding: Streamable HTTP needs a single endpoint for POST and GET, `Mcp-Session-Id` handling, `MCP-Protocol-Version` header handling, and Origin validation; stdio servers must write only MCP messages to stdout and may log to stderr.
- Source: https://modelcontextprotocol.io/specification/2025-06-18/basic/transports
- Decision: `rmcp` implements the endpoint semantics; the server configures hosts/origins and sends all logs to stderr through `tracing-subscriber` so stdio mode stays clean.

## R5. SQLite driver, bundling, WAL and transactions

- Finding: `rusqlite` 0.40.2; feature `bundled` "automatically compile[s] and link[s] in an up to date version of SQLite" (3.53.2). `Connection::open`, `execute_batch`, `pragma_update(None, "journal_mode", "WAL")`, `busy_timeout(Duration)`, `transaction(&mut self) -> Transaction` with `commit()`, `query_row`, `prepare`, `last_insert_rowid`. `Connection` is `Send` but not `Sync`.
- Source: https://docs.rs/rusqlite/latest/rusqlite/struct.Connection.html · https://github.com/rusqlite/rusqlite/blob/master/README.md
- Decision: `rusqlite = "0.40"` with `bundled`; one `Connection` behind `Arc<Mutex<_>>` (all writes serialized, which satisfies FR-24 and the first-write race by construction); WAL plus `busy_timeout` so the admin CLI can use the same file while the server runs; `foreign_keys = ON`.

## R6. How does Claude Code connect to a remote MCP server with a token?

- Finding: `claude mcp add --transport http <name> <url> --header "Authorization: Bearer <token>"`; in `.mcp.json`: `{"type":"http","url":"...","headers":{"Authorization":"Bearer ${SPEC_STORE_TOKEN}"}}` with `${VAR}` expansion. Variables named like Anthropic credentials are blanked, so the token variable must have its own name.
- Source: https://code.claude.com/docs/en/mcp
- Decision: the manual validation uses `SPEC_STORE_TOKEN`; the project-level `.mcp.json` snippet is documented in the README of a later spec.

## R7. Token format and hashing

- Finding: tokens are generated by the server with 32 random bytes, so they carry 256 bits of entropy; a slow password hash defends low-entropy secrets, which does not apply. SHA-256 of a high-entropy random token is the standard practice for API keys.
- Source: project reasoning on the spec (FR-2, NFR-3); `sha2` 0.11, `rand` 0.10 available.
- Decision: token = `sst_` + base64url(32 random bytes); stored as hex SHA-256; `argon2` rejected as unnecessary cost and dependency.

## R8. Markdown parsing for tasks and markers

- Finding: the task line format is fixed by `.sdd/templates/tasks.md` and lives on one line; markers are bracketed inline strings; headings in the templates are ATX (`#`). A line-oriented parser covers FR-25 to FR-28 without a CommonMark parser.
- Source: `.sdd/templates/tasks.md`, `.sdd/templates/spec.md`, spec FR-25 to FR-27 and Assumptions.
- Decision: hand-written line parser in `markdown/`; `pulldown-cmark` and `regex` not added (constitution principle 2, every extra dependency justified).

## R9. Timestamps and identifiers

- Finding: SQLite can produce RFC 3339 UTC timestamps with `strftime('%Y-%m-%dT%H:%M:%fZ','now')`; `INTEGER PRIMARY KEY` gives monotonic identifiers per table.
- Source: SQLite date functions, https://sqlite.org/lang_datefunc.html
- Decision: timestamps generated in SQL, no date crate; revision identifiers are integers unique across all documents (one `revisions` table).
