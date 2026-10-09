# Development

## Environment

Purra is a single Rust binary package. The minimum supported Rust version is
1.96.0. The repository selects that toolchain and requires the `rustfmt` and
`clippy` components through `rust-toolchain.toml` so local and CI validation
exercise the declared minimum.

The 1.x baseline is verified on Apple silicon macOS with:

- `rustc 1.96.0`
- `cargo 1.96.0`
- `clippy 0.1.96`
- `rustfmt 1.9.0`

`Cargo.lock` is version-controlled for reproducible application and
installation builds.

## Dependency Roles

Runtime dependencies are intentionally limited to capabilities already required
by the product specification:

- `clap` parses the command-line interface.
- `anstream` and `anstyle` provide terminal-aware styled diagnostics.
- `walkdir` supports recursive and non-recursive directory traversal.
- `ignore` compiles gitignore-style globs for hierarchical `.gitignore` rules
  and explicit exclusions; Purra retains control over traversal and errors.
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
typography, dense AI typography, dense scalar-to-text expansions,
regional-indicator, file-read-and-replace, directory-discovery, and atomic-write
workloads. Atomic-write cases compare approximately 1 MiB writes with and without
backups, preparing and cleaning up each fixture outside the measured operation.
Directory-filter cases cover nested `.gitignore` rules and both visiting and
pruning a 2,048-file subtree. Fixtures are created outside the timed iterations;
discovery includes rule loading and compilation.
Do not treat these synthetic cases as a performance claim; record a hardware
baseline and add representative real data before setting regression thresholds.

## GitHub Actions

`.github/workflows/ci.yml` runs on pushes to `main`, version tags, pull requests,
and manual dispatches. The quality job checks formatting, Clippy, unit and
integration tests, and documentation tests.

After quality succeeds, native runners test and build release binaries for:

| Artifact | Runner | Rust target |
| --- | --- | --- |
| `purra-macos-arm64` | `macos-26` | `aarch64-apple-darwin` |
| `purra-linux-x86_64` | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` |
| `purra-linux-arm64` | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` |

Each artifact contains a compressed `purra` executable and is retained for 14
days. These workflow artifacts validate native release builds; installation is
provided through Cargo from a versioned GitHub tag.
