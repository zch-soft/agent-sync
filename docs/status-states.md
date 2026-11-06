# status states

`agent-sync status` surveys five surfaces. Each has its own state vocabulary, and the
same words appear in the human report and in `status --json`, where every entry carries
`state` and `actionable`.

The link families print every entry that is not `linked` and summarize the rest as
`N linked`. The instructions, MCP, hooks and rendered-command surfaces print every
item. `sync` acts only on the states marked **actionable** below, with the one
`variant-identical` exception explained in place.

## Link families: skills, commands, agents

| State | Meaning | `sync` |
| --- | --- | --- |
| `linked` | Our link is present and canonical: it resolves to the right Commons entry and uses the canonical link text. | Nothing. |
| `missing` | The Commons has the entry; this Target directory does not. | Creates the link. |
| `stale` | Our link exists but is not canonical — absolute text, or onto the wrong entry. | Removes and recreates it. |
| `dangling` | Our link points at a Commons entry that no longer exists. | Removes the link. |
| `duplicate` | Our link sits in a Native agent's old fan-out directory. The agent already reads the Commons, so the link is a live duplicate. | Removes the link. |
| `variant-identical` | A real file or directory shadows the Commons entry and is byte-identical to it. | Nothing automatic. It is still counted as actionable so you can dedupe on purpose: remove the real object, then `sync` links it. |
| `variant` | A real file or directory shadows the Commons entry and differs from it. The difference is deliberate. | Nothing. A Variant is never clobbered. |
| `foreign` | A symlink resolving outside the Commons. | Nothing. It is somebody else's. |

For a `foreign` link the human report also prints the absolute path the link
resolves to, since the name alone says nothing about the destination.

## instructions (AGENTS.md)

| State | Meaning | `sync` |
| --- | --- | --- |
| `linked` | Our symlink is in place and canonical. | Nothing. |
| `missing` | Nothing is at the destination; the symlink route will create the link. | Creates the link. |
| `imported` | The user's file already carries the Import-line or Include-entry naming the Commons. | Nothing. |
| `import` | The user's file exists, or does not, and needs the Import-line added. | Adds the line. |
| `conflict` | A file agent-sync did not write occupies the destination. The occupant is named when recognisable. | Nothing; this is a decision about another tool's content. |
| `foreign` | A symlink pointing somewhere other than the Commons. | Nothing. It is somebody else's. |
| `legacy` | Our symlink or Include-entry left by a route this Target no longer takes. | Removes it. |
| `replace` | The other route's artifact of ours at the same path — for example Claude's `CLAUDE.md` as our link, or a file holding only the Import-line. | Swaps it for the chosen route's form. |

## mcp (mcp.json)

| State | Meaning | `sync` |
| --- | --- | --- |
| `managed` | The server is in the Commons and the agent's copy is exactly the Render. | Nothing. |
| `missing` | In the Commons, absent from this agent's config. | Writes it. |
| `drifted` | In both, but the agent's copy differs. The Commons wins. The note names the changed keys, never the values. | Overwrites it. |
| `foreign` | Only in the agent's config, not in the Commons. | Nothing. It is somebody else's. |
| `excluded` | In the Commons, but this agent is not on the server's allowlist and has no copy either. | Nothing to do. |
| `stranded` | In the Commons and in this agent's config, but no longer allowlisted for it. Reported so a narrowed allowlist leaves no leftovers. | Nothing; clear it with `agent-sync mcp remove <name>`. |
| `disabled` | In the Commons with `"disabled": true`, yet still in the agent's config. A Disabled server renders nowhere, including renders already made. | Removes this render by name. |

## hooks

| State | Meaning | `sync` |
| --- | --- | --- |
| `managed` | Present and exactly the Render. | Nothing. |
| `missing` | In the Commons, absent from this agent. | Writes it. |
| `drifted` | Present but different. The Commons wins. | Overwrites it. |
| `unsupported` | This agent has no name for the event, so nothing is written. | Nothing. |
| `foreign` | In the agent's config, not in the Commons. | Nothing. It is somebody else's. |

## rendered commands

| State | Meaning | `sync` |
| --- | --- | --- |
| `managed` | Carries the Marker and matches the Render. | Nothing. |
| `missing` | The Commons has the entry and the file is absent. | Generates it. |
| `drifted` | Ours — the Marker is present — but its contents changed. | Overwrites it. |
| `orphaned` | Ours — the Marker is present — but the Commons entry it came from is gone. | Prunes it. This is the only family that prunes without being asked by name. |
| `foreign` | No Marker, so somebody else wrote it. | Nothing. |

## actionable and exit codes

A state is actionable when `sync` would change it. Two consequences matter:

- `variant-identical` is actionable even though `sync` leaves it alone: removing the
  real object is a safe dedupe you must perform yourself.
- `variant`, `foreign`, `conflict`, `unsupported`, `excluded` and `stranded` are
  reported but not actionable. They are either somebody else's or a deliberate state.

Exit codes are the contract automation depends on: `0` clean, `2` actionable, `1` the
command could not answer — a missing Commons or a fault while surveying it. `sync
--dry-run` previews the same plan but exits `0` even with pending work, so `status`,
not dry-run, is the CI drift gate.
