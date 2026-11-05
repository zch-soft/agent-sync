# Conventions

## Commits

- One logical change per commit. History reads as sentences, not labels.
- Prefixes actually in use: `feat:`, `fix:`, `refactor:`, `test:`/`tests:`, `docs:`,
  `ci:`, `style:`, `scripts:`, `issues:`. A release commit is `Release X.Y.Z`.
- Subject states the outcome. Body explains **why** when the diff cannot, with
  evidence (a doc URL, a measured result, an upstream issue) where behavior rests
  on a claim about another agent.
- Commit on the current branch. Do not create a branch, push, tag or publish unless
  asked.

## Comments and docs

- Comments carry the reason, the measured evidence, or the hazard. They are denser
  than average on purpose: `src/registry.rs` and `tests/vocabulary.rs` encode
  decisions that would otherwise be re-litigated.
- User-facing strings are vocabulary surface. Review every new message against
  `CONTEXT.md`; `tests/vocabulary.rs` only catches the mechanically checkable subset.
- ADR: four-digit number, one decision, status/context/decision/consequences shape.
  ADRs are not rewritten; a later decision supersedes by adding a new one.
- `DECISIONS.md` is append-only. Each entry is `## YYYY-MM-DD — <call>`, prose
  stating the call, the evidence, and a closing `Decided-by:` line naming who chose
  what.

## Issues

- `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`.
- A `**Status:**` line holds the triage label. The canonical labels are
  `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`;
  `docs/agents/triage-labels.md` maps roles to them.
- Completed work keeps its file and flips to `done`; do not delete the ticket.
- Conversation appends under `## Comments`; wayfinder answers under `## Answer`
  before the status flips to `resolved`.

## Change shape

- Methods stay near their data; modules are named for the domain, not for the edit.
- New behavior earns a test at the seam that proves it, not a test per function.
  The suite deliberately pruned low-value tests; do not regrow it with mirrors of
  the implementation.
- Cross-platform behavior is part of the feature, not a follow-up. Windows paths,
  junctions and symlink privilege are first-class.
