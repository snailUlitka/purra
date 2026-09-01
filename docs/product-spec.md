# Product Specification

## Purpose

Purra is a high-performance command-line tool for replacing unwanted Unicode
characters in large volumes of text. Its primary use cases are:

1. CI/CD checks and cleanup for AI-assisted documentation.
2. Ad hoc cleanup of files, directories, streams, and pasted text.

The initial release targets the developer's current macOS environment and uses
stable Rust.

## Compatibility Contract

As of version 1.0, the documented CLI options and modes, preset grammar,
replacement semantics, exit statuses, and public Rust API are stable. Changes
that break existing callers, commands, or valid presets require a new major
version. Additive changes may be released in minor versions.

## Replacement Model

- Every rule maps exactly one Unicode scalar value to exactly one Unicode scalar
  value. A multi-scalar grapheme such as a complete flag is not one character
  under this contract.
- Rules are literal; regular expressions and multi-character strings are out of
  scope.
- All rules are applied as one non-transitive mapping. A character produced by
  a rule is not processed again during the same run.
- Keys must be unique. Multiple keys may map to the same value, which allows
  several typographic variants to share one normalized representation.
- A rule that maps a character to itself is invalid.

For example, with `a=b` and `b=c`, original `a` becomes `b` and original `b`
becomes `c`; the produced `b` is not processed a second time. Processing is
defined by each original input character only.

## Presets

Purra supports three sources of replacement rules:

- `--ai-preset` selects the built-in preset shipped with the tool.
- `--preset <path>.preset` loads an external preset.
- `--in-place-preset "a=b,c=d"` supplies rules directly on the command line.

The built-in AI preset normalizes these groups:

| Sources | Destination |
| --- | --- |
| all Unicode 17.0 `Dash_Punctuation` characters except ASCII hyphen-minus, plus minus sign `U+2212` | hyphen-minus `-` |
| all Unicode 17.0 `Space_Separator` characters except ASCII space | space |
| `«`, `»`, `“`, `”`, `„`, `‟` | ASCII double quote `"` |
| `‘`, `’`, `‚`, `‛` | ASCII apostrophe `'` |

Tabs, line and paragraph separators, zero-width spaces, and soft hyphens are
not part of those groups and remain unchanged.

Inline rules are comma-separated `K=V` pairs. External preset files contain one
pair per line; blank lines and lines whose first non-whitespace character is `#`
are ignored. Surrounding whitespace is ignored, so a literal space must use
`\s`.

The supported escapes are `\n`, `\r`, `\t`, `\0`, `\s`, `\\`, `\=`, `\,`,
`\#`, and `\u{HEX}` with one to six hexadecimal digits.

### Validation Guarantee

Purra must parse and validate the complete selected preset before reading or
modifying any target input. It must fail without partial processing when the
preset contains any syntax error, malformed pair, non-character key or value,
duplicate key, or no-op mapping.

An invalid preset is a usage/configuration error and exits with status `2`.

## Input and Output Modes

The intended interfaces include:

```sh
cat file | purra --ai-preset
purra --ai-preset input output
purra --ai-preset input > output
purra --preset custom.preset input output
purra --ai-preset path/to/directory
purra --in-place-preset "a=b,c=d" input output
```

Behavior by input shape:

- With no input path, Purra reads standard input and writes standard output.
- With one file path, Purra reads that file and writes standard output.
- With input and output file paths, Purra writes the transformed content to the
  output path.
- A directory is processed in place, one file at a time, without recursion by
  default.
- `-r` enables recursive directory traversal.
- Directory processing asks for `y/n` confirmation before each file replacement.
- `-f` disables per-file confirmation.

In-place writes use an atomic replacement strategy, preserve file permissions,
and create a unique hidden sibling backup. Backups are named
`.NAME.purra.bak`, `.NAME.purra.bak.1`, and so on, and are retained until the
user removes them. Generated backups are excluded from directory scans.

File-to-file output is also written atomically. An existing output keeps its own
permissions; a new output inherits the input permissions. When input and output
resolve to the same file, the operation uses the in-place backup workflow.

## File Classification

- Binary files are ignored.
- Invalid UTF-8 inputs are skipped with a warning.
- Symbolic links are not followed and are skipped with a warning.
- Hidden regular files are processed under the same rules as other files.

A NUL byte classifies a file as binary. Invalid UTF-8 and symbolic links produce
warnings; binary files are silently ignored.

## Dry Run

`--dry-run` reports matches without changing data. Its output should resemble
`rg` diagnostics:

- each finding identifies `path:line:column` when a path is available;
- offending characters are color-highlighted when color output is enabled;
- a summary reports the detected problems;
- exit status is `1` when at least one problem is found.

Columns count Unicode scalar values. `--color auto|always|never` controls ANSI
styling.

Exit status `0` means that processing or checking completed without findings.
Exit status `2` is reserved for invalid configuration or usage, including an
invalid preset. Fatal I/O and runtime failures use status `3`.

## Performance

Performance comparable to Ruff is a product goal, but it is not yet a measurable
acceptance criterion. Dataset sizes, hardware baselines, throughput targets,
memory limits, and regression thresholds must be defined from real workloads
before performance claims are made. The repository includes compile-checked
Criterion cases for clean ASCII, sparse and dense AI typography,
regional-indicator flag scalars, file loading plus replacement, and directory
discovery; results have not yet been recorded.
