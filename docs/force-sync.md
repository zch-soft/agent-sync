# Forced sync

`sync` normally leaves several states alone because they hold somebody else's
content. `--force` overrides the ones with a Commons definition:

```sh
agent-sync sync --force --dry-run     # preview the forced plan
agent-sync sync --force --yes         # perform it
agent-sync sync --force -y            # same, short form
```

`--force` respects `--agent`, `--only` and `--root`; it only acts inside the
selected scope.

## What is forced

### Link families: skills, commands, agents

| State | Normal | `--force --yes` | Risk |
| --- | --- | --- | --- |
| `variant-identical` | No action | Remove the real file/directory and link the Commons | Low: contents are identical |
| `variant` (diverged) | No action | Remove the real file/directory and link the Commons | **High: the local content is lost** |
| `foreign` | No action | Remove the foreign symlink and link the Commons | Low: the link target itself is untouched |

A `foreign` link with **no Commons entry of that name** is not forceable: there
is nothing to point it at, so it stays a bystander.

### instructions (AGENTS.md)

| State / route | Normal | `--force --yes` | Risk |
| --- | --- | --- | --- |
| `conflict`, Symlink or Rules-dir route | Report only | Remove the real file/directory and create the Commons link | **High** |
| `conflict`, Import-line route (our link points at another Commons file) | Report only | Remove the link and write a file holding the import line | Low |
| `conflict`, Include-entry route (the agent's own config is malformed or the wrong shape) | Report only | **Not forced** — rewriting the whole config would destroy settings agent-sync does not own | — |
| `foreign`, Symlink or Rules-dir route | Report only | Remove the foreign link and create the Commons link | Low |

### rendered commands

| State | Normal | `--force --yes` | Risk |
| --- | --- | --- | --- |
| `foreign` | Report only | Overwrite the unmarked file with the Commons render | **High: the hand-written content is lost** |

## What is not forced

- **MCP** `foreign` and `stranded` stay report-only. `foreign` has no Commons
  definition to write; `stranded` is an allowlist decision and is cleared with
  `agent-sync mcp remove`.
- **Hooks** `foreign` stay report-only; removing them would delete other tools'
  hooks.
- A link or file whose name has no Commons entry is never deleted by
  `--force`; normal sync still prunes its own marked orphans.
- Commons Co-tenants are never touched.

## Confirmation

`--force` without `--yes` and without `--dry-run` first prints the whole plan,
including every item it would delete or overwrite, then refuses to write:

```text
variant  research — force: discard the local copy and link the Commons

error: refusing to force 1 change(s) without --yes — rerun with --yes, or preview with --dry-run
```

- `--force --yes` performs the plan.
- `--force --dry-run` never writes and needs no `--yes`.
- `--yes` without `--force` is a CLI error.
- When no forceable item exists, `--force` behaves like a normal sync and does
  not ask for `--yes`.
