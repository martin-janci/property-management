# pm-devops — DevOps / Infrastructure

_Ran: 2026-08-31 (PM rotation, idx 4 → 5; previous run 2026-06-16, 76 days stale)_

## Summary

CI is green on `dev` for the 10-PR code-review batch that landed 2026-08-30 (#2889-#2899). Dispatcher observations flag one structural infrastructure gap that repeats every run: **mobile-native/KMP jobs are unlandable in the cloud runner** (7 of 8 currently-open backlog items are mobile-native/KMP; issue #2652 - AGP/Gradle needs egress that the sandbox blocks). This has been the buffer-starvation cause for 5+ consecutive dispatcher runs (`GC3-buffer-bounds=FAIL (record-only)` on every commit since 2026-08-30). Otherwise dev infra is healthy: no red workflows on `main`, `research-land.yml` continues to replay routine commits from session branches to `dev` cleanly, and the pmctl worktree fleet is drained.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Unblock mobile-native/KMP builds in the cloud runner (issue #2652). Either allowlist AGP/Gradle egress in the runner sandbox, add a Docker-cached Gradle build layer, or route mobile-native tasks to a dedicated local runner label — the current setup keeps 7/8 backlog items structurally unclaimable in the cloud loop and forces Tier-1d dev-review generator kicks every run just to fill the buffer | pm-tech-lead | dispatcher `claimable` counts mobile-native items > 0 on at least one cloud run without operator intervention |
| medium | Add a nightly `SKIP_NETWORK=1 ./.claude/skills/verify-all.sh --quick` smoke on `dev` HEAD so silent infra-only regressions surface without waiting for a PR — verify-all is already the auto-fix gate but has no scheduled sweep | pm-backend | new workflow file that runs on cron + fails visibly in Actions tab |
| medium | Confirm `screen-map.yml` CI gate is green on `main` post today's 10 PRs — none of the merged PRs touched `docs/screens/**` but `App.tsx` was modified (PR #2889); routine emits `screen-map-drift-pr-2889-ppt` this run | none | `gh run list --workflow=screen-map.yml --branch=main --limit=1` shows conclusion=success |
| low | Audit the dependabot open queue — 7 open dep-update PRs (#2673/2749/2865/2866/2867/2808 + npm-minor group #2867) sitting 3 days idle. Batch-review + auto-merge the safe minor bumps to reduce open-PR drag | pm-tech-lead | open dep PRs ≤ 3 after next sweep |

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Buffer starvation becomes chronic if mobile-native items keep accumulating and the cloud runner never picks them up. Every new mobile-native code-review finding adds an unclaimable item, pushing dispatcher back to Tier-1d generator kicks | high | medium | Fix #2652 or route mobile-native items to a `mobile-native` action-list bucket that is *not* counted toward the 36-slot cloud buffer |
| `research-land.yml` replay is the only path routine commits reach `dev` — a workflow break would silently accumulate `.research/` commits on session branches | low | high | Add an alert if `dev` HEAD hasn't advanced with a routine commit in >36h (mirror the stale-routine alert from the routine's Phase 1) |

## Decisions needed

- **Cloud runner scope decision (owner: pm-tech-lead):** commit to unblocking mobile-native/KMP in the cloud vs formally splitting the dispatcher into cloud + local queues. Current halfway state (all queues counted, only some claimable) causes the recurring `GC3-buffer-bounds=FAIL (record-only)` noise on every commit.
