# Technical plan — Spec 008

Spec: `specs/008-landing-page/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

The landing is rebuilt from `SDD Vault Landing.dc.html` as React components in `apps/landing/app/sections/`
(one file per section), driven by typed copy in `content/en.ts` and `content/es.ts` (same shape, enforced by a
`LandingCopy` type). Sections use the TapTap tokens and components of `@workspace/ui/taptap` (Button, Tag,
StatusTag, Icon, LockDocument). The FAQ is a list of native `<details>` elements, so it works without
JavaScript, with a small handler that keeps one open. Routes and prerendering stay as they are.

## 2. Technical context

React 19, React Router 7 prerender, Vitest; no new dependency.

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 2 | yes | TapTap components; no new dependency |
| 3 | yes | Tests first: prerender content test per language, FAQ behaviour test, copy shape test |
| 4 | yes | Sections are plain components; copy is data |
| 9 | yes | English code; copy in English and Spanish |

## 4. Module / file structure

```
apps/landing/app/landing.tsx                 page composition, APP_URL, meta, alternates
apps/landing/app/content/{types,en,es}.ts    LandingCopy type and both languages
apps/landing/app/sections/
  Header.tsx Hero.tsx Problem.tsx HowItWorks.tsx Features.tsx Guardrails.tsx Concurrency.tsx
  Teams.tsx Portable.tsx Pricing.tsx Faq.tsx Closing.tsx Terminal.tsx (shared terminal frame)
apps/landing/app/*.test.ts(x)
```

## 5. Data

None.

## 6. Contracts

Links: `${APP_URL}/auth/login?action=register` (Start free), `${APP_URL}/auth/login` (Sign in),
`mailto:hello@eduindev.com?subject=SDD%20Vault%20demo` (Book a demo).

## 7. Decisions

- **Native `<details>` FAQ** with "one open at a time" added in the browser. Rejected: state-only accordion,
  which hides answers when JavaScript is off.
- **Text links in `--primary-700`** as the design does, to lift contrast above the 600 teal.

## 8. Security

Static page; no user input.

## 9. Test strategy

Prerender tests for every section heading, plan, FAQ question, link target and command in both languages;
FAQ unit test; copy shape test (both languages have the same keys); Lighthouse in `make e2e`.
