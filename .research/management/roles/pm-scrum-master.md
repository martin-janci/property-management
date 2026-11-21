# pm-scrum-master — 2026-11-21

<sub>Always-on delivery synthesis for Phase 1.6.</sub>

## Summary

A big delivery wave: 32 PRs merged (#3650–#3686), with the three structural refactors pm-tech-lead escalated last run — push_fanout/voice_webhooks/oauth_integration_tests splits — all shipped in-window (#3659, #3685, #3686). Accounting MVP trio progressed (#3661 lifecycle + PAY-by-square QR closes #2555 / #2559). Issues #3649 + #3666 closed. Sprint spine (6/6 epics) remains done; delivery stays in continuous-hardening mode.

## Shipped since last run (2026-11-20 → 2026-11-21)

- #3659 — `push_fanout.rs` → `push/` module split (fcm/apns/adapter/worker/config)
- #3685 — `voice_webhooks.rs` module split (verification + handler + parsers)
- #3686 — `oauth_integration_tests.rs` split by flow (authorize/token/introspect-revoke/grants)
- #3669 — reality-server portal auth fix + buyer inquiry threads + notification settings + agency dashboard
- #3675 — feed delete + tenant application/forms 500 fixes + release 248 quarantined tests
- #3660 — reality-portal client alignment
- #3670 — reality-portal Authorization header fix
- #3661 — accounting-server invoice sent/cancel lifecycle + PAY-by-square QR (closes #2555 / #2559)
- #3666 — HelloSign webhook replay (issue closed)
- #3649 — suggestions seed-city for region `de` resolves to Wien (issue closed)
- Many code-review-* fix PRs across api-core, api-handlers, reality-web, mobile-rn

## Sprint progress

- **Sprint:** Hardening & refactor track (post-sprint — Epic 6/7A/8A/10A/10B/9 all done)
- **Epics done:** 6 of 6 (100%)
- **Coverage upkeep this run:** epic-82 (iOS SwiftUI) — 5/5 stories done; `last_checked` advanced for all five, no status transitions.

## Blockers

- **Dispatcher pipeline #3357** — 3 approved PRs still blocked on non-mechanical conflicts; the 32-PR merge wave did not drain it. Owner: `pm-tech-lead`.
- **KMP / mobile-native backlog** (CreateListingScreen stub, InquiriesResponse contract, Ktor HttpTimeout, SSO error body) — Gradle/KMP builds unavailable in cloud runner (#2652, #2966); claimable still ≈9/72. Owner: `pm-devops`.
- **Dependabot cargo-group bump #3668** — open, unmerged; needs rebase against post-split HEAD. Owner: `pm-devops`.
- **6 open PRs** (#3668 dependabot, #3687/#3688 pm-integration drafts, #3676/#3677 mobile-rn i18n drafts, #3682 search-drainer) — drafts need promotion-or-close decisions. Owner: `pm-scrum-master`.

## Next actions

1. **[high · pm-backend]** Sweep the new `push/` and `voice_webhooks/` child modules for the RLS-before-verify ordering bug that survived the split — DoD: handler ordering audited; sig-fail + RLS-fail regression tests merged.
2. **[high · pm-scrum-master]** Promote-or-close the 6 open PRs (#3668 rebase, #3687/#3688 drafts, #3676/#3677 mobile-rn i18n drafts, #3682 search-drainer) within 48h — DoD: each PR merged, closed, or has a dated owner + plan.
3. **[high · pm-tech-lead]** Resolve dispatcher stall #3357 — same blockers as last run; the merge wave did not drain it — DoD: #3357 closed with the 3 blocked PRs merged/reassigned and an escalation path documented.
4. **[high · pm-devops]** Unblock KMP + swagger-ui egress in the cloud runner (#2652, #2966) — DoD: cloud runner builds mobile-native/api-server; claimable backlog rebalances.
5. **[medium · pm-tech-lead]** Reset sprint-status.yaml — current sprint goal is fully "done"; next sprint goal should explicitly name the hardening/refactor track (new modules under watch, release-248 quarantine burn-down, mobile-native backlog) — DoD: sprint-status.yaml has a current sprint name, goal, and in-progress items.

## Risks

1. **[med/high]** Post-split modules (`push/`, `voice_webhooks/`, `oauth_integration_tests/`) are unproven under the next feature PR; a mechanical refactor + 32 landing PRs in one window is a lot of surface that hasn't been exercised end-to-end.  _Mitigation:_ freeze feature PRs on the new modules until a green integration run on `origin/dev` confirms no behavioural drift.
2. **[high/high]** Dispatcher stall #3357 persists across merge windows — the pipeline cannot absorb non-mechanical conflicts and will eventually deadlock on any hot-file cluster.  _Mitigation:_ define a conflict-escalation policy with a human-in-the-loop path within 48h.
3. **[med/med]** Open-PR drift (6 PRs incl. 4 drafts, 1 dependabot) + release-248 quarantine create accumulating review debt.  _Mitigation:_ promote-or-close sweep in 48h; quarantine-aging policy.
4. **[high/med]** Mobile-native backlog items (4 open) are compiled-but-dead stubs — cloud runner cannot catch them because KMP builds are blocked (#2652).  _Mitigation:_ pair every mobile-native backlog item with a contract test running on `jvmTest`; prioritize #2652 fix.
5. **[med/med]** 32 merged PRs in one window overloaded the per-PR reviewer; code-review coverage may have skipped items that would normally be caught.  _Mitigation:_ post-merge review sweep for the 32 PRs (`ppt-review-merged` on 24h window) to backfill findings.

## Open questions

- Did all three splits (#3659/#3685/#3686) keep test coverage 1:1 or did any test get dropped?
- Does #3661 fully close #2555 + #2559, and is #2558 still open?
- Which of the 4 open drafts are ready to promote vs still blocked?
- Did any of the 32 PRs skip the normal per-PR reviewer because of the merge-wave backlog?

## Decisions needed

- Promote-or-close policy for drafts open >72h — owner: pm-scrum-master
- Reviewer-slot allocation for large-scope PRs (accounting MVP learning) — owner: pm-tech-lead
- Reset sprint-status.yaml with the next hardening-track sprint — owner: pm-tech-lead
