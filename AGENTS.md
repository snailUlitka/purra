# Purra Agent Guide

Purra is a high-performance Rust library and CLI for replacing configured
Unicode scalar values with non-empty text in large text inputs. The first
supported environment is macOS.

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

- Keep replacement sources limited to one Unicode scalar value, replacement
  text non-empty, and application non-transitive unless the product
  specification is deliberately revised.
- Validate a complete preset before reading or modifying target files.
- Preserve the documented atomic-write, permission, and backup guarantees when
  changing in-place processing.
- Treat performance as a product requirement. Add representative benchmarks
  alongside the implementation rather than relying on unmeasured claims.
- Keep repository documentation and agent instructions in English.
- Update the documentation index whenever maintained documents are added,
  renamed, or removed.

## Commit and Release Messages

Every commit message must follow this format:

```text
<type>: <short commit summary>

- <completed work>
- <completed work>
```

- Choose an appropriate lowercase type such as `feature`, `fix`, `chore`,
  `docs`, `release`, `refactor`, `perf`, `test`, `build`, or `ci`.
- Keep the summary concise. Separate it from the body with a blank line, and
  use Markdown bullets (`- `) to describe what was added, fixed, or removed.
  Write concrete entries in English and include only applicable change types.
- For a release commit, use a cumulative bullet list covering all changes since
  the previous release tag, excluding that previous release and including the
  current release preparation. For the first release, cover the full history.
  Review the commit history and diff for that range, and consolidate related
  entries without losing distinct changes.
- Create annotated release tags. Use the same cumulative bullet list in the
  release commit body, the tag annotation, and any published release notes.

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
