# Purra

Purra is a Rust library and CLI for validated, non-transitive replacement of
Unicode scalar values with non-empty text. It supports stdin/stdout pipelines,
file output, safe in-place directory processing, and CI-friendly dry runs.

## Installation

Install the stable release directly from its GitHub tag:

```sh
cargo install --git https://github.com/snailUlitka/purra --tag v1.1.0 --locked purra
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
cat document.md | purra --ascii-preset
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
line and permit blank lines and `#` comments. A source must decode to exactly
one Unicode scalar value; its replacement may contain one or more scalar values.

`--ai-preset` preserves the 1.0 behavior for dashes, Unicode spaces, and smart
quotes. `--ascii-preset` is its superset and additionally expands ellipses,
typographic Latin ligatures, selected arrows, and comparison operators into
ASCII text. For example, `…`, `⇒`, and `≠` become `...`, `==>`, and `!=`.

Supported escapes are `\n`, `\r`, `\t`, `\0`, `\s` (space), `\\`, `\=`, `\,`,
`\#`, and `\u{HEX}`. Duplicate sources, empty or no-op replacements, malformed
escapes, and multi-scalar sources such as a complete flag are rejected before
any input is read or modified. Replacement text may contain a complete flag,
and multiple sources may share one destination.

## Library API

```rust
use purra::{TextEngine, TextRule};

let engine = TextEngine::new([
    TextRule::new('…', "..."),
    TextRule::new('⇒', "==>"),
    TextRule::new('≠', "!="),
])?;

let result = engine.replace("Wait… A ⇒ B ≠ C");
assert_eq!(result.text, "Wait... A ==> B != C");
assert_eq!(result.count, 3);
# Ok::<(), purra::TextEngineError>(())
```

The 1.0 `Rule`, `Engine`, `Finding`, and `Preset` APIs remain available with
their original behavior but are deprecated in favor of their `Text*`
counterparts. Version 2.0 will remove the legacy types and rename the text-
capable API to the shorter names.

See [`docs/README.md`](docs/README.md) for the complete product, architecture,
and development documentation.

## Stability and License

Version 1.1 extends preset destinations to non-empty text without removing the
stable 1.0 interfaces. Backward-incompatible changes remain reserved for a new
major version.

Purra is available under the [`MIT License`](LICENSE).
