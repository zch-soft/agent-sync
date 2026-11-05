# Open threads

Snapshot, not authority. Confirm against `.scratch/` and `git log` before acting;
update this page when a thread opens or closes.

## Shipped

- **agentstow v1/v2** — implemented, tagged, and rolled out across the author's
  fleet. The v2 spec remains the design record for the Commons rename, XDG private
  files, two-phase sync, the three adopt mechanics, `revert`, and
  `mcp enable|disable`.
- **instructions-route** — each agent's instructions route became user-selectable
  (ADR-0009, commit `978ebed`); the registry now lists routes per agent.
- **WorkBuddy** — added as a Target (`97a9d53`), skills plus standard `mcpServers`.

## Open

Both open tickets are `ready-for-human` and live in
`.scratch/instructions-route/issues/`:

- **01 — Ubuntu 26 runner check.** GitHub moved `ubuntu-latest` to Ubuntu 26 on
  2026-10-19. Read the first `ci` run on `main` after that date; pin failing jobs to
  `ubuntu-24.04` with a one-line reason. Decide before the next release whether the
  `release.yml` jobs need the same pin, since a failure there can leave a
  half-published release.
- **02 — opencode with both instructions routes.** It is unmeasured whether opencode
  loads the Commons once or twice when both the `instructions` entry and the
  `AGENTS.md` link exist. Measure with tools off, record the result in the Commons'
  `agent-clis.md`, and add a `doctor` warning only if the file loads twice.

## Standing cautions

- `release.yml` publishes to crates.io, npm and PyPI; none allows re-uploading a
  version. A tag is irreversible in practice — verify before creating one.
- The project was renamed in-tree from agentstow; retired names still appear in old
  ADRs and the yanked 1.x line. Do not "fix" historical wording.
