# Backlog of vectors

Ranked list of improvement / bugfix ideas the daily research routine has
surfaced. Higher-score items go to the top. The manual implementation agent
picks from here.

| Score | Title | Vector | Source | Added | Status |
|-------|-------|--------|--------|-------|--------|
|       |       |        |        |       |        |

## Scoring

- **+3** confirmed bug with reproduction
- **+2** repeated churn in same file/area over multiple PRs
- **+2** TODO/FIXME in recently-merged code, with author context still warm
- **+1** stalled PR review (signal of unclear direction)
- **+1** test gap exposed by a fix
- **−1** speculative / cosmetic refactor with no concrete pain
- **−2** already has an open ticket / in-flight PR

Vectors: `bug`, `refactor`, `perf`, `test-gap`, `dx`, `security`, `dep-update`.

## Status

- `new` — fresh, untouched
- `picked` — manual agent took it (moved into `plans/<slug>.md`)
- `done` — landed; archived plan in `plans/_archive/<slug>.md`
- `dropped` — reviewed and rejected, see plan body for why
