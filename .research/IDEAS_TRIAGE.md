# Triage queue

<sub>Last regenerated: 2026-05-27 03:14 UTC by routine</sub>

> **Canonical source:** `backlog.json` rows where `vector == "triage"`. This file is **regenerated** from it each run — do not edit by hand. To drop, defer, or re-score a triage row, edit `backlog.json` and let the next routine run rebuild this view.

Untriaged-issue signals (`vector: "triage"`) pile up here for human review rather than drowning the implementer's `backlog.md`. Phase 3's readiness gate explicitly excludes `triage`, so nothing in this file is promoted to a plan automatically.

| Score | Title | Source | Updated | Status |
|-------|-------|--------|---------|--------|
|       |       |        |         |        |

## Status legend

- `open` — needs review or more evidence; no plan exists yet (most triage rows live here)
- `done` — addressed by hand (issue closed, archived, or moved to a real vector)
- `dropped` — reviewed and rejected; reason is in the item's `evidence` array in `backlog.json`
- `needs-human-judgement` — blocked on a question only the user can answer
