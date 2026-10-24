# Validation — Spec 008

Spec: `specs/008-landing-page/spec.md` · Date: 2026-10-24 · Ticket: SV-2

## Requirements

| FR | Covered by | Result |
|---|---|---|
| FR-1 | `apps/landing/app/prerender.test.ts`: exact ordered list of section headings in `/` and `/es`; header anchors; browser check at 1280 px | pass |
| FR-2 | `prerender.test.ts`: four plans with prices and features, "Most teams" highlight, readers-free note | pass |
| FR-3 | `apps/landing/app/Faq.test.tsx`: first question open on load, opening another closes it; prerender: exactly one `<details open>` | pass |
| FR-4 | `prerender.test.ts`: Start free → `${APP_URL}/auth/login?action=register`, Sign in → `${APP_URL}/auth/login` | pass |
| FR-5 | `prerender.test.ts`: Book a demo → `mailto:hello@eduindev.com?subject=SDD%20Vault%20demo` | pass |
| FR-6 | `prerender.test.ts`: the real connect command and the `sddvault import … --dry-run` and `export` commands | pass |
| FR-7 | `prerender.test.ts` runs every assertion on `/` (English) and `/es` (Spanish); `content.test.ts`: both languages have the same keys and no empty string | pass |
| FR-8 | `prerender.test.ts`: ids `how-it-works`, `features`, `security`, `pricing`, `faq` and their header anchors | pass |
| NFR-1 | Lighthouse mobile: performance 0.98, accessibility 0.96 on `/` and `/es` | pass |
| NFR-2 | Browser check at 390 px and 1280 px: no horizontal overflow (scroll width equals viewport width) | pass |

## Gate

| Check | Result |
|---|---|
| `make fmt-check`, `make lint`, `make test` | pass; `perf_read` measured 130 ms once while other projects loaded the machine and 46–54 ms on reruns (threshold 100 ms) |
| Playwright | 8/8 |
| Lighthouse | 0.98 / 0.96; the colour-contrast audit still fails on white text over the TapTap teal buttons (decision raised on SV-1) |

## Deviations from the handoff

- How it works step 1 reads "Paste one command and approve access in your browser": agents authorize through MCP authorization, not with a pasted token.
- The connect and CLI commands are the real ones (`mcp-sddvault.eduindev.com`, `sddvault`).
- "Trust center" and "Status" links are omitted until those pages exist; Security goes to the Security section, Docs to How it works, DPA and Responsible disclosure to `security@eduindev.com`.

## Verdict

Local: pass. Staging: pending the user's check of https://staging-sddvault.eduindev.com and `/es`.
