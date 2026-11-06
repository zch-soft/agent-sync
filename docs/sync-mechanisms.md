# Sync mechanisms

Every shared surface is materialized into each Target by one of a small set of
mechanisms. The mechanism itself is fixed per agent in the registry; the one
exception is the **Instructions route**, which the user chooses in
`agent-sync.toml`.

Ownership follows the mechanism: a link that resolves into the Commons, a whole
file carrying the Marker, a server name present in the Commons, a hook whose
command matches a Commons hook. Everything else is Foreign or a Variant and is
never touched.

## At a glance

| Mechanism | Used by | What it does |
| --- | --- | --- |
| Symlink fan-out | skills, commands, agents | One relative symlink per Commons entry into the Target's directory. |
| Render (whole file + Marker) | commands whose native format differs from the Commons | Generate the whole file, first line carrying the Marker. |
| Key-merge (MCP) | mcp | Merge each Commons server into the agent's native config, by server name. |
| Key-merge (hooks) | hooks | Merge each Commons hook into the agent's hook array, by command string. |
| Native / discovery | skills, commands, mcp | Write nothing; the agent reads the Commons or discovers other agents' configs. |
| None | any family | The agent has no surface for this family. |

A Native agent may also carry a **legacy** fan-out directory it still reads
beside the Commons. `sync` visits that directory in prune-only mode: it removes
links of ours, creates nothing, and touches nothing foreign.

## Instructions routes

An **Instructions route** is how one agent reaches the Commons `AGENTS.md`. The
config names two routes, each backed by two mechanisms:

