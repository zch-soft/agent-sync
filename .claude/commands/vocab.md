---
description: Check the current change against CONTEXT.md vocabulary
allowed-tools: Bash(cargo test:*), Read, Grep, Glob, Bash(git diff:*)
---

Check the working tree's user-facing language against `CONTEXT.md`.

1. Run `cargo test --locked --test vocabulary`.
2. Read `git diff` and list every added or changed string literal, doc sentence, CLI
   message, and identifier that names a domain concept.
3. Verify each uses its glossary term and avoids the entry's _Avoid_ words. Flag
   "Store" anywhere user-visible.
4. Remember the test only scans string literals in `src/`; prose and identifiers are
   yours to review.

Report violations as `file:line` with the glossary term that applies. If a needed
concept has no term, say so instead of inventing a synonym.
