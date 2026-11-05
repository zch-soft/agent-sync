---
name: release-verify
description: Verify an agent-sync release without publishing — version consistency across Cargo/npm/Homebrew, the pre-tag gate, packaging dry runs, CI status, and the runbook checklist. Use when preparing a release, checking whether a version is ready, or recovering a failed publish.
---

# Release verification

The authority is `docs/release-runbook.md`. This skill is the short path through it.

## Never without explicit authorization

`git tag`, `git push`, `cargo publish`, `gh release`, and any publishing workflow are
irreversible in practice: crates.io, npm and PyPI do not allow re-uploading a version.
A human authorizes each one by name in the task.

## Pre-tag checklist

1. **Version consistency** — `Cargo.toml`, `Cargo.lock`, `Formula/agent-sync.rb`, and
   any version literal in `scripts/` and `site/` agree on `X.Y.Z`. The tree's
   `npm/agent-sync-sh/package.json` is deliberately stamped by `build-npm.sh` at
   build time, so a lagging version there is normal; check the generated package
   instead.
2. **Gate**
   ```bash
   cargo fmt --all --check
   cargo clippy --locked --all-targets -- -D warnings
   cargo test --locked
   cargo +1.97 check --locked
   ```
3. **Packaging** — `bash scripts/verify-packaging.sh`. It builds, assembles the seven
   npm platform packages, installs offline from local tarballs, and dry-runs every
   publish. It needs `npm` and `python`.
4. **Platform** — `cargo check --target x86_64-pc-windows-msvc` when platform code
   changed. The full Windows e2e (`scripts/e2e-windows.ps1`) needs elevation and
   network: propose it, do not launch it unprompted.
5. **CI** — `ci` must be green on Linux, macOS, Windows and the 1.97 MSRV job.
6. **Docs** — README install lines, the site, and the release journal entry in
   `DECISIONS.md` describe what actually shipped. The journal entry states what was
   verified, on which hosts, and what was left alone.

## Failure handling

If a publish fails partway, follow the runbook's "Recovering a half-published
release". Do not bump the version to hide a partial publish; some registries already
hold that version permanently.

## Report

A checklist of passed / failed / skipped checks, each with the command or URL that
proves it. Name the next irreversible step and who must authorize it.
