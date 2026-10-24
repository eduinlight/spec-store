# Spec 008 — Landing page

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-2

## 1. Context and goal

The landing page carries the placeholder copy and the three feature cards of spec 005. The SDD Vault design
handoff (`SDD Vault Landing.dc.html`, FUNCTIONALITY §1, product-spec §12) gives the final page: the story of
why specs need a governed home, how SDD Vault works, its features, its guardrail and concurrency stories, its
security posture, pricing and answers to common questions. The goal is to convert developers to sign up and
companies to ask for a demo, and to reassure security reviewers.

## 2. Users / actors

- Visitor (developer, tech lead, engineering manager, product person, security reviewer).

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| `apps/landing/app/landing.tsx` with three feature cards | Placeholder of spec 005 | replace | Design handoff |
| `content/en.ts`, `content/es.ts` | English and Spanish copy | replace | New copy in both languages (spec 005 FR-24) |
| Prerender of `/` and `/es` | Full text in the first HTML response (spec 005 FR-25) | keep | Same requirement |

## 4. User stories (by priority)

### US-1 (P1) — Understand and sign up
As a visitor I want to read what SDD Vault is, how it works and what it costs, and start free or sign in from any point of the page.
Independent test: the prerendered page contains every section and its sign-up and sign-in links.

### US-2 (P2) — Ask for a demo
As a company visitor I want to book a demo.
Independent test: every "Book a demo" action opens an email to the demo address.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL show, in this order, a sticky header (logo, anchors How it works, Features, Security, Pricing, FAQ, Sign in, Book a demo, Start free), the hero, the problem, how it works, features, the guardrails story, the concurrency story, the teams and security section, open and portable, pricing, the FAQ and the closing call to action with the footer, with the texts of the handoff.
- **FR-2** (US-1): THE SYSTEM SHALL show the four plans Free ($0), Pro (Per user), Team (Per seat, highlighted "Most teams") and Enterprise (Custom) with the feature lists of the handoff, and the note that readers and reviewers are free on every paid plan.
- **FR-3** (US-1): WHEN a visitor opens a FAQ question, THE SYSTEM SHALL show its answer and close the question that was open; the first question is open on load.
- **FR-4** (US-1): THE SYSTEM SHALL link every "Start free" action to the identity provider's sign-up through the app and every "Sign in" to its sign-in, as spec 005 FR-25 requires.
- **FR-5** (US-2): THE SYSTEM SHALL open an email to `hello@eduindev.com` with the subject "SDD Vault demo" from every "Book a demo" action.
- **FR-6** (US-1): THE SYSTEM SHALL show the real connect command `claude mcp add --transport http sdd-vault https://mcp-sddvault.eduindev.com/mcp` and the real CLI commands (`sddvault import <org>/<project> . --dry-run`, `sddvault export <org>/<project> ./out`).
- **FR-7** (US-1): THE SYSTEM SHALL serve the whole page in English at `/` and in Spanish at `/es`, with the full text in the first HTML response.
- **FR-8** (US-1): WHEN a visitor follows a header anchor, THE SYSTEM SHALL scroll to its section.

## 6. Non-functional requirements

- **NFR-1**: Lighthouse on mobile ≥ 90 for performance and accessibility on `/` and `/es` (spec 005 NFR-7).
- **NFR-2**: The page works at phone width without horizontal scroll.

## 7. Key entities

- Plan, FAQ entry, section.

## 8. Edge cases

- JavaScript disabled: every section and link is present; the FAQ shows the first answer open.
- Narrow screens: header anchors collapse; grids stack.

## 9. Out of scope

- A `/security` page and trust centre (not designed); the Security links go to the Security section.
- Pages for Status, Docs, DPA and Responsible disclosure: Docs goes to How it works, the others to an email to `security@eduindev.com` (DPA and disclosure) or are omitted (Status) until those pages exist.
- Real prices.

## 10. Completion criteria

- Every FR covered by a green test; Lighthouse thresholds met.

## 11. Assumptions

- The design's claims about roadmap features (SSO and SCIM, regions, SIEM export) stay as marketing copy; their tickets (SV-18, SV-19, SV-20) deliver them.
- The design's `mcp.specstore.app` host and `Authorization: Bearer` header are replaced by the real host and the MCP authorization flow, which needs no header.
