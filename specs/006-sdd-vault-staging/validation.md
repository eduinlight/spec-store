# Validation — Spec 006

Spec: `specs/006-sdd-vault-staging/spec.md` · Date: 2026-10-24 · Ticket: SV-21

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | `e2e/brand.spec.ts` (landing and identity provider show "SDD Vault", never "Spec Store"); `e2e/keycloak-theme.spec.ts` (sign-in, sign-up, verify email, email body, reset password, error pages in English and Spanish); `apps/app/src/router.test.tsx` (onboarding heading "Welcome to SDD Vault"); `packages/core/src/email.rs` tests (invitation subject) | pass (local) |
| FR-2 | `e2e/brand.spec.ts`: favicons 16/32, apple-touch icon and `theme-color #1F4FE0` on the landing and the app document; favicon and theme colour on the identity provider; horizontal logo with `alt="SDD Vault"` on the landing | pass (local) |
| FR-3 | Workspace builds as `sdd-vault-{core,testkit,api,mcp,cli}`; CLI tests run the `sddvault` binary (`apps/cli/tests/cli_login.rs` asserts "run sddvault login"); MCP server info name `sdd-vault` (`apps/mcp/src/tools.rs`); realm `sdd-vault` with clients `sdd-vault-app`, `sdd-vault-cli`, `sdd-vault-api-admin` imported by the local stack and used by every e2e test | pass (local) |
| FR-4 | Staging Ingress rules for the five `staging-*sddvault.eduindev.com` hosts (`eduinlight-org/sdd-vault-cd`, `staging/ingress.yaml`); `kubectl kustomize staging` and `prod` build | pending: hostnames checked after the merge |
| FR-5 | Deploy workflows build `ghcr.io/eduinlight/sdd-vault-*` and bump `eduinlight-org/sdd-vault-cd` with `CD_TOKEN` (secret set on 2026-10-24); ArgoCD Application `sdd-vault-staging` synced | pending: first pipeline run after the merge |
| FR-6 | Namespace `sdd-vault-staging` with generated database, realm and session secrets of its own; SMTP through the SES relay; Garage key scoped to bucket `sdd-vault-staging` | pass (secrets created); email delivery pending on staging |

## Gate

| Check | Result |
|---|---|
| `make fmt-check` | pass |
| `make lint` (clippy, Biome, typecheck, OpenAPI drift) | pass |
| `make test` | pass, except `apps/api/tests/http_invitations.rs` in parallel: three tests time out opening their database pool while another project's container used more than 3 CPU cores of the Docker VM; the same file passes 4/4 with `--test-threads=1` (9 s) |
| `make e2e`: Playwright 8/8, Lighthouse landing performance 0.98 and accessibility 1.0 on `/` and `/es/`, backup check | pass |

## Defects found and fixed

- The full-size horizontal logo (2314×555 PNG, 705 KiB page weight) dropped the landing's Lighthouse performance to 0.81; the copies served by the landing and the app are resized to 268×64 (145 KiB page weight, score 0.98).
- The e2e tests wrote screenshots into spec 005's evidence folder and assumed fixed ports; they now write to `test-results/evidence` and read the worktree's ports from the environment, and the app and landing dev servers take their ports from `APP_URL` and `LANDING_URL`.

## Verdict

Local: pass. Staging rows (FR-4, FR-5, FR-6 email) pending the first pipeline after the merge.
