# Roadmap

This roadmap records confirmed direction, not delivery dates.

## First Meaningful Deliverable

Design and implement a stable-Rust macOS CLI that:

- performs validated, non-transitive character-to-character replacement;
- supports standard input/output, file-to-file processing, and in-place
  directory processing;
- provides built-in, external-file, and inline presets;
- supports non-recursive traversal by default, `-r` recursion, and `-f`
  confirmation bypass;
- provides an `rg`-style dry run suitable for CI;
- protects in-place writes with atomic replacement, permission preservation,
  and backups;
- includes benchmarks representative of large text-cleanup workloads.

## Before Implementation

Resolve and document:

- the complete built-in AI preset;
- the external preset file grammar;
- escaping rules for inline and file-based presets;
- backup naming and retention behavior;
- fatal I/O exit statuses and the exact diagnostic format;
- representative workload sizes and benchmark methodology.

These decisions must be made before their affected interfaces are treated as
stable.

## Later Considerations

- Support platforms beyond the developer's current macOS environment only
  after the first implementation establishes portable behavior and tests.
- Set a minimum supported Rust version only when the dependency and release
  strategy makes it useful.
- Add architecture and validation documentation after real code, commands, and
  feedback loops exist.

## Current Non-Goals

- Regular-expression replacement.
- Multi-character search or replacement strings.
- Recursive directory traversal unless explicitly requested with `-r`.
- Following symbolic links.
- Claiming a performance threshold before representative measurements exist.
