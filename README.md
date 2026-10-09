# Purra

Purra is a Rust library and CLI for validated, non-transitive replacement of
Unicode scalar values with non-empty text. It supports stdin/stdout pipelines,
file output, safe in-place directory processing, and CI-friendly dry runs.

## Installation

Install the stable release directly from its GitHub tag:

```sh
cargo install --git https://github.com/snailUlitka/purra --tag v1.3.0 --locked purra
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
purra --ai-preset --dry-run -q docs/
purra --ai-preset -r -f docs/
purra --ai-preset -r -f . --ignore '*.pdf' --ignore '.git/'
purra --ai-preset -r -f . --no-gitignore --ignore '*.pdf'
purra --ai-preset -r -f --no-backup -v docs/
purra --ascii-preset -vv input.md > output.md
purra --in-place-preset "a=b,c=d" input.txt output.txt
```

A directory is processed in place. It is non-recursive and asks before each
changed file by default; `-r` enables recursion and `-f` bypasses confirmation.
Every in-place change creates a uniquely named hidden sibling backup by default.
`--no-backup` disables backup creation while retaining atomic writes and file
permissions. Existing backups are kept. The flag is accepted in every mode and
has no effect when no in-place write occurs, including dry runs.

Directory scans now respect `.gitignore` in the scan root and visited
subdirectories by default, including non-recursive scans and dry runs. This
deliberately changes directory selection from the 1.1 release.
`--no-gitignore` restores unfiltered selection unless explicit exclusions are
supplied. Parent ignore files, global Git settings, `.git/info/exclude`, and
`.ignore` are not loaded; a Git repository is not required.

Repeat `--ignore <GLOB>` to add exclusions using gitignore-style glob syntax,
not regular expressions. Patterns are relative to the input directory:
`*.pdf` matches names at any depth, `/assets/` excludes only the root's assets
directory, `assets/` excludes directories with that name at any depth, and
`docs/file.txt` excludes that relative path. Files need not exist when a pattern
is supplied. Quote patterns to prevent shell expansion; escape glob characters
when matching literal names, for example `--ignore 'data\[1\].txt'`.

Explicit exclusions accumulate and cannot be cancelled by `.gitignore` rules.
Negation with a leading `!` is rejected in `--ignore`; use `\!` for a literal
exclamation mark. `.gitignore` files support their usual comments, escapes, and
`!` negation. `--no-gitignore` leaves explicit exclusions active. Both flags
require a directory input and are rejected for stdin or explicit file inputs.
Excluded directories are not visited. Invalid applicable rules stop processing
with exit status `2` before any target content is read or changed; ignore-file
I/O failures use status `3`.

Dry runs produce `path:line:column` findings and exit with status `1` when a
problem is present, making them suitable for CI checks.

| Output option | Behavior |
| --- | --- |
| `-q`, `--quiet` | Hide warnings, update messages, and summaries; keep dry-run findings, transformed text, errors, and confirmation prompts. |
| `-v`, `--verbose` | Report each input's result and counts, plus totals for processed, skipped, and declined inputs and replacements. |
| `-vv` | Also show every match with its original position and replacement before confirmation or writing. Further repetitions use this same level. |

Quiet and verbose options conflict and exit with status `2` when combined.
Logs use stderr; transformed text and dry-run findings use stdout. Verbose dry
runs count findings and do not repeat individual findings on stderr.
`--color auto|always|never` controls styling in findings and diagnostics.

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

`purra::files::collect_directory_files_with_options` exposes directory filtering
through `DirectoryOptions` (`recursive`, `respect_gitignore`, and `ignores`) and
returns `DirectoryError`. Defaults enable `.gitignore` without recursion or
explicit exclusions. The original `collect_directory_files(root, recursive)`
and `FileError` retain their behavior and do not apply ignore rules.

See [`docs/README.md`](docs/README.md) for the complete product, architecture,
and development documentation.

## Stability and License

Version 1.1 extends preset destinations to non-empty text without removing the
stable 1.0 interfaces. Version 1.2 adds directory filtering with the
deliberate CLI default change described above; existing Rust APIs, replacement
semantics, and preset grammar remain unchanged.

Version 1.3 adds optional backup suppression and diagnostic verbosity controls
without changing default behavior or existing Rust APIs.

Purra is available under the [`MIT License`](LICENSE).
