# pm-scrum-master — delivery synthesis (2026-11-08)

**Summary:** Delivery is at 49/49 stories and all sprint epics done. Seven PRs merged in 2 days, mostly registry i18n/auth/toast re-work plus UC-57 session revocation. The dispatcher pipeline is stalled (#3357), and the claimable buffer is 6/72 with no non-gated backlog.

## Sprint progress

- Sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- Epics done: **6 / 6**

## Shipped since last run

- #3353 UC-57 session-revocation (revoke other sessions on password change; closes #3352)
- #3359 RegistryPage mutation toast surfacing + i18n + tests (re-land of #3356)
- #3358 Screen-map drift reality docs/screens update (re-land of #3333)
- #3355 RegistryPage chrome i18n
- #3354 Registry requests carry auth context
- #3351 Registry pet/vehicle i18n
- #3350 Reality-web shared-comparison body validation

## Blockers

- **#3357 dispatcher pipeline fully stalled** (owner: pm-devops / operator) — 3 approved PRs were blocked on non-mechanical conflicts after dev history was rewritten, and Tier-1d signals are a dead end. #3301, #3333, #3356 were closed; #3333 and #3356 were re-landed as #3358 and #3359. Operator action still needed on #3301 and on the Tier-1d ingestion gap.
- **buffer-low: claimable=6/72, backlog avail=0** (owner: pm-tech-lead / pm-scrum-master) — All open backlog is mobile-native/KMP-gated (#2652 cloud builds) or dependency-blocked. Coverage is 49/49 done, so there are no gap stories to pull.
- **#3301 reality-server inquiry FK classification** (owner: pm-backend) — Closed unmerged and not yet re-landed.

## Next actions

1. **Re-land #3301** (reality-server inquiry FK classification) as a fresh branch off current dev — priority: high · owner: pm-backend · DoD: new PR merged with CI green, or an explicit decision to drop it recorded.
2. **Resolve #3357 stall** — operator rebases or retires orphaned branches and fixes the Tier-1d dead-end rule — priority: high · owner: pm-devops · DoD: #3357 closed and dispatcher claims at least one new item.
3. **Seed fresh non-gated backlog vectors** — reality-server and ppt-web code-review sweeps, refresh-token localStorage security fix, post-registry-rework audit on other ppt-web pages — priority: high · owner: pm-tech-lead + pm-security · DoD: ≥10 new claimable web or reality-server items in backlog.json.

## Risks

- **Dev history rewrites keep orphaning branches and PRs** (prob high / impact high) → agree a no-force-push-on-dev policy; re-land via cherry-pick onto the current dev tip.
- **Backlog starvation: only KMP-gated items remain** (prob high / impact medium) → seed non-gated vectors; unblock #2652 cloud builds.
- **RegistryPage i18n/messages churn cluster** (prob medium / impact medium) → add an i18n key-parity check to CI; serialize PRs that touch messages/*.json.
- **Security item ppt-web refresh-token in localStorage (XSS-reachable) remains open** (prob medium / impact high) → prioritize as top non-gated security item (httpOnly cookie or in-memory token).

## Open questions

- Is the dev history rewrite complete, and are other open branches or worktrees still based on the old history?
- Does #3358 fully close screen-map-drift-pr-3293-reality (this run marked it done based on PR-body match)?
- Which of the carried blockers (#3187, #3168, #3164, #3131, #2652) are still open? All are shown CLOSED on GitHub this run.
- What is the Tier-1d dead-end condition in #3357, and is it a dispatcher rule bug?

## Decisions needed

- Re-land or drop #3301 — owner: pm-backend.
- Approve operator intervention on the pipeline (reset or retire orphaned branches) — owner: pm-devops / operator.
- Policy on dev history rewrites, and whether the dispatcher should auto-re-land on non-mechanical conflicts — owner: pm-tech-lead.
- Fund a KMP cloud-build unblock (#2652) or keep treating the mobile-native vector as gated — owner: pm-devops.

## Role focus today

`pm-scrum-master` (always-on): restore flow by clearing the #3357 stall, re-landing #3301, and refilling the backlog buffer with non-gated web and reality-server vectors. Rotation role: `pm-qa` (last run 2026-08-25, 75d stale).