| Mechanism | Route | Operation |
| --- | --- | --- |
| Symlink | `symlink` | Place a symlink to the Commons `AGENTS.md` at the agent's instructions path. |
| Rules-dir link | `symlink` | Drop a symlink to the Commons `AGENTS.md` into a directory the agent globs (Roo's `~/.roo/rules/`). |
| Import-line | `import` | Ensure one additive, idempotent line inside a user-owned file (Claude's `@~/.agents/AGENTS.md` in `CLAUDE.md`). |
| Include-entry | `import` | Ensure one entry naming the Commons `AGENTS.md` in a list the agent's own config declares (opencode's `instructions`, Gemini's `context.fileName`). |

An `import` route is offered only where the agent is measured to consume it: an
unparsed `@path` leaves the agent with no instructions and no error.

### Routes by agent

Each registry row lists the routes its agent can take, default first:

| Agent | Routes | Default | Symlink path |
| --- | --- | --- | --- |
| Claude | `import`, `symlink` | `import` | `~/.claude/CLAUDE.md` |
| opencode | `import`, `symlink` | `import` | `~/.config/opencode/AGENTS.md` |
| Gemini CLI | `import`, `symlink` | `import` | `~/.gemini/GEMINI.md` |
| Codex | `symlink` | `symlink` | `~/.codex/AGENTS.md` |
| pi | `symlink` | `symlink` | `~/.pi/agent/AGENTS.md` |
| oh-my-pi | `symlink` | `symlink` | `~/.omp/agent/AGENTS.md` |
| Windsurf | `symlink` | `symlink` | `~/.codeium/windsurf/memories/global_rules.md` |
| Roo | `symlink` | `symlink` | `~/.roo/rules/` (Rules-dir link) |

OpenClaw, Hermes, Cursor, Cline and WorkBuddy have no instructions surface.

The defaults follow measured behavior: Cowork skips a symlinked
`~/.claude/CLAUDE.md`, so Claude defaults to the Import-line; claude-mem and
Gemini's own memory tool write through their instructions files, so opencode and
Gemini default to the Include-entry; Codex is measured not to expand `@`
imports, so it has no import route. The symlink route remains available for
Claude, opencode and Gemini as an opt-in with those hazards (ADR-0009).

### Choosing a route

```toml
# $XDG_CONFIG_HOME/agent-sync/agent-sync.toml
[instructions]
claude = "symlink"   # "symlink" or "import"
opencode = "import"
```

- Omitted, the agent keeps its registry default.
- Naming a route the agent lacks is a configuration error. `codex = "import"` is
  refused, not written as an inert line.
- A `[custom.<name>]` Target takes fan-out families only and cannot appear here.
- `sync` converges a switch: it removes what the route not taken left of ours,
  but only while the chosen route is not blocked by a Conflict or a Foreign
  link, so a blocked switch never leaves the agent with neither route.
- Claude's two routes share one path, so `sync` swaps one step: our link becomes
  a file holding only the Import-line, or that file becomes our link. Any other
  real `CLAUDE.md` is a Conflict and is never rewritten.

## Skills

- **Fan-out** — one symlink per skill directory. Used by Claude, pi, WorkBuddy,
  OpenClaw and Hermes.
- **Native** — Codex, opencode, oh-my-pi, Gemini and Cursor read
  `~/.agents/skills` directly; nothing is written. Codex's `~/.codex/skills`
  and Cursor's `~/.cursor/skills` are legacy fan-out directories, pruned only.
- **None** — Windsurf, Roo and Cline.

## Commands

- **Fan-out** — one symlink per command file. Claude (`~/.claude/commands`),
  Codex (`~/.codex/prompts`), opencode, Cursor, Windsurf and Roo.
- **Render** — the Commons keeps commands as markdown; Gemini takes them as
  TOML, so the whole file is generated into `~/.gemini/commands` with the Marker
  on its first line. A marked file is ours to overwrite or prune; an unmarked
  file is Foreign. This is the only family that prunes without being asked by
  name.
- **NativeViaDiscovery** — oh-my-pi discovers other agents' command
  directories; writing would duplicate them.
- **None** — pi, OpenClaw, Hermes, Cline and WorkBuddy.

## Agents (subagents)

- **Fan-out** — one symlink per agent file. Claude (`~/.claude/agents`) and
  opencode (`~/.config/opencode/agents`).
- **None** — every other agent.

## MCP

MCP is a **Key-merge by server name**. The Commons `mcp.json` is the definition;
each server is rendered into the agent's native config:

| Agent | File | Root key | Format / dialect |
| --- | --- | --- | --- |
| Claude | `~/.claude.json` | `mcpServers` | JSON / Standard |
| Codex | `~/.codex/config.toml` | `mcp_servers` | TOML / Codex |
| opencode | `~/.config/opencode/opencode.json` | `mcp` | JSON / OpenCode |
| Gemini | `~/.gemini/settings.json` | `mcpServers` | JSON / Gemini |
| Cursor | `~/.cursor/mcp.json` | `mcpServers` | JSON / Standard |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | `mcpServers` | JSON / Windsurf |
| Cline | `~/.cline/mcp.json` | `mcpServers` | JSON / Standard |
| WorkBuddy | `~/.workbuddy/mcp.json` | `mcpServers` | JSON / Standard |

- A name present in the Commons is Managed and the Commons wins; every other
  server name and every other key in the file is Foreign and survives.
- Each dialect renames or restructures fields where the agent requires it.
- `[mcp.<server>] agents = [...]` scopes a server to an allowlist; an agent not
  on it is `excluded`, and one that still holds a render is `stranded`.
- A Commons entry with `"disabled": true` renders nowhere and its existing
  renders are removed by `sync`.
- `[mcp.<server>.tweaks.<agent>]` merges native keys into that agent's Render.
- `${env:VAR}` is resolved at render time; reports name changed keys, never
  values.
- `sync` never deletes a server because it left the Commons — that is
  indistinguishable from a hand-written one. Use `agent-sync mcp remove`.

oh-my-pi uses **NativeViaDiscovery**: it discovers other agents' MCP configs.
pi, Roo, OpenClaw and Hermes have no MCP surface.

## Hooks

Hooks are a **Key-merge by element identity**: a hook object whose command
matches a Commons hook is ours; every other element in the same event array
belongs to whoever put it there.

| Agent | File | Root key |
| --- | --- | --- |
| Claude | `~/.claude/settings.json` | `hooks` |
| Codex | `~/.codex/hooks.json` | `hooks` |
| Gemini | `~/.gemini/settings.json` | `hooks` |

- A matching hook is updated in place; foreign hooks in the same event survive.
- Event names map per agent (Gemini's `PreCompact` is `PreCompress`). An event
  the agent has no name for is `unsupported` and nothing is written.
- Agent-sync syncs the declaration, not the script the hook runs.
- Codex's `trusted_hash` is never written; approving code execution stays the
  user's decision.
- `${env:VAR}` is deliberately not expanded: the command string is the
  identity, and a resolved value would stop matching the reference that
  produced it.

## Agents at a glance

`—` means the agent has no surface for that family.

| Agent | skills | instructions (default) | commands | agents | mcp | hooks |
| --- | --- | --- | --- | --- | --- | --- |
| Claude | fan-out | import | fan-out | fan-out | key-merge | key-merge |
| Codex | native (prune `.codex/skills`) | symlink | fan-out | — | key-merge | key-merge |
| opencode | native | import | fan-out | fan-out | key-merge | — |
| pi | fan-out | symlink | — | — | — | — |
| oh-my-pi | native | symlink | discovery | — | discovery | — |
| OpenClaw | fan-out | — | — | — | — | — |
| Hermes | fan-out | — | — | — | — | — |
| Gemini CLI | native | import | render (TOML) | — | key-merge | key-merge |
| Cursor | native (prune `.cursor/skills`) | — | fan-out | — | key-merge | — |
| Windsurf | — | symlink | fan-out | — | key-merge | — |
| Roo | — | symlink (rules dir) | fan-out | — | — | — |
| Cline | — | — | — | — | key-merge | — |
| WorkBuddy | fan-out | — | — | — | key-merge | — |

## Configuring

The mechanism is a property of the agent, not a user setting. What the user can
configure in `$XDG_CONFIG_HOME/agent-sync/agent-sync.toml` is:

- `[targets] <agent> = false` — disable a Target entirely.
- `[instructions]` — pick the instructions route per agent.
- `[custom.<name>]` — define an agent with `root` and optional fan-out
  directories for `skills`, `commands` and `agents`.
- `[mcp.<server>]` — `agents` allowlist and `tweaks.<agent>` native keys.

On the Commons side: `skills/`, `commands/`, `agents/`, `AGENTS.md`, `mcp.json`
and `hooks/<Event>.toml`.

## Safety rules

- Every family is surveyed read-only first. Any problem in any family means
  `sync` writes nothing at all (ADR-0007).
- Write failures are reported and the remaining work continues; re-runs are
  idempotent.
- Ownership is decided from the filesystem: a link into the Commons, a Marker,
  a server name, a hook command. A Foreign entry is never touched, and a Variant
  is never clobbered.
- Refusals precede the lock and the writes.

See [status-states.md](status-states.md) for what `status` reports for each
mechanism.
