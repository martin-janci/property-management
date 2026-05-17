# Backlog of vectors

> **Canonical source:** `backlog.json`. This file is **regenerated** from it
> each run — do not edit by hand. To drop, defer, or re-score a vector, edit
> `backlog.json` and let the next run rebuild this view (or commit both
> together).

Ranked list of improvement / bugfix ideas the daily research routine has
surfaced. Higher-score items go to the top. The manual implementation agent
picks from here.

| Score | Title | Vector | Source | Updated | Status |
|-------|-------|--------|--------|---------|--------|
|       |       |        |        |         |        |

## Scoring rubric

- **+3** confirmed bug with reproduction
- **+2** repeated churn in same file/area over multiple runs
- **+2** unchecked TODO/FIXME in recently-merged code with author context still warm
- **+1** stalled PR review (signal of unclear direction)
- **+1** test gap exposed by a fix
- **−1** speculative / cosmetic refactor with no concrete pain
- **−2** already has an open ticket / in-flight PR

**Score cap:** 8. **Decay:** −1 per 14 days without new evidence on an `open` item.

Vectors: `bug`, `refactor`, `perf`, `test-gap`, `dx`, `security`, `dep-update`, `triage`.
`triage` is special: untriaged issue surface signal. **Never promoted to a plan** — Phase 3 readiness gate excludes it. Lives in the backlog as a human-only review queue.

## Status

- `open` — needs review or more evidence; no plan exists yet
- `ready` — routine has authored a plan at `plans/<slug>.md` and the item now points at it via the `plan` field; awaiting an implementer
- `done` — implementer shipped the PR; plan moved to `plans/_archive/<slug>.md`
- `dropped` — reviewed and rejected; reason is in the item's `evidence` array in `backlog.json` (one line starting with `dropped:`), or decayed to score 0 with `decayed: no new signals in 14 days`
- `needs-human-judgement` — blocked on a question only the user can answer; routine won't promote, decay, or drop without user input
