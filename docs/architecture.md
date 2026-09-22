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
  atomic output, in-place replacement, and backups.
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
