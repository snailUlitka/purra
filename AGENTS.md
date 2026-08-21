# Purra Agent Guide

Purra is a high-performance Rust library and CLI for replacing configured
Unicode characters in large text inputs. The first supported environment is
macOS.

## Documentation

- Start with [`docs/README.md`](docs/README.md) for the documentation map.
- Read [`README.md`](README.md) for the user-facing CLI and library overview.
- Read [`docs/product-spec.md`](docs/product-spec.md) before changing CLI
  behavior, preset semantics, file handling, or exit codes.
- Read [`docs/roadmap.md`](docs/roadmap.md) before selecting implementation
  work or turning a deferred decision into a requirement.
- Read [`docs/development.md`](docs/development.md) for the verified local
  toolchain, dependency roles, and validation commands.
- Read [`docs/architecture.md`](docs/architecture.md) before changing module
  boundaries, replacement lookup, or atomic file handling.
- Keep [`.github/workflows/ci.yml`](.github/workflows/ci.yml) aligned with the
  supported release targets and the commands in `docs/development.md`.

## Current Repository State

The repository contains a reusable Rust library, a thin CLI adapter, unit and
integration tests, and a Criterion benchmark target.

## Working Rules

- Keep replacement semantics character-to-character and non-transitive unless
  the product specification is deliberately revised.
- Validate a complete preset before reading or modifying target files.
- Preserve the documented atomic-write, permission, and backup guarantees when
  changing in-place processing.
- Treat performance as a product requirement. Add representative benchmarks
  alongside the implementation rather than relying on unmeasured claims.
- Keep repository documentation and agent instructions in English.
- Update the documentation index whenever maintained documents are added,
  renamed, or removed.

## Validation

Run the project checks from the repository root:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --lib --bins --tests --all-features
git diff --check
```

`cargo clippy --all-targets` compiles the benchmark target. Do not run
`cargo bench` unless benchmark execution is part of the current task.

For documentation-only changes, `git diff --check` plus a manual repository-link
check is sufficient.
