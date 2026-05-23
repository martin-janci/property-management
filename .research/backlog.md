# Backlog of vectors

<sub>Last regenerated: 2026-05-23 16:04 UTC by routine</sub>

> **Canonical source:** `backlog.json`. This file is **regenerated** from it
> each run — do not edit by hand. To drop, defer, or re-score a vector, edit
> `backlog.json` and let the next run rebuild this view (or commit both
> together).

Ranked list of improvement / bugfix ideas the daily research routine has
surfaced. Higher-score items go to the top. The manual implementation agent
picks from here.

| Score | Title | Vector | Source | Updated | Status |
|-------|-------|--------|--------|---------|--------|
| 3 | IDOR: unlink_voice_device deactivates any device by ID with no owner/org scoping | security | code-review api-core 2026-05-23, ai.rs:3002 | 2026-05-23 | ready |
| 2 | Complete RLS migration in 31 remaining handlers (voting, market_pricing, faults, notif_prefs, reports) | security | issue #160, PR #420, PR #421 | 2026-05-23 | done |
| 2 | Integration marketplace install/OAuth flows are placeholders — wire backend handlers + UI navigation | dx | PR #282, PR #328, commit c97781a, commit 254f01d | 2026-05-23 | open |
| 2 | integrations.rs churn-hot — 12,977 lines over 14d, candidate for module split | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20 | 2026-05-23 | open |
| 2 | organizations.rs churn-hot — 12,060 lines over 14d (multitenancy + admin) | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20 | 2026-05-23 | open |
| 2 | documents.rs churn-hot — 10,659 lines over 14d | refactor | git log origin/main since 2026-05-06, git log origin/dev since 2026-05-20 | 2026-05-23 | open |
| 2 | Dead/duplicate handler modules: AuthHandler & BuildingHandler unused, routes reimplement inline | refactor | code-review api-handlers 2026-05-23 | 2026-05-23 | open |
| 2 | Portfolio dashboard: alert mark-read/resolve mutations + property-card click navigation are no-op stubs | dx | PR #328, commit 254f01d | 2026-05-20 | open |

## Scoring rubric

Mirrors `routine-prompt.md` § *Phase 1 — Observe* signal table. **Canonical source** is the routine prompt; this is the human-readable summary.

| Signal type | Δscore | Notes |
|---|---|---|
| `unchecked-todo` (PR body has `- [ ]` after merge) | **+2** | warm context |
| `revert` (PR is a revert) | **+3** | dig into original |
| `stalled-review` (open PR >7 days, no reviewDecision) | **+1** | process signal |
| `churn-hotspot` (top-3 raw churn this run) | **+1** | filter exclusions first |
| `repeated-churn` (hotspot file in `hotspot_history.runs_seen >= 2`) | **+1 (stacks)** | instability proxy |
| `risky-churn` (churn alongside revert/bugfix-no-test) | **+2** | combine with churn |
| `fixme-in-merged-code` (new TODO/FIXME in merged diff) | **+2** | report file:line |
| `hotfix-no-test` (merged "fix"/"hotfix" PR with no test diff) | **+2** | classic test-gap |
| `untriaged-issue` (new issue, no label) | **+1** | vector=`triage`; never promoted |
| `closed-not-merged-pr` (PR closed unmerged) | **+1** | look at close reason |
| `dep-update-noise` (dependabot/renovate PR) | **0** | log in brief, don't score |

**Score cap:** 8. **Decay:** −1 per 14 days without new evidence on an `open` item.

Vectors: `bug`, `refactor`, `perf`, `test-gap`, `dx`, `security`, `dep-update`, `triage`.
`triage` is special: untriaged issue surface signal. **Never promoted to a plan** — Phase 3 readiness gate excludes it. Lives in the backlog as a human-only review queue.

## Status

- `open` — needs review or more evidence; no plan exists yet
- `ready` — routine has authored a plan at `plans/<slug>.md` and the item now points at it via the `plan` field; awaiting an implementer
- `done` — implementer shipped the PR; plan moved to `plans/_archive/<slug>.md`
- `dropped` — reviewed and rejected; reason is in the item's `evidence` array in `backlog.json` (one line starting with `dropped:`), or decayed to score 0 with `decayed: no new signals in 14 days`
- `needs-human-judgement` — blocked on a question only the user can answer; routine won't promote, decay, or drop without user input
