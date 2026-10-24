# Spec 006 — SDD Vault: rename and staging

Status: approved · Created: 2026-10-24 · Constitution: v3.0 · Ticket: SV-21

## 1. Context and goal

The product is renamed **SDD Vault**. The design handoff of 2026-10-24 (`SDD Vault App/Landing/Sign In`, brand guide with logo, app icon and favicons) and the owner's decisions of the same day fix the new name for everything people see and every technical identifier: crates, images, CLI binary, MCP server name, environment variables, Keycloak realm and clients, hostnames, the GitHub repository and the CD repository. At the same time nothing reaches staging: the CD repository exists only on the owner's machine, so every deploy run since spec 005 failed at the CD bump, and the cluster has no namespace, secrets or ArgoCD Application for the product. This spec renames the product and brings staging up, so every later change is delivered to staging when it merges. It also amends the constitution to admit the services and the design system that the following tickets need.

## 2. Users / actors

- Visitor and signed-in person: see the SDD Vault name, logo and icons everywhere.
- Agent and CLI user: call the MCP server named `sdd-vault` and the CLI `sddvault`.
- Owner (operator of the platform): merges to main and finds the change served on staging without manual steps.

## 3. Current state

| Current piece | What it existed for | Decision | Justification |
|---|---|---|---|
| Name `spec-store` in crates, images, realm, hosts, env vars, texts | Product name until 2026-10-24 | replace | Owner decision: "rename everything" (SV-21) |
| CD repository `eduinlight-org/spec-store-cd`, local only | GitOps manifests (spec 005 T81–T88) | replace | Published as `eduinlight-org/sdd-vault-cd` |
| Specs 001–005 | Records of past work | keep | Historical records keep their wording |
| Constitution v2.0 (PostgreSQL and Keycloak only services; shadcn/ui) | Stack limits | replace | v3.0 admits Redis, RabbitMQ, S3-compatible storage and the TapTap design system |

## 4. User stories (by priority)

### US-1 (P1) — One name everywhere
As a visitor or member I want to see SDD Vault, its logo and its icons on every page, so the product I sign up for is the one I use.
Independent test: no page of landing, app, Keycloak or email shows "Spec Store"; each page links the favicon set.

### US-2 (P1) — Merges reach staging
As the owner I want each merge to main to be served on staging without manual steps, so I can check every change there.
Independent test: a merge builds the images, bumps the CD repository and ArgoCD serves the new version on the staging hostnames.

## 5. Functional requirements (EARS)

- **FR-1** (US-1): THE SYSTEM SHALL show the name "SDD Vault" in every text of the landing page, the app, the Keycloak pages and the emails, and SHALL show no text "Spec Store".
- **FR-2** (US-1): THE SYSTEM SHALL link on every page of the landing page, the app and the Keycloak theme the favicons (16, 32, 64), the apple-touch icon and the theme colour `#1F4FE0`, and SHALL show the horizontal logo of the brand guide in the landing and Keycloak headers.
- **FR-3** (US-1): THE SYSTEM SHALL name its deliverables `sdd-vault-<deliverable>` (images `ghcr.io/eduinlight/sdd-vault-<deliverable>`), its CLI binary `sddvault`, its MCP server `sdd-vault`, its environment variables `SDD_VAULT_*`, its Keycloak realm `sdd-vault` and clients `sdd-vault-app`, `sdd-vault-cli`, `sdd-vault-api-admin`.
- **FR-4** (US-2): THE SYSTEM SHALL serve staging at `staging-sddvault.eduindev.com` (landing), `staging-app-sddvault.eduindev.com` (app), `staging-api-sddvault.eduindev.com` (API), `staging-mcp-sddvault.eduindev.com` (MCP) and `staging-auth-sddvault.eduindev.com` (identity provider), and production at the same names without the `staging-` prefix.
- **FR-5** (US-2): WHEN a change is merged to main, THE SYSTEM SHALL build each changed deliverable, record its new image in the staging overlay of `eduinlight-org/sdd-vault-cd`, and serve it on staging within 30 minutes without manual steps.
- **FR-6** (US-2): THE SYSTEM SHALL keep staging's database, realm and secrets separate from production's, and SHALL send staging email through the platform's SMTP relay.

## 6. Non-functional requirements

- **NFR-1**: The rename changes no behaviour: every existing test passes with the new names.
- **NFR-2**: No secret value is written to either repository.

## 7. Key entities

- Deliverable: landing, app, api, mcp, keycloak, backup — each with an image and a Deployment.

## 8. Edge cases

- Old bookmarks to `spec-store` hostnames: not served (no production users yet).
- The GitHub repository rename: GitHub redirects the old remote URL; workflows use the new image names.
- A deploy run that starts before the CD repository exists fails at the bump; the run of the next merge succeeds.

## 9. Out of scope

- Production rollout (only on a version tag, unchanged).
- Vector re-export of the brand assets (the PNGs of the handoff are used as they are).
- Any change of behaviour or screen layout; the design of each screen arrives with its own ticket.

## 10. Completion criteria

- Every FR covered by a green test or, for FR-4 to FR-6, by a recorded check of the pipeline and the staging hostnames.

## 11. Assumptions

- The VPS routes and terminates TLS for every `*.eduindev.com` hostname (owner, 2026-10-24).
- Staging borrows the cluster's SMTP relay credentials and the GHCR pull secret of another namespace; it gets its own Garage key and bucket (owner, 2026-10-24).
