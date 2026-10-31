# pm-scrum-master — 2026-10-31

**Mode:** always-on synthesis + role rotation picked `pm-qa` this run.

## Summary

Sprint "Epic 6, 7A, 8A & 10A" is finished on paper: 6/6 epics done, 49/49
stories done in `coverage.json`. The track is now review-fix, with 8 fix PRs
merged in 3 days. The headline risk is **dispatcher starvation**:
`claimable=17/72` (floor 36 — below half), WIP pool **8/8 human-gated**, 2
items dependency-blocked.

## Shipped since last run

- #3185 ppt-web neighbors stale-token useMemo
- #3186 ppt-web IoT WS hasGivenUp dropped
- #3188 cross-org IDOR fix in `assign_violation` (403 → 404), closes #3187
- #3189 mobile-RN vote-detail ranked ordering
- #3190 ppt-web authed-roots session-leak
- #3191 ppt-web invoice qty/unitPrice NaN guard
- #3197 reality-web listing-edit price-zero
- #3198 reality-server OS-RNG expect() panics removed

## Sprint progress

- Epics done: **6 / 6**
- Stories done: **49 / 49** (coverage.json upkeep from 2026-10-28)

## Next actions (top 5)

1. **(high)** Triage the 8/8 human-gated WIP pool — human maintainer.
2. **(high)** Refill planner — promote ready plans / backlog into
   claimable work. Owner: pm-tech-lead, pm-security.
3. **(medium)** Close or rebase stalled drafts #3075, #3108. Owner: human
   owner (martin-janci).
4. **(medium)** Clear stale dependabot PRs #2957, #2962, #2972, #3090
   (16–50 days old). Owner: pm-devops.
5. **(medium)** Audit sibling org-scoped handlers in `routes/violations.rs`
   after the IDOR fix in #3188; add cross-tenant tests. Owner: pm-security.

## Risks

- **(high/high)** Dispatcher starvation — claimable=17/72 below floor,
  WIP 8/8 human-gated. Mitigation: human triage then refill planner.
- **(medium/high)** Cross-org IDOR class — #3188 was found only after
  breaking CI; sibling handlers may share the flaw. Mitigation: audit
  org-scoping and add cross-tenant tests.
- **(medium/medium)** Backend `deny.toml` churned → dependency / license
  policy may have shifted; stale dependabot PRs could carry unpatched
  advisories. Mitigation: confirm cargo-deny on dev; close/merge the
  dependabot PRs.
- **(medium/medium)** Sprint-status.yaml stale — #480 open while 8a-3 is
  "done" (test_hardening_batch). Partial file read only — rest
  unconfirmed. Mitigation: reconcile against live GitHub issue state.
- **(low/low)** Routine lagged 3 days — some context missing; churn in
  reality-web agency/[slug] page not explained by any listed PR.

## Decisions needed

- Close current sprint and define next sprint goal (6/6, 49/49). Owner:
  human maintainer / product.
- Policy on stalled drafts and dependabot PRs (close vs rebase vs
  auto-merge). Owner: human maintainer / pm-devops.
- Whether to loosen the human-gate on low-risk WIP so the dispatcher
  can refill. Owner: human maintainer.

## Open questions

- Is issue #480 still open? sprint-status marks it open; story 8a-3 is done.
- Which 8 items make up the human-gated WIP pool, and what approval does
  each need?
- Which 2 items are dependency-blocked, and on what?
- What changed in `backend/deny.toml` this run (intentional policy change
  or advisory ignore)?
- Formally close the current sprint and define the next goal?
