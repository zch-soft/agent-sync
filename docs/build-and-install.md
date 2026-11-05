# Building and installing from source

This is the contributor path: build the binary from this checkout and put it on
your `PATH`. For a prebuilt binary, the README's [Install](../README.md#install)
section lists npm, pip, crates.io and Homebrew. Releasing to those channels is
the separate, tag-driven process in the [release runbook](release-runbook.md).

## Toolchain

| What | Value |
| --- | --- |
| Edition | 2024 |
| MSRV | Rust 1.97 (`rust-version` in `Cargo.toml`) |
| Project toolchain | `stable`, pinned by `rust-toolchain.toml` |
| Windows target | `x86_64-pc-windows-msvc` (the MSVC linker is required) |

`rust-toolchain.toml` selects `stable`, so `rustup update stable` is enough on a
development machine. The MSRV is a separate promise, checked in CI with
`cargo +1.97 check`.

## Build

```sh
cargo build
```

produces the debug binary at `target/debug/agent-sync` (`.exe` on Windows). Run
it without installing:

```sh
cargo run -- doctor
```

```sh
cargo build --release
```

produces `target/release/agent-sync` (`.exe` on Windows). The release profile
sets `lto = true` and `strip = true`, so the binary is a single small file with
no debug symbols — about 1.9 MB on Windows.

The crate is named `agent-sync-sh`; the binary it builds is named `agent-sync`.
Both names are explicit in `Cargo.toml`, so `cargo install` and `cargo run`
behave as expected while the published package keeps its registry name.

## Install into `$CARGO_HOME/bin`

```sh
cargo install --path .
```

`--path` says where the **code** comes from, not where the binary goes. Every
`cargo install` puts the executable in `$CARGO_HOME/bin`:

| `CARGO_HOME` | Installed binary |
| --- | --- |
| unset (default) | `~/.cargo/bin/agent-sync` (`%USERPROFILE%\.cargo\bin\agent-sync.exe` on Windows) |
| set | `$CARGO_HOME/bin/agent-sync` |

That directory is on `PATH` on a normal rustup install. Pick another root with
`--root`:

```sh
cargo install --path . --root D:\tools
```

which installs to `D:\tools\bin\agent-sync.exe`.

Installing from the registry instead of this checkout uses the same location:

```sh
cargo install agent-sync-sh
```

Use `--path .` whenever the binary should reflect local changes; the registry
form downloads the published crate at the published version.

### Reinstall after a change

Re-running at the same version is a no-op: cargo reports the package as already
installed and leaves the binary alone. After a version bump `cargo install
--path .` replaces it; add `--force` to reinstall the same version, or to
overwrite a binary cargo does not track.

```sh
cargo install --path . --force
```

Rebuilding alone (`cargo build --release`) does **not** update what `PATH`
resolves; copy `target/release/agent-sync` over the installed file only if you
deliberately want to skip cargo's bookkeeping.

### Remove the binary

```sh
cargo uninstall agent-sync-sh
```

This removes the binary from `$CARGO_HOME/bin`. It does not touch the Commons
(`~/.agents/`), the tool config, the lock, or anything agent-sync fanned out —
those are runtime state, not files the installer owns.

### Where the install went

```sh
cargo install --list
```

lists installed packages, their versions, and — for a `--path` install — the
directory it came from. `which -a agent-sync` (`where agent-sync` in cmd,
`Get-Command agent-sync -All` in PowerShell) shows which copy `PATH` resolves
first when more than one exists.

## Verify the install

```sh
agent-sync --version
```

must print the version in `Cargo.toml`. Then check the machine:

```sh
agent-sync doctor
```

`doctor` reports the resolved Commons, the config path, installed Targets, and
Commons hygiene without changing the Commons or any Target, so it is safe as
the first command after an install.

## Run it after installing

The binary is only the tool; the content lives in the Commons (`~/.agents/`).
A fresh machine usually goes:

```sh
agent-sync init      # create the Commons and report what is already installed
agent-sync doctor    # confirm detection and configuration
agent-sync sync --dry-run
agent-sync sync
```

On Windows, creating symlinks needs Developer Mode (Settings → System → For
developers) or an elevated shell. Without it, `sync` fails with OS error 1314
and names the remedy; that is an environment limitation, not a build problem.

## Cross-building

The release workflow builds six targets from three runners. Locally, add the
target and name it on the build:

```sh
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

The artifact lands in `target/<triple>/release/agent-sync` (`.exe` on Windows).
Cross-building needs the target's linker (and SDK, on macOS) configured;
`win32-arm64` is the one target CI never executes, because an x64 host cannot
run it.

## Pre-commit gate

Run this before committing; CI runs the same checks.

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo +1.97 check --locked
```

`cargo test` drives the public `agent_sync::run` seam against a throwaway tree
and never touches the real home directory. On Windows the symlink fixtures need
Developer Mode or an elevated shell, as above.

## Packaging checks

`bash scripts/verify-packaging.sh` exercises the release pipeline without
publishing: it builds the crate, assembles the npm packages with
`scripts/build-npm.sh`, installs them offline from local tarballs, seals the host
binary into wheels with `scripts/build-wheels.py`, and asserts the
no-install-hook and executable-bit invariants. It is the local companion to the
checks in `.github/workflows/release.yml`; see the release runbook for what each
registry expects.
