# Selective sync

`agent-sync sync` normally converges every detected Target to the whole Commons. Four
flags narrow that run without changing the all-or-nothing safety rules:

```text
--agent NAME                     repeatable; sync only these Targets
--root PATH                      repeatable; alternate config root for the one --agent
--only FAMILY[:ITEM[,ITEM...]]   repeatable; a surface, or named items in it
--strict                         audit every Target and surface before writing
```

No selector means the whole machine, exactly as before.

## Families and item identities

`--only` accepts one of these families, optionally followed by a comma-separated item
list:

| Family | Item identity |
| --- | --- |
| `skills` | directory name under `skills/` |
| `commands` | command file stem; covers both the commands fan-out and rendered commands |
| `agents` | agent file stem under `agents/` |
| `instructions` | no items — it is one whole surface |
| `mcp` | server name in `mcp.json` |
| `hooks` | event name; the `hooks/<Event>.toml` file stem |

```sh
agent-sync sync --only skills
agent-sync sync --only skills:research,tdd
agent-sync sync --only mcp:codegraph
agent-sync sync --only hooks:SessionStart
agent-sync sync --only commands:ship
```

`--only` is repeatable; the selectors merge. A whole-family selector wins over an item
list for that surface:

```sh
agent-sync sync --only skills:research --only mcp:codegraph
agent-sync sync --only skills:research --only skills     # all skills
```

`--agent` is repeatable too, and composes with `--only`:

```sh
agent-sync sync --agent claude --only mcp:codegraph
agent-sync sync --agent claude --agent pi --only skills:research
```

Unknown families, empty item lists, unknown item names, and
`--only instructions:<item>` are errors. They are reported before the lock and before
any write.

## Abbreviated names

Family and item names accept any prefix long enough to be unique, compared
case-insensitively:

```sh
agent-sync sync --only s                 # skills
agent-sync sync --only SKILLS:RES        # research
agent-sync sync --only mcp:SER           # server named serena
agent-sync sync --only hooks:sess        # SessionStart
```

An exact name always wins: `skills:ima` selects `ima` even when `ima-skills` also
exists. A prefix that matches more than one candidate is an error listing the
candidates, so type more characters; a prefix that matches none is an unknown-name
error. The same rules apply to skills, commands, agents, MCP server names and hook
events.

## Alternate roots

`--root` supplies an alternate config root for **exactly one** selected agent. It is the
way to treat profiles of the same agent as separate Targets:

```sh
agent-sync sync --agent hermes --root ~/.hermes/profiles/work
agent-sync sync --agent hermes --root ~/.hermes/profiles/work --root ~/.hermes/profiles/home
```

- A root may be absolute, start with `~/` (or `~\`), or be home-relative. The root must
  already exist: agent-sync never creates an agent root.
- **Every** path of that agent moves under the alternate root. Paths under the agent's
  registry root lose that prefix; paths outside it are still placed there:
  - `.hermes/skills` → `<root>/skills`
  - `.claude/skills` → `<root>/skills`
  - `.claude.json` → `<root>/.claude.json`
- Repeating `--root` creates one Target per root. Reports label them
  `hermes (<root>)`, so two profiles are distinguishable.
- A **Native** surface cannot move: Codex and opencode still read `~/.agents/skills`
  directly. For those agents `--root` moves only the paths agent-sync writes.
- `--root` is refused for a `[custom.<name>]` Target: a custom Target already declares
  its root in `agent-sync.toml`.
- The default root is left untouched.

## Strict auditing

By default a narrowed run surveys only the selected Targets, surfaces and items.
Problems outside the selection are not read and do not block:

```sh
agent-sync sync --only skills          # a broken mcp.json elsewhere does not block
```

`--strict` audits every detected Target and every surface first, exactly as a full sync
would, and blocks all writes if any of them has a problem. The write set is still the
selection:

```sh
agent-sync sync --only skills --strict
```

With no selectors, `--strict` is the normal full sync.

## Dry run

`--dry-run` prints the same filtered plan the real run executes, and changes nothing:

```sh
agent-sync sync --only skills:research --dry-run
```

## What a narrowed run leaves alone

- Unselected items in the same directory are not linked, not rewritten, and not pruned.
- Dangling or Foreign links are reported only when their name was selected.
- With an item filter, rendered-command orphan pruning is skipped: an orphan has no
  Commons name to select. A whole-family `--only commands` still prunes them.
- A Native agent's legacy fan-out directory is pruned only for selected skill names.
