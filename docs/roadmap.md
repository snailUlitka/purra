# Roadmap

This roadmap records confirmed direction, not delivery dates.

## Implemented 1.0 Release

The stable-Rust macOS implementation now:

- performs validated, non-transitive character-to-character replacement;
- supports standard input/output, file-to-file processing, and in-place
  directory processing;
- provides built-in, external-file, and inline presets;
- supports non-recursive traversal by default, `-r` recursion, and `-f`
  confirmation bypass;
- provides an `rg`-style dry run suitable for CI;
- protects in-place writes with atomic replacement, permission preservation,
  and backups;
- includes initial synthetic engine, file, and directory benchmarks.

It also exposes the replacement engine, preset handling, and safe file
operations as a reusable library rather than coupling them to the CLI.

## Implemented 1.1 Release

Version 1.1 extends the replacement destination from one scalar to non-empty
text while keeping each source exactly one Unicode scalar. It adds:

- the `TextRule`, `TextEngine`, `TextFinding`, and `TextPreset` library API;
- multi-scalar destinations in external and inline CLI presets;
- an `--ascii-preset` that includes the unchanged AI preset plus explicit
  ellipsis, Latin ligature, arrow, and comparison-operator expansions;
- dry-run diagnostics and a benchmark case for expanding replacements.

The 1.0 `Rule`, `Engine`, `Finding`, and `Preset` types remain functional and
are deprecated with migration guidance.

## Implemented 1.2 Release

- CLI directory scans respect root and nested `.gitignore` by default, with or
  without recursion and during dry runs.
- Repeatable `--ignore <GLOB>` adds gitignore-style exclusions;
  `--no-gitignore` disables ignore-file loading without disabling explicit rules.
- Complete discovery and rule validation precede target processing. Excluded
  directories are pruned and invalid applicable rules stop the run.
- `DirectoryOptions`, `DirectoryError`, and
  `collect_directory_files_with_options` expose filtering through the library.
  The original file API retains its unfiltered discovery behavior.
- Compile-checked benchmarks cover nested ignore rules and subtree pruning.

## Stable 1.x Contract

Version 1.0 treats the documented CLI surface, preset grammar, replacement
semantics, exit statuses, and public Rust API as stable. Backward-incompatible
changes to those interfaces require a new major version. Version 1.1 only adds
accepted preset values, CLI options, and public types; existing valid inputs
retain their previous behavior.

Version 1.2 deliberately revises CLI directory selection. `--no-gitignore`
restores previous selection when no explicit exclusions are supplied. Existing
public Rust APIs remain unchanged.

The minimum supported Rust version is 1.96.0 and is enforced by package
metadata and the repository toolchain selection.

## Next Decisions

- Evaluate future AI preset additions against real generated-text samples and
  keep the Unicode-versioned normalization groups documented.
- Define representative real-world workload sizes, hardware baselines,
  throughput targets, memory limits, and performance regression thresholds.
- Measure the checked-in benchmarks before deciding whether streaming, memory
  mapping, a different lookup structure, or parallel directory traversal is
  justified.

## Later Considerations

- In 2.0, remove the deprecated scalar-to-scalar library types and rename the
  text-capable `TextRule`, `TextEngine`, `TextFinding`, and `TextPreset` types to
  `Rule`, `Engine`, `Finding`, and `Preset`. The resulting API will have one
  rule model: one Unicode scalar source to non-empty replacement text.

- Support platforms beyond the developer's current macOS environment only
  after the first implementation establishes portable behavior and tests.
- Add platform-specific test coverage before claiming support beyond macOS.

## Current Non-Goals

- Regular-expression replacement.
- Multi-scalar search strings.
- Recursive directory traversal unless explicitly requested with `-r`.
- Following symbolic links.
- Claiming a performance threshold before representative measurements exist.
