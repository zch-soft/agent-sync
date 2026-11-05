---
name: vocabulary-guard
description: Check or change this project's domain vocabulary — CONTEXT.md terms and avoid-lists, user-facing CLI strings, docs, and the tests/vocabulary.rs enforcement. Use before committing anything that adds a message, command, doc section, or identifier, or when asked to review wording.
---

# Vocabulary guard

`CONTEXT.md` is the glossary, and it is binding. The project treats language as part
of the interface because users read CLI output, and contributors read identifiers.

## What to check

1. Every domain concept uses its defined term: Commons, Target, Fan-out, Native,
   Co-tenant, Protocol surface, Variant, Adopt, Sourced, Revert, Foreign, Conflict,
   Render, Managed, Disabled, Tweak, Instructions route, Import-line, Include-entry,
   Rules-dir link, Marker.
2. No entry's _Avoid_ words appear for that concept. The retired word **Store** must
   not appear anywhere user-visible. The retired family name **subagents** survives
   only as the single alias literal in `src/family.rs`.
3. A new concept that is not in the glossary is a signal: either the project already
   has a word for it, or the glossary has a real gap. Do not invent vocabulary in a
   message; propose the glossary entry first.

## What the test enforces

`tests/vocabulary.rs` scans **string literals** in `src/**/*.rs` and fails on:

- `store`, word-bounded, any case;
- `subagents`, word-bounded, with exactly one allowlisted occurrence;
- the phrases `canonical source`, `central repo`, `agent-sync's directory`,
  `zero-config agent`, `linked skill`, `repo-backed`.

The enforced terms are read back out of `CONTEXT.md` at runtime, so deleting a term
from the glossary fails the test until the rule is re-derived.

It deliberately does **not** scan comments, identifiers, docs, `Cargo.toml` or the npm
launcher, and it does not enforce homonyms such as `source`, `linked`, `external`,
`override`, `conflict`, `import`, `add`, `reset` or `delete`. Those rely on review.
Partial enforcement is not full enforcement.

## Workflow

```bash
cargo test --locked --test vocabulary
```

Then read the diff's user-facing strings and docs yourself, against the glossary.
When adding a term, update `CONTEXT.md` (definition plus _Avoid_ list) and, if it is
mechanically checkable, extend `tests/vocabulary.rs` and its doc comment explaining
why the other terms stay review-only.

Report: terms checked, any violation with `file:line`, and whether the test result and
the prose review agree.
