# .claude — agent-sync framework

Project-local Claude Code configuration. Everything here is checked in and shared;
machine-local overrides belong in `settings.local.json`, which is not committed.

| Path | Purpose |
| --- | --- |
| `../CLAUDE.md` | Project instructions loaded every session |
| `settings.json` | Permissions and project MCP enablement |
| `../.mcp.json` | Project MCP servers (`codegraph`) |
| `agents/` | Subagents: implementer, test author, domain reviewer, triager, release engineer |
| `skills/` | Reusable workflows: core dev, add a Target, issues, vocabulary, release |
| `commands/` | `/gate`, `/issue`, `/new-target`, `/vocab`, `/decision` |
| `memory/` | Durable working context indexed by `memory/MEMORY.md` |

## Using it

- Everyday code change: the `agent-sync-dev` skill loads automatically on relevant
  prompts; run `/gate` before committing.
- New agent support: `/new-target <agent>`.
- Ticket work: `/issue <path-or-number>`.
- Wording change: `/vocab`.
- A design call the spec did not settle: `/decision <call>`.

## Maintenance

- Update `memory/open-threads.md` when a `.scratch` effort opens or closes.
- Update `memory/release.md` on each release.
- When an agent target's behavior changes, update `skills/add-agent-target` and
  `memory/environment.md` if the local procedure changed.
- Do not duplicate `CONTEXT.md`, `DECISIONS.md`, or the ADRs here. Link to them.
