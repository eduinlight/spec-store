# Gate evidence — spec 005 (FR-27)

Recorded on 2026-10-22 on the development machine. Each failure was injected into the working tree, the target was run, and the change was reverted with `git checkout` before the next run.

| Injected change | Target | Exit code |
|---|---|---|
| Rust crate spec-store-core: unused variable | `make lint` | 2 |
| Rust crate spec-store-core: reverted | `make lint` | 0 |
| TypeScript app: type error | `make lint` | 2 |
| TypeScript app: reverted | `make lint` | 0 |
| Rust crate spec-store-core: badly formatted function | `make fmt-check` | 2 |
| Rust crate spec-store-core: reverted | `make fmt-check` | 0 |
| TypeScript app: badly formatted constant | `make fmt-check` | 2 |
| TypeScript app: reverted | `make fmt-check` | 0 |
| Rust crate spec-store-core: failing assertion | `make test` | 2 |
| Rust crate spec-store-core: reverted | `make test` | 0 |
| TypeScript app: failing assertion | `make test` | 2 |
| TypeScript app: reverted | `make test` | 0 |
