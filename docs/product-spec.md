# Product Specification

## Purpose

Purra is a high-performance command-line tool for replacing unwanted Unicode
characters in large volumes of text. Its primary use cases are:

1. CI/CD checks and cleanup for AI-assisted documentation.
2. Ad hoc cleanup of files, directories, streams, and pasted text.

The initial release targets the developer's current macOS environment and uses
stable Rust.

## Compatibility Contract

Version 1.1 retains every stable 1.0 CLI mode, valid preset, exit status, and
public Rust API. It adds scalar-to-text rules and an ASCII preset. The 1.0
scalar-to-scalar library types are deprecated but remain functional until 2.0.
Changes that break existing callers, commands, or valid presets require a new
major version. Additive changes may be released in minor versions.

Version 1.2 deliberately revises CLI directory selection to respect
`.gitignore` by default. `--no-gitignore` restores previous selection when no
explicit exclusions are supplied. This accepted default change does not alter
the existing public Rust APIs, preset grammar, or replacement semantics.

Version 1.3 adds `--no-backup`, `-q` / `--quiet`, and repeatable `-v` /
`--verbose` as opt-in controls. Existing default output, backup behavior, public
Rust APIs, and exit statuses remain unchanged.

## Replacement Model

- Every rule maps exactly one Unicode scalar value to a non-empty replacement
  string. A multi-scalar grapheme such as a complete flag cannot be a source but
  may be replacement text.
- Rules are literal; regular expressions and multi-scalar sources are out of
  scope.
- All rules are applied as one non-transitive mapping. A character produced by
  a rule is not processed again during the same run.
- Keys must be unique. Multiple keys may map to the same value, which allows
  several typographic variants to share one normalized representation.
- An empty replacement and a rule that maps a character to itself are invalid.

For example, with `a=b` and `b=c`, original `a` becomes `b` and original `b`
becomes `c`; the produced `b` is not processed a second time. Processing is
defined by each original input character only.

## Presets

Purra supports four sources of replacement rules:

- `--ai-preset` selects the built-in preset shipped with the tool.
- `--ascii-preset` selects the AI preset plus explicit ASCII expansions.
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

The built-in ASCII preset includes every AI preset rule and additionally
normalizes these groups:

| Sources | Destination |
| --- | --- |
| `…` | `...` |
| `‥` | `..` |
| `ﬀ`, `ﬁ`, `ﬂ`, `ﬃ`, `ﬄ`, `ﬅ`, `ﬆ` | their ASCII letter sequences |
| `→`, `⟶` | `->` |
| `←`, `⟵` | `<-` |
| `↔`, `⟷` | `<->` |
| `⇒`, `⟹` | `==>` |
| `⇐`, `⟸` | `<==` |
| `⇔`, `⟺` | `<==>` |
| `≠` | `!=` |
| `≤` | `<=` |
| `≥` | `>=` |
| `≡` | `===` |

The ASCII preset deliberately leaves `≈`, `×`, `÷`, `±`, `¬`, `∧`, `∨`, `Æ`,
`Œ`, and `ß` unchanged because their ASCII representation is ambiguous or
language-dependent.

Inline rules are comma-separated `K=V` pairs. External preset files contain one
pair per line; blank lines and lines whose first non-whitespace character is `#`
are ignored. Surrounding whitespace is ignored, so a literal space must use
`\s`.

The supported escapes are `\n`, `\r`, `\t`, `\0`, `\s`, `\\`, `\=`, `\,`,
`\#`, and `\u{HEX}` with one to six hexadecimal digits.

An unescaped `=` separates source and replacement. A replacement containing
`=` must escape it; for example, `≠=!\=` defines `≠` to `!=`.

### Validation Guarantee

Purra must parse and validate the complete selected preset before reading or
modifying any target input. It must fail without partial processing when the
preset contains any syntax error, malformed pair, source that is not exactly one
Unicode scalar, empty replacement, duplicate key, or no-op mapping.

An invalid preset is a usage/configuration error and exits with status `2`.

## Input and Output Modes

The intended interfaces include:

