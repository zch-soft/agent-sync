---
description: Implement one issue from the .scratch tracker
argument-hint: <path-or-issue-number>
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(cargo:*), Bash(git:*), mcp__codegraph__codegraph_explore
---

Implement the issue at `$ARGUMENTS`. If that is a number, locate it under
`.scratch/*/issues/`.

1. Read the issue, its spec, `CONTEXT.md`, and the ADRs it cites.
2. Set `**Status:** claimed` in the issue file before editing code (use
   `Status: done` only after the gate passes).
3. Use `codegraph_explore` for the symbols involved; implement the smallest change
   that satisfies every acceptance criterion.
4. Add or extend a test at the `agent_sync::run` seam that proves the behavior.
5. Run `cargo fmt --all --check`, `cargo clippy --locked --all-targets -- -D
   warnings`, and `cargo test --locked`.
6. Flip the issue's status to `done`, appending a short `## Comments` note on what
   changed and what remains unverified.

Do not commit or push. Report the files changed, the gate result, and any criterion
you could not verify.
