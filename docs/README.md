# Purra Documentation

This directory is the durable project knowledge base. It records confirmed
product decisions, implemented architecture, development workflows, and
deliberately deferred work.

## Documents

- [`product-spec.md`](product-spec.md) defines the intended user-facing behavior,
  replacement rules, safety guarantees, and exit status contract.
- [`roadmap.md`](roadmap.md) records delivered scope and deliberately deferred
  decisions.
- [`development.md`](development.md) records the verified Rust toolchain,
  dependency roles, and local validation workflow.
- [`architecture.md`](architecture.md) describes the library-first module
  boundaries, data flow, and safe-write sequence.

## Maintenance

Update these documents when a product decision changes or verified repository
behavior makes a statement obsolete. Do not document speculative architecture,
commands, or platform support as established behavior. Update development
commands when the corresponding tooling changes and verify them before
documenting them.
