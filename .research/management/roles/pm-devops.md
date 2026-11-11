# pm-devops — DevOps / Infrastructure

_Ran: 2026-11-11 (PM rotation, idx 4 → 5; previous run 2026-08-31, 72 days stale)_

## Summary

CI is **green on `dev`** for the 37-PR code-review burst that landed 2026-11-10 (#3371–#3409). `backend.yml`, `frontend.yml` and `mobile-native.yml` all finished `success` on HEAD after the last merge; one `cancelled` run is just a concurrency-cancel from a follow-up push, not a failure. The chronic `utoipa-swagger-ui` egress block that halted api-server builds in September **has not re-triggered** — every backend security PR this week compiled and shipped (argon2 floor #3394, cors policy share #3400, legal.rs error sanitization #3383, sso callback sanitization #3384, big SQL-injection audit #3395). Mobile-native cloud-build starvation (issue #2652) remains structurally unresolved but the dispatcher pressure has eased: only 2 open non-triage backlog items remain, both mobile-native-kmp. Four very-old unreviewed PRs (#2555, #2559, #2588, #2590 — 103d) are the main remaining drag.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Re-evaluate pm-devops-unblock-mobile-native-cloud-builds vs the new throughput picture. Mobile-native pressure dropped to 2/last 72h — decide formally: fix #2652, OR split the dispatcher queue, OR accept the current state and close the action | pm-tech-lead | action-list item resolved with a documented decision in `decisions.md` |
| medium | Add a scheduled (nightly cron) smoke of `SKIP_NETWORK=1 ./.claude/skills/verify-all.sh --quick` on `dev` HEAD so silent infra-only regressions surface without waiting for a PR — this was requested on 2026-08-31 and never landed | pm-backend | new workflow file `verify-dev-nightly.yml` running on cron + fails visibly in the Actions tab |
| medium | Drive a reviewer decision on #2555 (invoice lifecycle) and #2559 (PAY by square QR) — 103d open, author-owned, zero `reviewDecision` | pm-tech-lead | both PRs resolved (merged, split, or closed-deferred) with a decision log line |
| low | Close/rebase the two stuck expo-57 dependabot PRs #2588 and #2590 — expo SDK 57 already landed via sibling PRs this sprint; they're now superseded | none | both PRs closed or merged |
| low | Verify utoipa-swagger-ui cloud-build fix was codified (vendored / allowlisted / cached) so a runner-base rebuild does not re-trigger the Sept 403s | none | written note under `.research/management/decisions.md` or the risk's `mitigation` field names the exact fix mechanism |

## Risks

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| utoipa-swagger-ui egress 403 can re-trigger on runner-base rebuild if the Sept mitigation was ad-hoc. All 37 PRs merged cleanly this week, but the policy has not been shown fixed in writing | medium | high | Codify the fix (vendor/allowlist/cache) in a decision log entry; add a CI job that fails loudly if the prod swagger-ui URL is reached from the runner |
| Mobile-native cloud-build starvation is now a throughput risk, not a security one — the dispatcher pipeline has fewer KMP items queued, so visible pain is low, but every new finding still cannot run | medium | medium | Decide: fix #2652, split the queue, or accept and close the action. Each has an owner |
| Four PRs sitting unreviewed 100+ days (#2555/#2559/#2588/#2590) silently inflate open-PR drag on every stale-pr-guard and dependabot-related workflow run | high | low | This run's new action items force a resolution |

## Decisions needed

- **Mobile-native cloud-build policy (owner: pm-tech-lead):** given the pressure has eased, do we still commit to fixing #2652, or do we accept the current state and close pm-devops-unblock-mobile-native-cloud-builds as "won't fix / split queue formally later"?
- **Stale-PR eviction policy (owner: pm-tech-lead):** adopt a hard rule — any PR without reviewDecision at 60 days is auto-closed by stale-pr-guard — to prevent #2555-class drag from ever recurring?

## Open questions

- Was the Sept 2026 utoipa-swagger-ui unblock vendored, allowlisted or cached? The risk entry has been open since 2026-10-01; the fact that this week's 37-PR burst compiled green suggests the block is gone, but the mechanism is not documented.
- Is `screen-map.yml` on `main` actually stale because no screen docs have changed since June 2026, or because it stopped triggering? Last run on main was 2026-06-04 — worth one spot-check.
