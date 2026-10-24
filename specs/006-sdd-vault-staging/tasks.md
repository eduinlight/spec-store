# Tasks — Spec 006

Plan: `specs/006-sdd-vault-staging/plan.md` · Date: 2026-10-24

Format: `- [ ] Tnn [P] (FR-x) Description — Done when: verifiable criterion`

## Phase 1 — Rename
- [x] T01 (FR-1, FR-3) Add a test in `e2e/brand.spec.ts` that the landing, the app sign-in redirect and the Keycloak page show "SDD Vault", no "Spec Store", and link the favicon set — Done when: test written and red
- [x] T02 (FR-1, FR-3, FR-4) Apply the ordered rename of plan §2 to every file outside `specs/001-*`…`specs/005-*` and `.claude/`, rename the realm, users and theme files, set the CLI binary to `sddvault` — Done when: `git grep -iE 'spec[-_ ]?store'` outside those paths is empty and `make lint test` pass
- [x] T03 (FR-2) Copy the brand assets into `packages/ui/src/assets/brand`, add favicons, apple-touch icon and `theme-color` to the landing, the app and the Keycloak login template, and the horizontal logo to the landing and Keycloak headers — Done when: T01 green

## Phase 2 — Staging
- [x] T04 (FR-4, FR-5, FR-6) Rename the CD repository manifests (namespaces `sdd-vault`/`sdd-vault-staging`, images, hosts, realm, theme), add `object-storage-credentials` to `create-secrets.sh`, document the bootstrap, publish it as `eduinlight-org/sdd-vault-cd` — Done when: `kustomize build staging` and `kustomize build prod` succeed and the repository exists on GitHub
- [x] T05 (FR-5, FR-6) Bootstrap staging: Garage key and bucket, `sdd-vault-staging` secrets, ArgoCD AppProject and staging Application, `CD_TOKEN` secret on the app repository, GitHub repository renamed — Done when: ArgoCD lists `sdd-vault-staging` and the secrets exist
- [ ] T06 (FR-4, FR-5) Merge and record the pipeline and the five staging hostnames in `specs/006-sdd-vault-staging/validation.md` — Done when: every staging hostname answers with the new version