```sh
cat file | purra --ai-preset
cat file | purra --ascii-preset
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
and create a unique hidden sibling backup by default. Backups are named
`.NAME.purra.bak`, `.NAME.purra.bak.1`, and so on, and are retained until the
user removes them. Generated backups are excluded from directory scans.

`--no-backup` disables backup creation for directory processing and same-file
input/output, while preserving atomic replacement and file permissions. It
does not remove existing backups. The flag is accepted in every input mode;
it has no effect on stdin/stdout, distinct output paths, or dry runs. Unchanged
in-place files and declined replacements are not written and create no backup.

File-to-file output is also written atomically. An existing output keeps its own
permissions; a new output inherits the input permissions. When input and output
resolve to the same file, the operation uses the in-place backup workflow.
`--no-backup` also applies to that workflow.

## Directory Exclusions

Directory scans respect `.gitignore` in the input directory and, with `-r`,
visited subdirectories. This also applies without `-r` and during `--dry-run`.
Each file's patterns are relative to its containing directory. Later matching
rules in one file take precedence; rules in deeper directories override
matching ancestor rules. Comments, escaping, and `!` negation use gitignore
semantics. An excluded directory is pruned, so its contents and nested ignore
files are not visited and cannot be re-included by negation.

The repeatable `--ignore <GLOB>` adds exclusions relative to the scan root,
using the same glob syntax rather than Rust regex or filesystem-based type
detection. For example:

```sh
purra --ai-preset -r -f . --ignore '*.pdf' --ignore '.git/'
purra --ai-preset -r -f docs/ --ignore '/assets/' --ignore 'drafts/old.txt'
purra --ai-preset -r --dry-run . --no-gitignore --ignore '*.pdf'
```

A leading `/` anchors to the scan root, not the filesystem root. Patterns with
no leading or internal `/` can match names at any depth; a trailing `/`
restricts the pattern to directories and excludes their subtrees. Paths need
not exist for a pattern to be valid. Literal glob characters must be escaped.
All explicit exclusions accumulate independently of flag order and take
precedence over every `.gitignore` rule. An unescaped leading `!` is invalid in
`--ignore`; `\!` matches a literal exclamation mark.

`--no-gitignore` disables loading every `.gitignore` while keeping explicit
exclusions active. Both new flags require one directory input; stdin,
file-to-stdout, and file-to-file modes reject them with exit status `2`.
Explicit file inputs continue to bypass `.gitignore` selection.

Only `.gitignore` inside the scan tree is loaded. Parent ignore files, global
Git settings, `.git/info/exclude`, and `.ignore` do not affect selection. The
rules work outside Git repositories and apply regardless of tracked status.
Hidden files, including `.gitignore` itself, remain eligible unless excluded;
there is no implicit exclusion of `.git`. Symbolic-link ignore files are not
read. A present non-regular, non-symlink `.gitignore` is a runtime error.

The CLI validates the preset first, compiles explicit exclusions, and collects
the complete sorted file list before reading target content or making changes.
An invalid applicable ignore rule aborts with status `2`; diagnostics identify
the CLI pattern or ignore file and line. Ignore-file inspection, reading
(including invalid UTF-8), and traversal failures use status `3`. Rules inside
pruned directories are not validated. Excluded entries produce no findings,
confirmation prompts, skip warnings, updates, or backups.

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

## Diagnostic Verbosity

Without verbosity options, existing output behavior is retained. New controls
are:

- `-q` / `--quiet` suppress warnings, update messages, and summaries. Dry-run
  findings, transformed stdout text, errors, confirmation prompts, and messages
  requesting a valid confirmation response remain visible.
- `-v` / `--verbose` report each input's result: replacement count, no matches,
  skip reason, or declined confirmation. A final summary includes processed,
  skipped, and declined input counts, the replacement count, and the number of
  inputs with replacements.
- `-vv` (or two `--verbose` options) additionally show each match in original
  source order, with its original path, line, Unicode-scalar column, and
  replacement. For directory processing, these proposals precede confirmation
  and remain visible even if the user declines. Further repetitions use the
  same detail level.

Quiet and verbose options are mutually exclusive in either order. Combining
them is a usage error with exit status `2`. The long quiet option is `--quiet`;
`--quite` is not an alias.

All additional logs use stderr and honor `--color`. Transformed content and
dry-run findings remain on stdout. Dry runs never duplicate individual findings
on stderr, even with `-vv`.

A processed input is a successfully inspected UTF-8 text input, including an
unchanged input or one whose replacement was declined. Skipped inputs are
counted separately. Normal processing counts only replacements successfully
written; a declined input contributes no replacements. Dry-run statistics count
all findings. A count measures original matched Unicode scalar values, not
replacement-string length. Verbosity options do not change exit statuses or
confirmation requirements.

## Performance

Performance comparable to Ruff is a product goal, but it is not yet a measurable
acceptance criterion. Dataset sizes, hardware baselines, throughput targets,
memory limits, and regression thresholds must be defined from real workloads
before performance claims are made. The repository includes compile-checked
Criterion cases for clean ASCII, sparse and dense AI typography, dense
scalar-to-text expansion, regional-indicator flag scalars, file loading plus
replacement, directory discovery, nested `.gitignore` rules, and pruning a
2,048-file subtree; results have not yet been recorded.
Atomic-write cases compare approximately 1 MiB writes with and without a backup,
with fixture setup and cleanup outside the measured operation.
