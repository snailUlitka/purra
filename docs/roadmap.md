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

## Stable 1.0 Contract

Version 1.0 treats the documented CLI surface, preset grammar, replacement
semantics, exit statuses, and public Rust API as stable. Backward-incompatible
changes to those interfaces require a new major version.

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

- Support platforms beyond the developer's current macOS environment only
  after the first implementation establishes portable behavior and tests.
- Add platform-specific test coverage before claiming support beyond macOS.

## Current Non-Goals

- Regular-expression replacement.
- Multi-character search or replacement strings.
- Recursive directory traversal unless explicitly requested with `-r`.
- Following symbolic links.
- Claiming a performance threshold before representative measurements exist.
