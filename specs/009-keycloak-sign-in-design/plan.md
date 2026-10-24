# Technical plan — Spec 009

Spec: `specs/009-keycloak-sign-in-design/spec.md` · Constitution: v3.0 · Date: 2026-10-24

## 1. Summary

Override only the frame of the keycloak.v2 login theme: `login/template.ftl` copied from Keycloak 26.8.1 and
changed to the two-column layout (left column: logo, title, Keycloak's `kcContent` with each page's form, the
switch footer and terms note; right panel: the enforced-rules story). Keycloak's page templates (`login.ftl`,
`register.ftl`, …) stay untouched and are restyled by `css/sdd-vault.css` on the TapTap tokens. New message
keys in `messages_en.properties` and `messages_es.properties` carry every new text.

## 2. Technical context

Keycloak 26.8.1 keycloak.v2 login theme (PatternFly 5 markup), FreeMarker; the template source comes from
`org.keycloak.keycloak-themes-26.8.1.jar` (`theme/keycloak.v2/login/template.ftl`).

## 3. Constitution check

| Principle | Complies | Note |
|---|---|---|
| 2 | yes | No new dependency |
| 3 | yes | e2e tests updated first |
| 4 | yes | One override (the frame); page templates inherited |

## 4. Files

`deploy/keycloak/themes/sdd-vault/login/template.ftl` (new), `resources/css/sdd-vault.css`,
`messages/messages_{en,es}.properties`, `e2e/keycloak-theme.spec.ts`, `e2e/brand.spec.ts`.

## 5. Decisions

- **Override `template.ftl` only.** Rejected: overriding every page template (more to keep in step with
  Keycloak upgrades), and a Keycloakify React theme (new toolchain and dependency for a static layout).
- The page knows whether it is sign-in or registration from `pageId` (`login.ftl`, `register.ftl`).

## 6. Test strategy

Playwright: each page in English and Spanish shows the right panel texts, the logo link, the title, and the
right footer; narrow viewport has no horizontal overflow; existing sign-in, sign-up and email flows stay green.
