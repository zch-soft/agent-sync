# Environment

## Local setup

- Windows 11, Git Bash. Creating symlinks requires Developer Mode (Settings → System
  → For developers) or an elevated shell; without either, fixtures fail with OS
  error 1314. That is an environment failure, not a product regression.
- Rust stable via `rustup`; `rust-toolchain.toml` pins the channel. The MSRV is
  checked with `cargo +1.97 check --locked`, never by changing the default toolchain
  — `rust-toolchain.toml` outranks `rustup default`.
- CodeGraph is indexed at `.codegraph/`. Use `codegraph_explore` (or the CLI) before
  a grep/read loop. Re-index after large moves with `codegraph sync`.

## Commands

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo +1.97 check --locked
```

`cargo test` includes integration tests that create real links in a temporary tree.
On Windows, run it from an elevated shell or with Developer Mode enabled.

## Packaging

- `bash scripts/verify-packaging.sh` — builds, assembles npm packages, installs
  offline from local tarballs, and dry-runs publishes. Needs `npm` and `python`.
- `bash scripts/build-npm.sh <out-dir> <target>` — assembles one platform package.
- `bash scripts/build-wheels.py` — builds the PyPI wheels; `--help` lists targets.
- `bash scripts/check-fallback-wheel.sh` — the fallback-wheel assertion shared with
  `release.yml`.
- `powershell -File scripts/e2e-windows.ps1 -Version <v>` — full Windows end-to-end;
  needs elevation and network. Do not run it without the user's go-ahead.

## MCP

`.mcp.json` carries only project-relevant servers. Machine-local servers (for example
`ai-memory`, which needs an absolute executable path) stay in the user's global
config, so they never land in this public repository.
