---
name: add-agent-target
description: Add or correct support for an AI coding agent as a Target in agent-sync — a new registry row, a capability flip, a new instructions route, or a new MCP dialect. Use when asked to support an agent such as Cursor, WorkBuddy, Cline, or when a registry claim needs verification.
---

# Adding an agent Target

Adding an agent is a **data change**: one `Agent` row in `src/registry.rs`, plus tests
and docs. It must not add control flow.

## The bar for a registry row

A row is a claim about the agent's real behavior, not about its documentation's
aspirations.

- **Detection is the config root's existence.** agent-sync may create subdirectories
  under an existing root but never the root itself. Do not make `init` or `doctor`
  create it.
- **A capability is claimed only when it holds unconditionally.** If the agent reads
  `~/.agents/skills` only after the user edits its config, that agent is not
  `Skills::Native` (ADR-0004). Prefer the mechanism that always works.
- **Verify against the agent itself or its source**, on a real install. Record the
  evidence — version, date, URL or command output — in the row's comment and in a
  `DECISIONS.md` entry. A row sourced from a neighbour project's adapter is an
  unverified guess.
- **Instructions routes need a measured parser.** An import line is offered only where
  the agent is proven to consume it; an include entry only where the config key is
  proven; a symlink only where nothing else rewrites that file through the link
  (ADR-0008). Offering two routes is allowed when both are measured (ADR-0009).

## Steps

1. **Observe the install.** Which config directories exist? Which files does the
   agent write on its own? Which file does it read instructions from? Reproduce the
   read with a sentinel string before claiming the path.
2. **Add the row** in `src/registry.rs` with every field decided:
   `name`, `root`, `skills`, `instructions`, `mcp`, `commands`, `agents`, `hooks`.
   Use `*::None` where the surface does not exist — never a guessed path.
3. **Add the MCP dialect** in `src/mcp.rs` only if the format is not
   `McpDialect::Standard`. A dialect is about key shape and naming, not file location.
4. **Test the row**:
   - `tests/sync_skills.rs` — native agents get no links; fan-out agents get the
     expected links; legacy duplicates are pruned.
   - `tests/instructions.rs` — every offered route converges, and the unchosen route
     is removed.
   - `tests/mcp_dialects.rs` — exact rendering into that agent's format.
   - `tests/doctor.rs` — detection and capability reporting.
5. **Update docs**: the agent list in `README.md`, and the registry comment with the
   evidence.
6. **Run the gate** (see the `agent-sync-dev` skill).
7. If the capability is new rather than a corrected path, add a `DECISIONS.md` entry
   with the evidence.

## Common mistakes

- Treating a documented path as a measured one.
- Fanning out skills to an agent that also reads the Commons natively, creating
  duplicates.
- Creating the agent's root during detection.
- Writing into a file another tool owns without checking who rewrites it.
- Assuming Unix path behavior; Windows junctions and link privilege are part of the
  feature.
