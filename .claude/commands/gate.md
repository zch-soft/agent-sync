---
description: Run the full pre-commit gate (fmt, clippy, tests) and report failures
allowed-tools: Bash(cargo:*), Read, Grep, Glob
---

Run the project gate and nothing else:

1. `cargo fmt --all --check`
2. `cargo clippy --locked --all-targets -- -D warnings`
3. `cargo test --locked`

If any step fails, diagnose the root cause before touching code. Fix only failures
caused by the current change; do not silence a lint with `allow` or weaken a test to
get green. Then rerun the whole gate from the top.

Report each step as passed or failed with the exact failing line. If a failure is
environmental (for example Windows symlink privilege, OS error 1314), say so rather
than reporting the suite as red.
