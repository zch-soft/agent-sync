---
description: Append a dated entry to DECISIONS.md
argument-hint: <the call that was made>
allowed-tools: Read, Edit, Write, Grep
---

Record this decision in `DECISIONS.md`: `$ARGUMENTS`.

`DECISIONS.md` is append-only. Add a new section at the end — never rewrite or reorder
existing entries:

```markdown
## YYYY-MM-DD — <the call, as a sentence>

What was decided, the evidence that supports it (a measured result, a URL, a spec or
ADR reference), and what was rejected and why. Keep it prose, not bullets.

Decided-by: human (<what>) ; agent (<what>)
```

Use today's date. If the call contradicts an existing ADR, say so explicitly and
propose a new ADR rather than editing the old one. If the call is still open, do not
record it as settled — ask what remains undecided.
