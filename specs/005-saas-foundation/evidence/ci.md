# CI evidence — spec 005 (FR-28)

The workflow `.github/workflows/ci.yml` runs on GitHub-hosted `ubuntu-24.04` runners (research R25) and calls `make lint`, `make fmt-check` and `make test`. Before the workflow existed on `main`, two short-lived branches ran the same workflow with their own name added to the push trigger:

| Branch | Change | Run | Result |
|---|---|---|---|
| `ci-probe-ok` | none | https://github.com/eduinlight/spec-store/actions/runs/39865715852 | success |
| `ci-probe-fail` | `let unused = 1;` in `packages/core/src/lib.rs` | https://github.com/eduinlight/spec-store/actions/runs/39865719503 | failure at the Lint step (`unused variable`) |

Both branches were deleted after the runs. The first run on `main` is recorded below after the merge.
