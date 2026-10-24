# Validation — Spec 010

Spec: `specs/010-agent-attribution/spec.md` · Date: 2026-10-24 · Ticket: SV-4

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | `packages/core/tests/attribution.rs` (client and origin on revisions and audit entries); API tests: bearer with `azp=sdd-vault-cli` → "sddvault CLI", session → "Web app", other bearer → registered name or "API client"; MCP tests: per-request client info name, registered name through the resolver, "MCP client" fallback | pass |
| FR-2 | `core::origin::client_origin` unit tests and API/MCP tests: socket peer, `X-Forwarded-For` with `TRUSTED_PROXY_HOPS`, clamping, missing header | pass |
| FR-3 | `packages/core/tests/refusals.rs`: an editor's constitution write leaves a denied entry and nothing else; through MCP too | pass |
| FR-4 | `refusals.rs`: stale document write leaves a conflict entry with `expected r{n}, current r{m}`; concurrency tests' losers recorded as conflicts | pass |
| FR-5 | `refusals.rs`: a non-member's write answers not found and records nothing in the project | pass |
| FR-6 | API audit JSON (`client`, `origin`, `result`, `detail`) and revision JSON (`client`), MCP `RevisionOutput.client`, regenerated OpenAPI; app audit (Via, Origin, Result) and history (Client) columns with tests | pass |
| FR-7 | `apps/api/tests/http_parity.rs` and the MCP tool tests unchanged apart from added fields | pass |
| NFR-1 | `CachedResolver` tests with an injected clock: one lookup per (user, client) per 10 minutes | pass |
| NFR-2 | `make test-perf`: 62 ms (API) and 74 ms (core) p95, threshold 100 ms | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint` (with OpenAPI drift check), `make test` | pass |
| Playwright | 10/10 |

## Deployment

- `TRUSTED_PROXY_HOPS=2` for the API and MCP server in both overlays (VPS proxy and Traefik, whose web entry point trusts forwarded headers from 10.8.0.1).
- The MCP server receives `KEYCLOAK_ADMIN_CLIENT_SECRET` (from `api-credentials`) and `KEYCLOAK_INTERNAL_URL` to resolve registered client names.

## Verdict

Local: pass. Staging: migration 0002 and the origin of real requests pending the user's check.
