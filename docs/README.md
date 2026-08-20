# Purra Documentation

This directory is the durable project knowledge base. It records confirmed
product decisions separately from implementation details that do not exist yet.

## Documents

- [`product-spec.md`](product-spec.md) defines the intended user-facing behavior,
  replacement rules, safety guarantees, and exit status contract.
- [`roadmap.md`](roadmap.md) records the approved initial direction and decisions
  deliberately deferred until design or implementation work begins.
- [`development.md`](development.md) records the verified Rust toolchain,
  dependency roles, and local validation workflow.

## Maintenance

Update these documents when a product decision changes or verified repository
behavior makes a statement obsolete. Do not document speculative architecture,
commands, or platform support as established behavior. Update development
commands when the corresponding tooling changes and verify them before
documenting them.
