# Spec 009 — Sign-in pages in the SDD Vault design

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-3

## 1. Context and goal

People sign in, sign up, verify their email and reset their password on Keycloak's pages, which carry only the
product name, logo and colours on Keycloak's own card layout (spec 005 FR-9, spec 006). The design handoff
(`SDD Vault Sign In.dc.html`) gives the sign-in page its own layout: the form in a left column with the logo,
title, subtitle, a footer that switches between sign-in and sign-up and the terms note, and a dark right panel
telling the "Prompts are suggestions. SDD Vault rules are enforced." story. This spec gives every Keycloak page
that layout, for the flows Keycloak already offers. New sign-in methods (social providers, magic link,
two-factor, company SSO) come with SV-18.

## 2. Users / actors

- Visitor signing in or creating an account; person verifying an email or resetting a password.

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| `deploy/keycloak/themes/sdd-vault/login` (keycloak.v2 parent, CSS only) | Branded Keycloak card | replace layout | Design handoff |
| Email theme | Branded emails | keep, tokens updated | Same tokens |

## 4. User stories

### US-1 (P1) — Designed sign-in
As a visitor I want the sign-in and sign-up pages to look like the rest of SDD Vault and tell me what it protects.
Independent test: each Keycloak page renders the two-column layout, in English and Spanish.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL render every login-theme page (sign in, sign up, forgot password, update password, verify email, info, error, page expired, and the default pages of other required actions) with a left column holding the horizontal logo linking to the landing page, the page title (30/38, weight 500) and the form, and a right panel on `--neutral-700` with the eyebrow "Prompts are suggestions", the heading "SDD Vault rules are enforced.", the `put_constitution(project: "payments", base_revision: 7)` call, the `403 authorization_error` box with "Only project owners can write the constitution.", and the two trust lines of the design.
- **FR-2** (US-1): THE SYSTEM SHALL style inputs, buttons, links, alerts and field errors with the TapTap tokens: 40 px controls, 4 px radius, inset 1 px borders, primary buttons in `--primary-600`, links in `--primary-700`, field errors in `--danger-600`.
- **FR-3** (US-1): ON the sign-in page THE SYSTEM SHALL show the subtitle and a footer "New to SDD Vault? Create an account" linking to registration; ON the registration page the subtitle "Free for up to 3 projects. A personal organization is created for you." and "Already have an account? Sign in"; on both the note "By continuing you agree to the Terms and Privacy Policy."
- **FR-4** (US-1): THE SYSTEM SHALL show every text of FR-1 to FR-3 in English and Spanish, following the language chosen on the page or the browser's language.
- **FR-5** (US-1): WHEN the window is narrower than 880 px, THE SYSTEM SHALL stack the right panel below the form without horizontal scroll.

## 6. Non-functional requirements

- **NFR-1**: No Keycloak behaviour changes: every existing sign-in, sign-up and email test keeps passing.

## 7. Key entities

- Login theme page.

## 8. Edge cases

- Error messages and field validation (wrong password, taken username) stay visible in the new layout.
- The language selector stays available.

## 9. Out of scope

- Social providers, magic link, two-factor code entry, recovery codes, SSO domain hint (SV-18).
- Terms and Privacy Policy pages (not designed; the note is plain text until they exist).
- Status, Security and Docs footer links (no pages yet; omitted).

## 10. Completion criteria

- Every FR covered by a green e2e test with screenshots of each page in both languages.

## 11. Assumptions

- The layout is built by overriding the keycloak.v2 `template.ftl` frame and styling Keycloak's own form markup, so Keycloak upgrades keep the page templates.
