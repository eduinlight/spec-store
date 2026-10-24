# Validation — Spec 009

Spec: `specs/009-keycloak-sign-in-design/spec.md` · Date: 2026-10-24 · Ticket: SV-3

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | `e2e/keycloak-theme.spec.ts`: sign-in, sign-up, reset password, verify email and error pages show the logo linking to the landing page, the title and the right panel (eyebrow, heading, `put_constitution` call, `403 authorization_error` box, trust lines) in English and Spanish; screenshots in `test-results/evidence` compared with the design | pass |
| FR-2 | Screenshots: 40 px inputs and buttons, 4 px radius, inset borders, focus border in the primary colour, primary submit button, links in `--primary-700`; field errors visible in the sign-up flow of `e2e/sign-in.spec.ts` | pass |
| FR-3 | `keycloak-theme.spec.ts`: sign-in subtitle and "New to SDD Vault? Create an account", registration subtitle and "Already have an account? Sign in", terms note on both; `e2e/sign-in.spec.ts` follows "Create an account" to registration | pass |
| FR-4 | Every assertion runs with `ui_locales=en` and `ui_locales=es` | pass |
| FR-5 | `keycloak-theme.spec.ts` at 390 px: no horizontal overflow and the panel below the form | pass |
| NFR-1 | `e2e/sign-in.spec.ts`, `e2e/seeded-sign-in.spec.ts`, `e2e/brand.spec.ts` stay green (10/10 Playwright) | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint`, `make test` | pass, except `perf_read` (101 ms and 960 ms on two runs) while Xcode builds, the iOS simulator and other projects loaded this machine; this branch changes no Rust code, and main's CI runs the same check after the merge |
| Playwright | 10/10 |

## Deviations from the handoff

- The sign-in subtitle reads "Use your username or email and your password." because the design's subtitle names SSO and magic links, which arrive with SV-18.
- "Forgot password?" stays under the password field, where Keycloak's form puts it.
- The error page for an invalid redirect address follows the browser language, not `ui_locales` (Keycloak behaviour).
- Terms and Privacy Policy are plain text until those pages exist.

## Verdict

Local: pass. Staging: pending the user's check of the sign-in and sign-up pages.
