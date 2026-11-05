---
name: issue-workflow
description: Work the local markdown issue tracker in .scratch/ — create, triage, claim, block, resolve, or run wayfinder map/frontier operations. Use when asked what to work on next, to file or triage an issue, or to close out a ticket.
---

# Issue workflow

The tracker is files in `.scratch/`; there is no external service.

## Layout

```
.scratch/<effort>/
├── spec.md                     # what the effort is
├── map.md                      # wayfinder only
└── issues/
    ├── 01-<slug>.md
    └── 02-<slug>.md
```

Issue header:

```markdown
# 01 — Imperative title

**What to build:** one paragraph, in domain vocabulary, naming the evidence.

**Blocked by:** None — can start immediately.   (or: 02, 03)

**Status:** needs-triage

- [ ] acceptance criterion
- [ ] acceptance criterion
```

## Labels

`needs-triage` · `needs-info` · `ready-for-agent` · `ready-for-human` · `wontfix`.
A closed ticket uses `done` (implementation) or `resolved` (wayfinder question).
`docs/agents/triage-labels.md` is the mapping table.

## Triage

Read the code the report names before choosing a label. Then:

- **needs-info** — the report cannot be acted on without an answer; say which answer.
- **ready-for-agent** — fully specified; an agent can implement it without design
  calls.
- **ready-for-human** — needs a physical measurement, a device, credentials, or a
  judgment the maintainer reserved.
- **wontfix** — record why in `## Comments`; do not delete the file.

## Wayfinder

- **Map**: `.scratch/<effort>/map.md`, with Notes / Decisions-so-far / Fog.
- **Frontier**: open, unblocked, unclaimed tickets; lowest number first.
- **Claim**: set `Status: claimed` and save before doing any work.
- **Resolve**: append `## Answer`, set `Status: resolved`, then append a gist-plus-link
  pointer to the map's Decisions-so-far.

## Rules

- Never delete a ticket; close it and keep the history.
- Never mark `done` without the acceptance criteria actually met and the gate green.
- Do not rewrite someone else's `## Comments`; append.
- When a ticket is implemented, keep its file and flip the status — the tracker is the
  record of what was decided, not a to-do list to clear.
