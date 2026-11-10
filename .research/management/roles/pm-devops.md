# pm-devops — 2026-11-10

_Rotation slot: 4 → 5 (74-day stale going in; last run 2026-08-31). pm-scrum-master always-on._

## Summary

CI pipeline is healthy — 0 failures across 15 named workflows in the recent window. The long-standing cloud-runner starvation for mobile-native/KMP persists even though tracking issues #2652 (AGP egress) and #2966 (utoipa build-script egress) are both closed — the action-list is stale, but the operational gap is real.

## Next actions

| Priority | Action | DoD |
|----------|--------|-----|
| medium | Update `pm-devops-unblock-mobile-native-cloud-builds` action text: drop #2652 reference (issue closed), re-anchor to the structural claim-starvation evidence directly, open a fresh issue if the gap deserves one | action text no longer cites a closed issue; new issue opened only if a concrete pre-flight failure is reproducible |
| low | Audit workflow matrix for the 6 new draft PRs (#3372-#3377) to confirm Band-C paths are not accidentally triggered on code-review relands | workflows that ran on each draft are only fmt/clippy/typecheck + impact-scoped gates, not full matrix |
| low | Verify `deploy-server.yml` remains dormant — last run 2026-09-27 (~44 days ago); confirm no stale artifact or expired credential would silently break on next invocation | a dry-run or `workflow_dispatch` invocation succeeds |

## Risks

| Prob | Impact | Risk | Mitigation |
|------|--------|------|------------|
| medium | high | `deploy-server.yml` has not run in ~44 days; token/cred rotations may have silently broken it | dry-run or scheduled smoke-check; keep rollback branch warm |
| low | low | Six concurrent draft PRs may saturate GitHub Actions concurrency buckets on `auto-impl/*` branches | most are small code-review relands; concurrency groups already set |
| high | medium | Mobile-native KMP structural gap continues to accumulate backlog items that cannot clear in cloud | either land the cloud-runner work (needs a maintainer with a local runner) or route KMP work to a local-only path explicitly |

## Open questions

- Is there ownership for the mobile-native cloud-runner gap, or is it accepted that KMP work must run local-only indefinitely?
