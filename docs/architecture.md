# Architecture

## Library-First Boundary

Purra is one Cargo package with both a reusable library and a binary target:

- `src/engine.rs` owns the pure replacement algorithm and finding positions.
- `src/preset.rs` owns preset grammar, complete validation, loading, and the
  built-in AI preset.
- `src/files.rs` owns file classification, deterministic directory discovery,
  atomic output, in-place replacement, and backups.
- `src/main.rs` owns only CLI argument mapping, stdin/stdout interaction,
  confirmation, diagnostic rendering, and process exit statuses.

Another interface should depend on the public library types rather than call or
copy CLI code. `Engine` is independent of the filesystem and can be retained and
reused across many inputs.

## Engine

`Engine::new` accepts validated `Rule` values. ASCII sources are compiled into a
128-entry direct lookup table. Non-ASCII sources are stored as a sorted boxed
slice and found with binary search. Replacement walks the original UTF-8 input
once after locating the first match and returns borrowed input when unchanged.

Every lookup uses the original input scalar value, so rule chains are
non-transitive. Findings contain the original and replacement scalar, byte
offset, and one-based line and scalar-column position.

Purra deliberately defines a character as a Unicode scalar value, not a user-
perceived grapheme cluster. A flag such as `🇺🇸` contains two regional-indicator
scalars. A whole flag cannot be one side of a rule, but its two components can be
mapped independently.

## Preset Validation

Preset parsing produces all rules before an `Engine` is constructed. Validation
rejects empty presets, malformed separators or escapes, sides that do not decode
to one scalar, duplicate sources, and no-op mappings. Multiple sources may share
one destination so typographic variants can be normalized to the same character.
The CLI constructs the preset and engine before reading any target input.

## File Processing

Files are currently read into memory. A NUL byte classifies input as binary;
otherwise the complete content must be valid UTF-8. Directory discovery is
deterministic, includes hidden regular files, does not follow symbolic links,
and filters Purra-generated backup files.

For an in-place replacement, the library:

1. rejects symbolic-link output;
2. writes and synchronizes a temporary sibling file;
3. applies the original permissions to the temporary file;
4. copies the original to a unique hidden sibling backup;
5. atomically persists the temporary file over the original.

Backups use `.NAME.purra.bak`, then `.NAME.purra.bak.1`, `.2`, and so on. They
are retained until explicitly removed by the user.

The current whole-file design is intentional for a simple, safe first version.
Streaming, memory mapping, and parallel traversal require benchmark evidence and
must preserve UTF-8 boundary handling, deterministic diagnostics, and the
validate-before-input invariant.
