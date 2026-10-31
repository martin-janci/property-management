# pm-qa — 2026-10-31

**Last run:** 2026-10-31 (previous 2026-08-25 — overdue, picked by rotating cursor).

## Summary

The sprint-status file is stale and breaks its own gating rule: 8a-3 is marked
done while gates #480 and #484 are still open. Violations authz coverage (#3184)
is thin: 6 tests in the role-authz file cover only resident-deny and
manager-assign paths. The cross-org IDOR regression sits in a different file.

Static review only; three files read.

## Next actions

1. **(high)** Reopen or formally defer 8a-3-notification-preference-sync; triage
   issues #480 (WS token in query-param logged, no re-validation after JWT
   expiry) and #484 (serial dispatch, FCM stub swallows failures). *Owner:
   pm-scrum / rust-backend. DoD: sprint-status gates and story status agree;
   batch `thb-2026-05-25` closed or has explicit deferrals with owners.*
2. **(high)** Expand #3184 beyond resident-deny — cross-org (tenant-B manager
   gets 404) for every `{violation_id}` route (get/update/assign/resolve,
   appeals/decide, fines); add 401 + wrong-role 403 matrix; add non-member case
   for `create_violation`. *Owner: rust-backend. DoD: every route in
   `routes/violations.rs` has at least one 401, one wrong-role 403, one
   cross-org 404 test and backend.yml is green.*
3. **(high)** Verify `cross_org_assign_violation_is_not_found` is actually in
   the CI suite; investigate how it reached dev as RED despite backend.yml.
   *Owner: rust-backend / devops. DoD: test listed in cargo-test output and a
   recorded root cause.*
4. **(medium)** Add negative-path tests to the recently merged frontend/price
   work (#3185, #3186, #3189, #3190, #3191, #3197): error states, empty data,
   locale variants, boundary prices (zero / negative / NaN / currency
   rounding) for `_price.test.ts`. *Owner: react-web.*
5. **(medium)** Clean up `_bmad-output/implementation-artifacts/sprint-status.yaml` —
   duplicate `80-2-dispute-filing-flow` key, 2025-12-21 sprint metadata, record
   #483 open. *Owner: pm-scrum. DoD: YAML parses with unique keys; sprint
   reflects current work (violations hardening, code-review fixes); blockers
   list accurate.*
6. **(medium)** Write issue #483 IDOR tests for voice devices (cross-org
   device access; `list-commands` returning 403 vs empty-list leak) — use as
   IDOR test template for other routes. *Owner: rust-backend. DoD:
   `#[sqlx::test]` cases exist and #483 closed.*

## Risks

- **(high/high)** Authz/IDOR regression on the violations surface (4 touches
  in 14d). Role-authz test file has no cross-org or unauthenticated matrix.
  **Mitigation:** prioritise #3184 route-by-route matrix; add
  code-review checklist for cross-org + wrong-role tests on every new handler.
- **(medium/high)** CI gate not holding — red test on dev means backend.yml
  either didn't run or was bypassed. **Mitigation:** require every bug-fix
  PR to name red-before/green-after test; audit how red reached dev
  (suite registration, --no-verify, stale CI).
- **(high/medium)** Release-readiness overstated — 8a-3 done while #480
  (JWT in logs) and #484 (false 'sent' counts) open. **Mitigation:** treat
  #480 as release-blocking; revert 8a-3 to review; reconcile sprint-status
  at every release.
- **(high/medium)** Sprint-status data unreliable (stale metadata, duplicate
  key, no active work). **Mitigation:** named owner updates file per sprint.
- **(medium/medium)** Test bandwidth concentrated on single draft #3184;
  mobile RN+KMP + 6 locales have no visible new coverage this cycle.
  **Mitigation:** smoke-test floor in `mobile-native.yml` + RN jest.

## Decisions needed

- Does #480 (JWT in WS query-param logs) block next release, or is it
  accepted with mitigation? *Owner: security/tech-lead.*
- Policy: bug-fix PRs require named regression test red-before/green-after;
  merges blocked when pre-existing test is red. *Owner: eng lead / QA.*
- Mandatory authz test matrix (401 / 403 / cross-org-404) for every new
  mutation route. *Owner: rust-backend lead.*
- Who owns sprint-status.yaml reconciliation and how often? *Owner:
  pm-scrum.*

## Open questions

- Where does `cross_org_assign_violation_is_not_found` live, and was it ever
  excluded from or failing in CI? Which commit introduced it RED?
- Is the current sprint still Epic 6/7A/8A/10A, or has active work moved to
  violations and review-fix hardening? What's actually in-progress/review?
- Have #480, #483, #484 been triaged since 2026-05-25? Is 8a-3 →done a
  deliberate deferral?
- Does #3184 have a defined target matrix of routes by roles, or is it ad hoc?
- Any coverage / mutation thresholds in CI for backend and frontend?
