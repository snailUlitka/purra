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
- `anyhow` adds context to failures at the application boundary.
- `anstream` and `anstyle` provide terminal-aware styled diagnostics.
- `walkdir` supports recursive and non-recursive directory traversal.
- `tempfile` supports atomic replacement workflows.

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

Run all tests:

```sh
cargo test --all-targets --all-features
```

Build an optimized executable:

```sh
cargo build --release
```

Do not document a benchmark command as a meaningful validation step until the
repository contains a representative benchmark target and dataset strategy.
