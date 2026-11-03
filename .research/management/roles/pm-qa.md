# pm-qa — rotating role report

_Generated: 2026-11-03 (routine Phase 1.6, 70-day stale rotation slot)_

## Summary

Test discipline is up this window — all 5 merged PRs (#3216 docs, #3217 reality-web, #3218 ppt-web, #3219 reality-server, #3220 ppt-web) shipped a two-commit `test:`/`fix:` TDD split with explicit fails-before-fix evidence (IG3). But VoteDetailScreen.test.tsx is on its 2nd consecutive churn run (PRs #3189 → #3209), the pure-Rust announcement fan-out model test still fakes the RLS predicate (carried risk from 2026-07-23), and #3187 (`cross_org_assign_violation_is_not_found` expects 404, gets 403) continues to block backend CI on `dev` — so the gating fault is a test-vs-code contract gap, exactly pm-qa territory.

## Risk-based test view (top gaps this cycle)

| Area | Risk | Priority | Test type needed |
|---|---|---|---|
| violations.rs `assign` cross-tenant contract | Dev CI red; manager-role 403 masks the cross-tenant 404 contract (#3187) | **high** | integration (axum + sqlx), mirror of `community_cross_tenant_idor_tests.rs` |
| mobile-rn voting feature | VoteDetailScreen.test.tsx churn 2/2 — suite is unstable/under-asserting (ranked-ballot + i18n fallback bugs both landed on top of green tests) | high | add contract test for `vote.cast` POST + i18n lockstep guard |
| Announcement cross-tenant fan-out (carried #2484) | SQL predicate re-modelled in pure Rust — RLS could regress silently | medium | replace with `#[sqlx::test]` integration pinning the actual policy |
| Dispute KPIs endpoint (carried) | Quarantined test in main; shape regression could ship undetected | medium | un-quarantine + add window validation + consumer-contract test |
| Mobile-rn form submission pattern | 3 "wire-the-button-but-not-POST" bugs in 7 days — chronic defect class | high | per-screen MSW smoke asserting Submit emits POST with payload |

## Release-readiness signal — **AMBER**

- `backend.yml` on `dev` is currently red (#3187 and #3168 cargo-deny). No backend PR can merge until either is cleared.
- Frontend lanes green. 5513 ppt-web tests + 378 reality-web tests pass.
- No release-blocking regression in the merged window; all 5 PRs carried green CI + fails-before-fix proof.

## next_actions

```json
[
  {"action":"Decide the cross-tenant 404-vs-role-403 contract for violations.assign and either fix assign_violation to return 404 cross-tenant OR adjust the regression test's role shape — unblocks backend.yml on dev (#3187)","priority":"high","dependency":"pm-security","definition_of_done":"backend.yml green on dev; cross_org_assign_violation_is_not_found passes with the chosen contract documented in violations_mutation_cross_org_idor_tests.rs"},
  {"action":"Open integration-level sqlx replacement for the pure-Rust announcement fan-out model test (#2484 carried)","priority":"medium","dependency":"none","definition_of_done":"announcement_targeting_rls_integration_test.rs lands and the pure-Rust model is deleted or demoted to a unit-only sanity check"},
  {"action":"Add VoteDetailScreen.test.tsx contract assertion that onSubmit emits POST to /api/v1/votes/{id}/cast with the collected ballot — close the ranked-ballot regression class driving the 2nd churn run","priority":"high","dependency":"pm-frontend","definition_of_done":"test fails before the next ranked-ballot regression; VoteDetailScreen stops hitting the hotspot list"},
  {"action":"Un-quarantine the dispute KPIs test and add window_start<=window_end validation + reporting-consumer-contract case (carried from pm-backend 2026-07-30 risk)","priority":"medium","dependency":"pm-backend","definition_of_done":"dispute KPIs test executes in default CI run; shape regression surfaces before merge"},
  {"action":"Adopt the per-screen MSW/Mirage submit-smoke gate (mobile + reality-web): any form with a Submit button must assert the network call emits with the collected payload (closes the fake-submit-pattern cluster class)","priority":"high","dependency":"pm-frontend","definition_of_done":"CI job rejects a PR that adds a Submit handler without an MSW assertion; retrofit at least ReportFault + reality-web /sell + VoteDetail"},
  {"action":"Audit sprint-status.yaml drift vs coverage.json — the 2026-10-26 reconcile pass still leaves blockers list unchanged although #3187 is now the dominant one; make sprint-status the single source of truth for CI-gating blockers too","priority":"low","dependency":"pm-scrum-master","definition_of_done":"sprint-status.yaml `blockers:` reflects #3187/#3168/#3164/#3131 or explicitly delegates them to action-list.json"}
]
```

## risks

```json
[
  {"risk":"VoteDetailScreen.test.tsx is on consecutive-run churn (runs_seen=2) while mobile-rn voting bugs keep landing on top of green tests — the test suite is passing but under-asserting the ranked-ballot + i18n lifecycle, so each bug costs a round-trip","probability":"high","impact":"medium","mitigation":"Convert the test from render-smoke to behavior-contract: assert cast-POST payload, ranked ballot ordering, and i18n defaultValue fallbacks; freeze the test shape with a lockstep guard"},
  {"risk":"Backend CI on dev is red (#3187 + #3168) long enough that implementers start merging around it or shipping without re-running CI — classic broken-windows for a test gate","probability":"medium","impact":"high","mitigation":"Treat #3187 as a release-blocker: pm-security owns the contract decision today, pm-qa validates the fix; add a routine Phase-6 alarm when dev CI stays red >24h"},
  {"risk":"Three 'wire-but-no-POST' mobile/web regressions in 7 days suggest the e2e-smoke gap pm-frontend already surfaced is still not shipping — pm-qa must co-own the eslint/MSW gate roll-out or the pattern will keep landing","probability":"high","impact":"high","mitigation":"Co-sign the lint+MSW gate action with pm-frontend; QA DoD includes a per-screen Submit-POST smoke before any new form-screen merge"}
]
```

## open_questions

- Is the intended cross-tenant contract for violations.assign 404 (opacity, matches community IDOR pattern) or 403 (role-gate first)? pm-security owns this call; without the answer #3187 cannot land.
- Should the dispute KPIs test un-quarantine happen before or after the carried pm-backend window_start validation lands? Sequence matters for TDD evidence.

## decisions_needed

- Cross-tenant 404-vs-403 contract for violations mutation handlers — owner: pm-security
- Adopt per-screen Submit-POST MSW gate across mobile + reality-web — owner: pm-frontend (with pm-qa co-signing the DoD)
