# Roadmap

This roadmap records confirmed direction, not delivery dates.

## Implemented First Deliverable

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

## Next Decisions

- Expand the conservative built-in AI preset only with mappings that preserve
  the unique-destination invariant.
- Define representative real-world workload sizes, hardware baselines,
  throughput targets, memory limits, and performance regression thresholds.
- Measure the checked-in benchmarks before deciding whether streaming, memory
  mapping, a different lookup structure, or parallel directory traversal is
  justified.
- Decide whether the preset grammar and CLI surface are ready to be treated as
  stable public interfaces.

## Later Considerations

- Support platforms beyond the developer's current macOS environment only
  after the first implementation establishes portable behavior and tests.
- Set a minimum supported Rust version only when the dependency and release
  strategy makes it useful.
- Add platform-specific test coverage before claiming support beyond macOS.

## Current Non-Goals

- Regular-expression replacement.
- Multi-character search or replacement strings.
- Recursive directory traversal unless explicitly requested with `-r`.
- Following symbolic links.
- Claiming a performance threshold before representative measurements exist.
