# Configuration and environment

agent-sync reads one configuration file and a handful of environment variables
that decide where it looks. There is nothing else to configure: the Commons holds
the content, and the filesystem is the state (ADR-0003), so no manifest records
what was synced.

## Where things live

| What | Default | Moved by |
| --- | --- | --- |
| Tool config | `~/.config/agent-sync/agent-sync.toml` | `$XDG_CONFIG_HOME/agent-sync/agent-sync.toml` |
| Commons | `~/.agents/` | `AGENT_SYNC_HOME` |
| Lock (the only machine state) | `~/.local/state/agent-sync/lock` | `$XDG_STATE_HOME/agent-sync/lock` |
| Home that Targets resolve against | `$HOME`, else `%USERPROFILE%` | `AGENT_SYNC_TARGET_ROOT` |

The XDG defaults are the same on every platform: on Windows the config file is
`%USERPROFILE%\.config\agent-sync\agent-sync.toml` unless `XDG_CONFIG_HOME` says
otherwise.

The tool config deliberately does **not** live in the Commons. The Commons is an
interop contract other agents read and other tools keep files in (ADR-0004), so
nothing agent-sync-specific squats there.

## `agent-sync.toml`

Optional. An absent file means pure defaults. A file that exists but does not
parse is an error, reported with its position and nothing written — and the
parse error is never formatted with the offending source line, because a Tweak
can hold an adopted secret. Edits made by agent-sync itself are format
preserving: comments, key order and spacing survive.

Four top-level tables, all optional.

### `[targets]`

```toml
[targets]
cursor = false     # switch a Target off
```

`false` marks a **Disabled** Target: it leaves the resolved Target list entirely,
so `sync`, `status`, `doctor` and `mcp` stop visiting it. What it already has on
disk is left alone — disabling is not offboarding. To remove what a Target
already received, run `agent-sync revert <name>`, which reaches Disabled Targets
on purpose and refuses while the Target is still enabled.

`true` is the default and changes nothing. A name that matches no built-in agent
and no `[custom.<name>]` entry has no effect.

### `[instructions]`

```toml
[instructions]
claude = "symlink"   # "symlink" or "import"
```

Picks another **Instructions route** for one agent. Omitted, the agent keeps its
registry default (first route listed). Only routes the agent actually has may be
named: Claude, opencode and Gemini CLI offer both; every other agent takes the
one route it has, and naming a route it lacks is a configuration error
(ADR-0009). A `[custom.<name>]` Target takes fan-out families only and cannot
appear here.

### `[custom.<name>]`

```toml
[custom.myagent]
root = ".myagent"                     # required; detection looks for this directory
skills = ".myagent/skills"            # optional, per fan-out family
commands = ".myagent/commands"
agents = ".myagent/agents"
```

Registers an agent the built-in registry does not know, with a home-relative
**fan-out** directory per family. `root` is required; each family key is
optional. The family names are `skills`, `commands` and `agents`; the pre-v2
name `subagents` is still accepted for `agents`, and `agents` wins when both
appear.

A config-defined Target has no registry row, so it takes fan-out families only —
no instructions route, no MCP rendering, no hooks. `[targets].<name> = false`
switches it off exactly like a built-in.

### `[mcp.<server>]`

```toml
[mcp.github]
agents = ["claude", "codex"]     # omitted: every MCP-capable agent

[mcp.github.tweaks.codex]        # a Tweak: native keys for one agent's rendering
startup_timeout_sec = 60
```

`agents` scopes one **Managed** server to a list of agents. Omitted, every agent
that can take MCP receives it, which is the default.

`tweaks.<agent>` merges native keys into that agent's **Render** of the server. A
Tweak lives here, never in the Commons, because the Commons file holds the
ecosystem-standard shape only.

Only `agents` and `tweaks` are accepted under a server — a typo is an error
rather than a setting silently ignored for the life of the file.

`agent-sync mcp adopt` writes this table (the allowlist it recorded from where
the server actually came from, plus any absorbed Tweaks), and `agent-sync mcp
remove` deletes the rule along with the server. `mcp disable` and `mcp enable`
do **not** touch it: a **Disabled** server is marked `"disabled": true` in the
Commons itself, so its definition and rules survive.

## Environment variables

| Variable | Effect |
| --- | --- |
| `AGENT_SYNC_HOME` | Relocates the Commons. Non-empty value only; no other validation. |
| `AGENT_SYNC_TARGET_ROOT` | Replaces `HOME` as the root every home-relative path resolves against — including the default Commons, which is derived from it. |
| `HOME` | The default home. |
| `USERPROFILE` | Fallback home when `HOME` is unset — Windows shells set this instead. |
| `XDG_CONFIG_HOME` | Base for the tool config directory; the tail `/agent-sync` is appended. A relative or empty value is ignored, per the XDG spec. |
| `XDG_STATE_HOME` | Base for the state directory (the lock), same rules. |
| `AGENT_SYNC_LOCK_TIMEOUT_MS` | How long a mutating command waits for a competing run. Default `30000`; an unparseable value falls back to the default rather than erroring. |

`AGENT_SYNC_HOME` moves agent-sync's Commons but cannot move a path a **Native**
agent hardcodes. Those agents keep reading `~/.agents/` and silently diverge, so
`doctor` names each one when the relocated Commons differs from the canonical
path.

### `${env:VAR}` in MCP servers

A Commons MCP server definition, and any Tweak for it, may embed `${env:VAR}` in
any string, at any depth; the value is substituted when the server is rendered
into an agent's config. This is how a secret stays out of both the Commons and
the rendered config.

- An unset or empty variable is a problem: `sync` names it and writes nothing at
  all, so no partially resolved entry can reach a file (ADR-0007).
- An unterminated `${env:` is literal text, not an error.
- Resolved values never reach a report. Notes name the keys that changed, never
  the values, so redaction is structural rather than a discipline at each call
  site.

Hook commands are the deliberate exception: `${env:VAR}` is **not** expanded
there. The command string is the hook's element identity, so a value that varied
with the ambient environment would stop matching the declaration that produced
it. A hook that needs a secret reads it from the environment itself.

## Checking the current state

`agent-sync doctor` prints the resolved config path and the Commons, warns when a
relocated Commons is invisible to Native agents, and names leftover config
directories from before the rename (`~/.agentstow/` and `~/.config/agentstow/`)
without reading them.
