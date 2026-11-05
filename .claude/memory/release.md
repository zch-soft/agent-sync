# Release

Current published line: **1.2.0** (`Cargo.toml`), plus the Homebrew formula commit
`0266212`. The `agentstow` 1.x/2.0.x lines are yanked; the project ships as
`agent-sync-sh` with the binary named `agent-sync`.

## Entry points

- `docs/release-runbook.md` — the authority: version bump, tag, CI, four registries,
  fleet rollout, and how to recover a half-published release.
- `.github/workflows/release.yml` — tag-driven publish for crates.io, npm (seven
  platform packages plus the launcher), PyPI (seven wheels), and the Homebrew tap.
- `.github/workflows/ci.yml` — fmt, clippy `-D warnings`, the full suite on Linux,
  macOS and Windows, and the 1.97 MSRV check.

## Pre-tag gate

1. `cargo fmt --all --check`
2. `cargo clippy --locked --all-targets -- -D warnings`
3. `cargo test --locked`
4. `cargo +1.97 check --locked`
5. `bash scripts/verify-packaging.sh`
6. `cargo check --target x86_64-pc-windows-msvc` when touching platform code

## Rules

- Never tag, publish, or dispatch a release workflow without an explicit request.
- A version cannot be re-uploaded to crates.io, npm or PyPI. If a publish fails
  partway, follow the runbook's recovery section rather than bumping silently.
- The release journal entry in `DECISIONS.md` records what was verified and what was
  left alone; keep that shape.
