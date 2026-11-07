# agent-sync — project instructions

One canonical `~/.agents/` **Commons**, fanned out to every installed AI coding agent.
Rust 2024, MSRV 1.97, no async runtime, and no state file.

`AGENTS.md` is the agent-facing index. `CONTEXT.md` is the glossary and is binding for
code identifiers, docs, CLI output and tests. This file is the working memory; the
`.claude/` directory holds the reusable framework.

## Three invariants

1. **The filesystem is the state.** Never add a state file, manifest, database or
   backup directory recording what was synced. Ownership is decided from what is on
   disk: a link into the Commons is ours, anything else is not. A git-versioned
   Commons is the history.
2. **Plan before writing.** `sync` surveys every family read-only, collects every
   problem, and writes nothing if any problem exists. `--dry-run` is that same phase
   one plus the printed plan. Write failures report and continue; re-runs are
   idempotent. New behavior must keep `sync --dry-run` a true preview.
3. **Own only what is ours.** A **Foreign** entry is never touched. A **Variant**
   (a real directory shadowing a Commons entry) is preserved, never clobbered. A
   **Co-tenant** (another tool's file in the Commons) is named by `doctor` and left
   alone. Refusals always precede the lock and the writes.

## Language is code

Use `CONTEXT.md` vocabulary — **Commons**, **Target**, **Fan-out**, **Native**,
**Co-tenant**, **Protocol surface**, **Variant**, **Adopt**, **Sourced**, **Revert**,
**Foreign**, **Conflict**, **Render**, **Managed**, **Disabled**, **Tweak**,
**Instructions route**, **Import-line**, **Include-entry**, **Rules-dir link**,
**Marker** — and never the words in each entry's _Avoid_ list. "Store" is retired.
The avoid-list is enforced by `tests/vocabulary.rs` against string literals in
`src/`; that test does not catch prose, so review docs and comments against
`CONTEXT.md` yourself.

## Where truth lives

| Question | Source |
| --- | --- |
| What does a word mean? | `CONTEXT.md` |
| How is it configured, and which env vars matter? | `docs/configuration.md` |
| How do I build and install it locally? | `docs/build-and-install.md` |
| Why is the design this way? | `docs/adr/0001`–`0009`, `DECISIONS.md` |
| What is the current version building? | highest `.scratch/agentstow-v*/spec.md` |
| What is left to do? | `.scratch/<effort>/issues/NN-*.md` |
| How do I sync only part of the Commons? | `docs/selective-sync.md` |
| How do I force-overwrite states sync normally leaves alone? | `docs/force-sync.md` |
| How does each family sync, including instructions routes? | `docs/sync-mechanisms.md` |
| What does a `status` state mean? | `docs/status-states.md` |
| How is a release cut and verified? | `docs/release-runbook.md` |

`DECISIONS.md` is append-only. Record calls the spec did not settle as a dated
entry that states the call, the evidence, and a closing `Decided-by:` line.

## Gate

Run before every commit; CI runs the same four checks.

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo +1.97 check --locked   # MSRV promise; dev-dependencies excluded by design
```

`bash scripts/verify-packaging.sh` exercises the release pipeline without publishing.
The Windows end-to-end script is `scripts/e2e-windows.ps1` and needs an elevated shell
or Developer Mode.

## Framework

- `.claude/agents/` — `rust-implementer`, `test-author`, `domain-reviewer`,
  `issue-triager`, `release-engineer`.
- `.claude/skills/` — `agent-sync-dev` (core loop), `add-agent-target`,
  `issue-workflow`, `vocabulary-guard`, `release-verify`.
- `.claude/commands/` — `/gate`, `/issue`, `/new-target`, `/vocab`, `/decision`.
- `.claude/memory/` — durable working context not derivable from the code.
- `.mcp.json` — project MCP servers (`codegraph`). Machine-local servers such as
  `ai-memory` stay in the user's global config.
- `.codegraph/` — index. Prefer `codegraph_explore` over grep/read loops.

## Working style

- Read `CONTEXT.md` and the ADRs that touch the area before proposing a design.
- Make the smallest change that satisfies the request: no speculative abstraction,
  no compatibility shims for code paths nothing has ever written.
- Comments explain **why** a non-obvious constraint exists; they do not narrate the
  code. Existing files are comment-dense on purpose — match that only where the
  reasoning is genuinely hidden.
- Tests drive the single seam `agent_sync::run(args, vars, out, err)` against a
  throwaway tree. Never touch the real home directory in a test.
- Keep behavior cross-platform. Windows symlink creation needs Developer Mode or
  elevation; a fixture that fails with OS error 1314 is an environment problem, not
  a product bug.
- Commit on the current branch unless asked otherwise. Do not push, tag, publish, or
  open a PR without an explicit request.
