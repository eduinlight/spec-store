# spec-store

Remote, multi-project store for Spec Driven Development documents (constitution, spec, plan,
tasks, research, validation) exposed to AI agents through an MCP server. One Rust binary serves
the Streamable HTTP MCP endpoint, a read-only browser view and the administration commands, with
a single SQLite file as the only state. Documents are markdown with append-only revisions; tasks, markers and project
status are derived from the current revision on every read.

## Run the server

```sh
cargo build --release
./target/release/spec-store --db /var/lib/spec-store/store.db serve \
  --bind 0.0.0.0:7391 \
  --allowed-host specs.example.com:7391
```

- `--db` (or `SPEC_STORE_DB`) is the SQLite file; it is created on first use.
- `--bind` defaults to `127.0.0.1:7391`.
- `--allowed-host` lists the `Host` header values the server accepts. Without it only loopback
  hosts are accepted, which protects against DNS rebinding; pass the public host name and port
  when serving a team.
- The MCP endpoint is `POST /mcp`; the browser view is at `/ui/`; `GET /healthz` answers `ok`
  without authentication.
- Logs go to stderr; set `RUST_LOG=debug` for more detail.

## Bootstrap an admin, a project and a token

Administration runs on the server host against the database file, without tokens. The first
user created on an empty store becomes an admin; every later command names the acting user with
`--as`.

```sh
spec-store --db store.db user add alice
spec-store --db store.db user add bob --as alice
spec-store --db store.db project add demo "Demo project" --as alice
spec-store --db store.db member set demo bob editor --as alice
spec-store --db store.db token issue bob --label laptop --as alice
```

`token issue` prints the token once on stdout and never again. Roles are `owner` (manages
members, archives specs, writes the constitution), `editor` (creates specs, writes documents,
marks tasks) and `reader`. Admins read every project but write only through a membership.

Other commands: `token list HANDLE`, `token revoke ID`, `member remove PROJECT HANDLE`,
`audit list [--project SLUG] [--limit N]`, all with `--as`.

## Connect Claude Code

```sh
claude mcp add --transport http spec-store https://specs.example.com:7391/mcp \
  --header "Authorization: Bearer sst_..."
```

Or, shared with the team through `.mcp.json` in the project repository, with the token read from
an environment variable (use your own variable name; names of Anthropic credentials are blanked
by Claude Code):

```json
{
  "mcpServers": {
    "spec-store": {
      "type": "http",
      "url": "https://specs.example.com:7391/mcp",
      "headers": {
        "Authorization": "Bearer ${SPEC_STORE_TOKEN}"
      }
    }
  }
}
```

## Browser view

The same server serves a read-only browser view at `/ui/` (the root `/` redirects there).
People sign in with their own API token on `/ui/login`; the server exchanges it for a session
cookie that lasts 30 days or until logout or revocation of the token. The browser can only read:
projects, specs, documents rendered from markdown with a raw view, tasks with their commits, open
markers, revision history with diffs, the project's members, and the audit trail (owners for their
projects, admins for everything). Nothing is written from the browser.

- Behind HTTPS, start the server with `--secure-cookies` so the session cookie is only sent over
  TLS. Without the flag the cookie works over plain HTTP, which suits loopback testing.
- To turn commit identifiers of done tasks into links, record the project's repository URL:

```sh
spec-store --db store.db project set-repo demo https://github.com/acme/demo --as alice
spec-store --db store.db project set-repo demo --clear --as alice
```

Admins and owners of the project may set it. Links are built as `<repository URL>/commit/<id>`,
which fits GitHub, GitLab and Gitea.

## Stdio mode

On the server host, or for tests, the same tools are available over stdin and stdout acting for
one user, without a token:

```sh
spec-store --db store.db stdio --as bob
```

## Tools

Projects are addressed by slug and specs by number.

| Tool | Purpose |
|---|---|
| `list_projects`, `list_specs`, `get_active_spec`, `project_status` | discovery and progress |
| `get_document`, `get_constitution`, `get_template` | reading |
| `create_spec`, `put_document`, `put_constitution`, `set_spec_status`, `mark_task_done` | writing; `put_*` take `expected_revision` for optimistic locking |
| `list_tasks`, `list_markers` | derived from the documents |
| `list_revisions`, `get_revision` | history |

## Development

```sh
cargo test
cargo clippy --all-targets -- -D warnings && cargo fmt --check
cargo test --release --test perf -- --ignored
cargo test --release --test perf_ui -- --ignored
```
