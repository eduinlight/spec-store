# Technical plan — Spec 010

Spec: `specs/010-agent-attribution/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

`Caller` gains a `Via { client: String, origin: Option<String> }` filled by each transport. Migration 0002 adds
`revisions.client`, `audit_log.client`, `audit_log.origin`, `audit_log.result` (`ok|denied|conflict`, default
`ok`) and `audit_log.detail`. The audit helpers write the caller's `Via`. Refused writes are recorded by a
`Store::audited` decorator around the write's transaction: when the closure returns `Forbidden` or `Conflict`,
it records the refusal in a new transaction through a `RefusalRecorder` port (Diesel adapter in core), and
returns the original error. Client names for MCP come from a `ClientNameResolver` port with an in-memory TTL
cache in front of the Keycloak consent lookup.

## 2. Technical context

Rust 1.97, Diesel 2.3, axum 0.8, rmcp 3.5.1 (stateless: `ctx.client_info()` is "rmcp" unless the request sends
per-request `_meta` client info), Keycloak admin client already in `packages/core/src/keycloak.rs`
(`list_consents`). No new dependency.

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 4 | yes | Two ports, each justified: `ClientNameResolver` (Keycloak lookup behind a cache; tests use a fake) and the refusal recording as a decorator (one place instead of 18 write functions) |
| 6 | yes | Numbered migration 0002; audit stays append-only |
| 7 | yes | Non-member probes are not attributed to projects |

## 4. Module / file structure

```
packages/core/migrations/0002_attribution/up.sql
packages/core/src/schema.rs                         new columns
packages/core/src/model.rs                          Via, Caller.via, AuditResult
packages/core/src/store/mod.rs                      audit/audit_in write via; AuditEntry gains client, origin, result, detail;
                                                    Store::audited(caller, Attempt{action,target,project}, f) records refusals
packages/core/src/store/documents.rs, specs.rs, tasks.rs, projects.rs, organizations.rs, invitations.rs, transfer.rs
                                                    revisions carry client; writes wrapped by audited
packages/core/src/clients.rs                        ClientNameResolver trait, CachedResolver (TTL 10 min), KeycloakConsentNames
apps/api/src/origin.rs                              origin layer (socket peer or X-Forwarded-For with TRUSTED_PROXY_HOPS)
apps/api/src/auth.rs, bff.rs                        Via for bearer (azp sdd-vault-cli → "sddvault CLI") and session ("Web app")
apps/mcp/src/auth.rs, tools.rs                      Via from per-request client info, else resolver(azp), else "MCP client"
apps/api/src/routes/audit.rs, documents.rs, apps/mcp/src/tools.rs  new output fields; make openapi
apps/app/src/routes/audit.tsx, history.tsx           Via, Origin, Result and client columns
```

## 5. Data

```sql
ALTER TABLE revisions ADD COLUMN client text;
ALTER TABLE audit_log ADD COLUMN client text, ADD COLUMN origin text,
  ADD COLUMN result text NOT NULL DEFAULT 'ok' CHECK (result IN ('ok','denied','conflict')),
  ADD COLUMN detail text;
```

## 6. Contracts

- Audit entry JSON: `client` (string or null), `origin` (string or null), `result` (`ok|denied|conflict`), `detail` (string or null).
- Revision JSON (API and MCP): `client` (string or null).
- Conflict detail: `expected r{n}, current r{m}`.

## 7. Decisions

- **`Via` inside `Caller`** rather than a new parameter on every write: every write already receives the caller;
  rejected: a separate `RequestContext` argument (touches every signature and test for the same information).
- **Decorator for refusals**: rejected: recording inside each write (18 places, and the rollback removes it).
- **Not recording non-member probes against the project** (FR-5): owners would otherwise see outsiders' guesses.
- **In-memory TTL cache** for client names; rejected: Redis now (no shared state needed for a cache of
  display names; Redis arrives with presence in SV-13).

## 8. Security

Origins are personal data: shown only where audit entries are shown (owners, operators). The trusted hop count
prevents clients from spoofing their origin with their own `X-Forwarded-For` values.

## 9. Test strategy

Core: refusal decorator (denied, conflict, non-member not recorded, validation not recorded, original error
returned), attribution on revisions and audit, migration list and schema columns. API: origin from socket and
from `X-Forwarded-For` with hops; bearer CLI vs session client names; audit JSON fields. MCP: client name from
per-request info, from resolver, fallback; conflict and denied audit entries through MCP. App: columns render.
