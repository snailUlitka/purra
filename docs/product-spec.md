# Product Specification

## Purpose

Purra is a high-performance command-line tool for replacing unwanted Unicode
characters in large volumes of text. Its primary use cases are:

1. CI/CD checks and cleanup for AI-assisted documentation.
2. Ad hoc cleanup of files, directories, streams, and pasted text.

The initial release targets the developer's current macOS environment and uses
stable Rust.

## Replacement Model

- Every rule maps exactly one Unicode character to exactly one Unicode
  character.
- Rules are literal; regular expressions and multi-character strings are out of
  scope.
- All rules are applied as one non-transitive mapping. A character produced by
  a rule is not processed again during the same run.
- Keys must be unique. Values must also be unique.

For example, with `a=b` and `b=c`, original `a` becomes `b` and original `b`
becomes `c`; the produced `b` is not processed a second time. Processing is
defined by each original input character only.

## Presets

Purra will support three sources of replacement rules:

- `--ai-preset` selects a built-in preset shipped with the tool. It will include
  replacements for long dashes and similar AI-associated typography. Its exact
  contents are deferred.
- `--preset <path>.preset` loads an external preset.
- `--in-place-preset "a=b,c=d"` supplies rules directly on the command line.

External and inline rules use `K=V` pairs. The exact file layout, whitespace
rules, comment syntax, and escaping for delimiters or control characters remain
to be designed.

### Validation Guarantee

Purra must parse and validate the complete selected preset before reading or
modifying any target input. It must fail without partial processing when the
preset contains any syntax error, malformed pair, non-character key or value,
duplicate key, or duplicate value.

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

In-place writes must use an atomic replacement strategy, preserve file
permissions, and create a backup. The backup naming and retention policy remain
to be designed before implementation.

## File Classification

- Binary files are ignored.
- Invalid UTF-8 inputs are skipped with a warning.
- Symbolic links are not followed and are skipped with a warning.
- Hidden regular files are processed under the same rules as other files.

The exact binary-detection method and warning format remain implementation
decisions.

## Dry Run

`--dry-run` reports matches without changing data. Its output should resemble
`rg` diagnostics:

- each finding identifies `path:line:column` when a path is available;
- offending characters are color-highlighted when color output is enabled;
- a summary reports the detected problems;
- exit status is `1` when at least one problem is found.

Exit status `0` means that processing or checking completed without findings.
Exit status `2` is reserved for invalid configuration or usage, including an
invalid preset. Additional fatal I/O status semantics will be specified during
CLI design.

## Performance

Performance comparable to Ruff is a product goal, but it is not yet a measurable
acceptance criterion. Implementation work must introduce representative
benchmarks for both streaming and file/directory workloads. Dataset sizes,
hardware baselines, throughput targets, memory limits, and regression thresholds
must be defined from real workloads before performance claims are made.
