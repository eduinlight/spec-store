# CI evidence — spec 005 (FR-28)

The workflow `.github/workflows/ci.yml` runs on GitHub-hosted `ubuntu-24.04` runners (research R25) and calls `make lint`, `make fmt-check` and `make test`. Before the workflow existed on `main`, two short-lived branches ran the same workflow with their own name added to the push trigger:

| Branch | Change | Run | Result |
|---|---|---|---|
| `ci-probe-ok` | none | https://github.com/eduinlight/spec-store/actions/runs/39865715852 | success |
| `ci-probe-fail` | `let unused = 1;` in `packages/core/src/lib.rs` | https://github.com/eduinlight/spec-store/actions/runs/39865719503 | failure at the Lint step (`unused variable`) |

Both branches were deleted after the runs.

Runs on `main`:

| Commit | Run | Result |
|---|---|---|
| `a1e3808` merge of #5 | https://github.com/eduinlight/spec-store/actions/runs/39923145831 | failure: a fresh checkout has no `.env`, so `make test` ran without `DATABASE_URL` |
| `7502efa` merge of #6, which creates `.env` through its own make rule | https://github.com/eduinlight/spec-store/actions/runs/39924983232 | success, 2026-10-23 08:29–08:40 UTC |

The failure on `a1e3808` came from moving every local credential into a generated `.env`: the Makefile's fallback `DATABASE_URL` went away, and `.env` was created by `make env` only after make had read the empty include. CI does not run on pull requests, so it showed up on `main`. #6 was checked with a manual run on its branch (https://github.com/eduinlight/spec-store/actions/runs/39923892462) before it was merged.
