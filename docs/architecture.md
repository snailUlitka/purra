# Architecture

## Library-First Boundary

Purra is one Cargo package with both a reusable library and a binary target:

- `src/text_engine.rs` owns the primary scalar-to-text replacement algorithm and
  finding positions.
- `src/text_preset.rs` owns the extended preset grammar, complete validation,
  loading, and the built-in AI and ASCII presets.
- `src/engine.rs` and `src/preset.rs` retain the deprecated 1.0 scalar-to-scalar
  library API until 2.0.
- `src/files.rs` owns file classification, deterministic directory discovery,
  hierarchical ignore rules, atomic output, in-place replacement, and backups.
- `src/main.rs` owns only CLI argument mapping, stdin/stdout interaction,
  confirmation, diagnostic rendering, and process exit statuses.

Another interface should depend on the public library types rather than call or
copy CLI code. `TextEngine` is independent of the filesystem and can be retained
and reused across many inputs.

## Engine

`TextEngine::new` accepts validated `TextRule` values. ASCII sources are compiled
into a 128-entry direct lookup table. Non-ASCII sources are stored as a sorted
boxed slice and found with binary search. Replacement walks the original UTF-8
input once after locating the first match and returns borrowed input when
unchanged.

Every lookup uses the original input scalar value, so rule chains are
non-transitive even when replacement text contains another configured source.
Findings contain the original scalar, replacement text, byte offset, and
one-based line and scalar-column position. A finding count measures matched
source scalars, not the number of produced scalars.

Purra deliberately defines a character as a Unicode scalar value, not a user-
perceived grapheme cluster. A flag such as `🇺🇸` contains two regional-indicator
scalars. A whole flag cannot be a rule source, but it can be replacement text.

## Preset Validation

Preset parsing produces all rules before a `TextEngine` is constructed.
Validation rejects empty presets, malformed separators or escapes, sources that
do not decode to one scalar, empty replacements, duplicate sources, and no-op
mappings. Multiple sources may share replacement text. The CLI constructs the
preset and engine before reading any target input.

The 1.0 `Rule`, `Engine`, `Finding`, and `Preset` types remain available and
deprecated in 1.x. Version 2.0 is planned to remove them and rename the text-
capable types to the unprefixed names, leaving one scalar-to-text API.

## File Processing

Files are currently read into memory. A NUL byte classifies input as binary;
otherwise the complete content must be valid UTF-8. Directory discovery is
deterministic, includes hidden regular files, does not follow symbolic links,
and filters Purra-generated backup files.

The CLI uses `collect_directory_files_with_options` with `DirectoryOptions`:
recursion disabled, `.gitignore` enabled, and no explicit exclusions by default.
`DirectoryError` distinguishes invalid rules from file failures without adding
variants to the existing `FileError`. The legacy `collect_directory_files`
function retains its original unfiltered behavior.

Filtered discovery retains `WalkDir` and uses `ignore::gitignore::GitignoreBuilder`
to compile patterns. Explicit exclusions are compiled once, reject negation,
and take priority over ignore-file rules. A depth-scoped stack holds matchers
for `.gitignore` in the root and visited directories. The deepest matching
scope decides selection; leaving a directory removes its scope. Excluded
directories are pruned before loading their rules. Parent and global ignore
sources are never consulted, and a Git repository is not required.

Ignore files are inspected without following symbolic links and read line by
line so errors include their source location. Every compilation failure aborts
discovery, including failures after valid rules. Discovery finishes and sorts
the complete list before the CLI reads any target content, asks for confirmation,
or writes files. `--no-gitignore` skips ignore-file loading while retaining
explicit exclusions. Hidden control files remain eligible as normal inputs.

For an in-place replacement, the library:

1. rejects symbolic-link output;
2. writes and synchronizes a temporary sibling file;
3. applies the original permissions to the temporary file;
4. copies the original to a unique hidden sibling backup;
5. atomically persists the temporary file over the original.

Backups use `.NAME.purra.bak`, then `.NAME.purra.bak.1`, `.2`, and so on. They
are retained until explicitly removed by the user.

The CLI selects `replace_in_place` by default. With `--no-backup`, it selects
the existing `write_atomic` operation on the original path instead: this retains
the temporary sibling, synchronization, original permissions, symlink rejection,
and atomic persistence, but omits the backup copy. This selection applies both
to directories and same-file input/output. Public file-operation signatures and
the backup guarantee of `replace_in_place` are unchanged.

## CLI Diagnostics

The CLI keeps verbosity and per-run statistics in a shared diagnostic reporter.
Default output remains compatible; quiet mode suppresses optional diagnostics
without hiding errors, confirmations, transformed text, or dry-run findings.
Verbose modes add per-input results and totals. Declined text inputs are counted
as processed but do not contribute to applied-replacement totals.

Individual matches use one renderer for stdout dry-run findings and stderr
verbose proposals, with stream-aware colors and original scalar positions.
Normal transformation only collects detailed findings at verbosity level two or
higher. Dry runs reuse their existing findings and never duplicate them on
stderr. Replacement lookup and the public library API are unaffected.

## Future Performance Work

The current whole-file design is intentional for a simple, safe first version.
Streaming, memory mapping, and parallel traversal require benchmark evidence and
must preserve UTF-8 boundary handling, deterministic diagnostics, and the
validate-before-input invariant.
