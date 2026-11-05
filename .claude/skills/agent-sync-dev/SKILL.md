---
name: agent-sync-dev
description: Core development workflow for the agent-sync Rust crate. Use for any change under src/ or tests/ — features, bug fixes, refactors, new agent support, or understanding how a command works. Covers the module map, the two-phase sync contract, the run() test seam, and the pre-commit gate.
---

# Developing agent-sync

## Read first

1. `CONTEXT.md` — the vocabulary is binding.
2. The ADR for the area you are touching (`docs/adr/`), plus any `DECISIONS.md` entry
   that names it.
3. The current spec if the work is larger than a bug fix: the highest
   `.scratch/agentstow-v*/spec.md`.

Then use `codegraph_explore` with the symbols that span the change. It returns the
verbatim source and the call path in one call; do not build the picture with a
grep/read loop.

## Module map

| Module | Responsibility |
| --- | --- |
| `lib.rs` | The single seam: `run(args, vars, out, err) -> i32` |
| `cli.rs` | clap argument parsing only |
| `env.rs` | Path resolution for every path the tool touches |
| `config.rs`, `config_edit.rs` | `agent-sync.toml`, format-preserving edits |
| `registry.rs` | Static Target registry — adding an agent is one row |
| `target.rs` | Registry + config + installed state, resolved per run |
| `commons.rs` | Scanning the Commons and classifying entries |
| `family.rs` | Skill/command/agent/hooks/MCP families as a type |
| `link.rs` | Symlink identity and the fan-out engine |
| `instructions.rs` | The four instructions routes |
| `mcp.rs`, `mcp_toml.rs` | MCP render and key-merge (JSON and TOML) |
| `mcp_cmd.rs` | `mcp list/adopt/remove/enable/disable` |
| `hooks.rs` | Hook key-merge by command-string identity |
| `render.rs` | Whole-file render with a Marker |
| `write.rs` | Atomic temp-file + rename for rendered files |
| `sync.rs` | Two-phase survey then execute |
| `status.rs`, `doctor.rs` | Read-only reporting and readiness |
| `adopt.rs`, `revert.rs`, `init.rs` | The lifecycle verbs |
| `lock.rs` | Global advisory lock in the state directory |
| `report.rs` | stdout/stderr discipline and exit codes |

## Contracts that must survive your change

- **No state file.** Ownership comes from link targets and identity rules, never
  bookkeeping.
- **Two-phase sync.** Phase one surveys read-only and reports every problem; any
  problem means nothing is written. `--dry-run` is phase one plus the plan, through
  the same code path.
- **Identity rules.** Links by resolved target; MCP servers by name; hooks by command
  string; rendered files by Marker. Anything else is Foreign and untouched.
- **Refusals precede the lock and the writes.**
- **Exit codes.** 0 clean, 1 error, 2 actionable. `status` is the CI gate, not
  `sync --dry-run`.

## Tests

Integration tests call `agent_sync::run` with a throwaway tree, passing environment
overrides in `vars` (`AGENT_SYNC_TARGET_ROOT`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME`,
`AGENT_SYNC_HOME`). Never touch the real home directory. Use `tests/common/` and
`tempfile`.

Prefer one test that proves the guarantee over many that mirror the implementation.
If your change touches user-facing output, also run the vocabulary test:

```bash
cargo test --locked --test vocabulary
```

## Gate

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

MSRV when the change could raise the floor:

```bash
cargo +1.97 check --locked
```

Do not commit or push unless asked. Report files changed, gate results, and what was
not verified.
