# Purra

Purra is a Rust library and CLI for validated, non-transitive replacement of
Unicode scalar values in text. It supports stdin/stdout pipelines, file output,
safe in-place directory processing, and CI-friendly dry runs.

## Installation

Install the stable release directly from its GitHub tag:

```sh
cargo install --git https://github.com/snailUlitka/purra --tag v1.0.0 --locked purra
```

Cargo installs the executable into its configured binary directory, which is
`$HOME/.cargo/bin` by default.

## Build

```sh
cargo build --release
```

The executable is written to `target/release/purra`.

GitHub Actions validates the project and publishes packaged workflow artifacts
for modern Apple Silicon macOS, Linux x86_64, and Linux ARM64.

## CLI Examples

```sh
cat document.md | purra --ai-preset
purra --ai-preset input.md output.md
purra --ai-preset input.md > output.md
purra --preset typography.preset input.md output.md
purra --ai-preset --dry-run docs/
purra --ai-preset -r -f docs/
purra --in-place-preset "a=b,c=d" input.txt output.txt
```

A directory is processed in place. It is non-recursive and asks before each
changed file by default; `-r` enables recursion and `-f` bypasses confirmation.
Every in-place change creates a uniquely named hidden sibling backup.

Dry runs produce `path:line:column` findings and exit with status `1` when a
problem is present, making them suitable for CI checks.

## Presets

Inline presets use comma-separated `K=V` pairs. Preset files use one pair per
line and permit blank lines and `#` comments. Each side must decode to exactly
one Unicode scalar value.

Supported escapes are `\n`, `\r`, `\t`, `\0`, `\s` (space), `\\`, `\=`, `\,`,
`\#`, and `\u{HEX}`. Duplicate sources, no-op mappings, malformed escapes, and
multi-scalar graphemes such as a complete flag are rejected before any input is
read or modified. Multiple sources may share one destination.

## Library API

```rust
use purra::{Engine, Rule};

let engine = Engine::new([
    Rule::new('—', '-'),
    Rule::new('🇺', '🇨'),
    Rule::new('🇸', '🇦'),
])?;

let result = engine.replace("Fast — 🇺🇸");
assert_eq!(result.text, "Fast - 🇨🇦");
assert_eq!(result.count, 3);
# Ok::<(), purra::EngineError>(())
```

See [`docs/README.md`](docs/README.md) for the complete product, architecture,
and development documentation.

## Stability and License

Version 1.0 stabilizes the CLI, preset grammar, exit status contract, and public
Rust API. Backward-incompatible changes to those interfaces require a new major
version.

Purra is available under the [`MIT License`](LICENSE).
