---
description: Add support for a new AI coding agent Target
argument-hint: <agent-name> [evidence-url-or-version]
allowed-tools: Read, Edit, Write, Grep, Glob, Bash(cargo:*), mcp__codegraph__codegraph_explore
---

Add `$ARGUMENTS` as an agent Target. Follow the `add-agent-target` skill.

Start by observing the real install or the agent's source: config root, native
`~/.agents/skills` support, instructions mechanism and its measured parser, MCP file
and dialect, commands/agents/hooks directories. Do not add a registry row on
documentation alone.

Then add the `Agent` row in `src/registry.rs`, any dialect in `src/mcp.rs`, tests in
the relevant `tests/` files, the README agent list, and a `DECISIONS.md` entry
recording the evidence. Run the full gate.

Report what you verified, what you inferred, and what remains unverified. Do not
commit.
