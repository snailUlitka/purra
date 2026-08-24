# Development

## Environment

Purra is a single Rust binary package. The repository selects the stable Rust
channel and requires the `rustfmt` and `clippy` components through
`rust-toolchain.toml`.

The scaffold was verified on Apple silicon macOS with:

- `rustc 1.96.0`
- `cargo 1.96.0`
- `clippy 0.1.96`
- `rustfmt 1.9.0`

These versions describe the initial environment; they are not a minimum
supported Rust version. `Cargo.lock` is part of the scaffold and should be
version-controlled for reproducible application builds.

## Dependency Roles

Runtime dependencies are intentionally limited to capabilities already required
by the product specification:

- `clap` parses the command-line interface.
- `anstream` and `anstyle` provide terminal-aware styled diagnostics.
- `walkdir` supports recursive and non-recursive directory traversal.
- `tempfile` supports atomic replacement workflows.
- `thiserror` defines typed errors across the public library boundary.

Development dependencies establish the planned feedback loops:

- `assert_cmd` and `predicates` support end-to-end CLI assertions.
- `criterion` supports throughput and regression benchmarks.

Do not add a parser, memory-mapping library, parallel runtime, or specialized
search algorithm until the corresponding preset grammar and benchmark evidence
justify that decision.

## Commands

Format source files:

```sh
cargo fmt
```

Check formatting without changing files:

```sh
cargo fmt --check
```

Compile all targets and feature combinations while treating lints as errors:

```sh
cargo clippy --all-targets --all-features -- -D warnings
```

Run unit and integration tests without executing benchmark targets:

```sh
cargo test --lib --bins --tests --all-features
```

Build an optimized executable:

```sh
cargo build --release
```

Compile benchmarks without running them as part of lint validation:

```sh
cargo clippy --all-targets --all-features -- -D warnings
```

When benchmark execution is explicitly requested, run:

```sh
cargo bench --bench engine
```

The benchmark target covers approximately 1 MiB clean ASCII, sparse AI
typography, dense AI typography, regional-indicator, file-read-and-replace, and
directory-discovery workloads. Do not treat these synthetic cases as a
performance claim; record a hardware baseline and add representative real data
before setting regression thresholds.

## GitHub Actions

`.github/workflows/ci.yml` runs on pushes to `main`, version tags, pull requests,
and manual dispatches. The quality job checks formatting, Clippy, unit and
integration tests, and documentation tests.

After quality succeeds, native runners test and build release binaries for:

| Artifact | Runner | Rust target |
| --- | --- | --- |
| `purra-0.2.0-macos-arm64` | `macos-26` | `aarch64-apple-darwin` |
| `purra-0.2.0-linux-x86_64` | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` |
| `purra-0.2.0-linux-arm64` | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` |

Each artifact contains a compressed `purra` executable and is retained for 14
days. Update the embedded artifact version when preparing a later release.
