# pm-qa — 2026-11-07 (routine Phase 1.6 role rotation)

_Last run 2026-08-25 (74 days ago). Static-analysis only; agent read ≤8 files._

## Summary

Sprint-status shows every epic done and the `test_hardening_batch` still marked in-progress, with #480 still `status: open`. The open QA risk is **test fidelity and test-shadow debt**, not missing stories. Several open security/authz issues (#2946, #3164, #2574, #2486) have no named regression tests, and the 83-day-old mobile-rn lint gap is still unmitigated.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Write a cross-tenant regression test for violations `add_evidence` (#3164) that drives the real route as a user from another org and expects 403/404. Cover dispute `add_evidence` (#2483/#2490) in the same sweep, reusing the pattern from the #3338-#3340 inquiry-idor tests. | pm-backend | A `#[sqlx::test]` fails on main for the cross-org user and passes after the guard. Same-org manager succeeds; unauthenticated caller gets 401. |
| high | Add a failing-first test for portfolio_analytics cross-tenant IDOR (#2946), where `org_id` is computed and then discarded. The test seeds two orgs and asserts that org A's metrics never include org B's properties. | pm-security | A two-org seed test is in the PR with #2946 and asserts exact counts per org. Red before fix, green after. |
| high | Add an integration test that exercises the real announcement fan-out RLS and targeting SQL, replacing the pure-Rust re-model (#2484). Include a cross-tenant `target_ids` case. | rust-backend | An sqlx test creates an announcement in org A with a foreign-org `target_id` and asserts zero org-B users receive it. The re-model test is deleted or demoted. |
| high | Add a happy-path callback test for the Android SSO CSRF state fix (#2574), where `SsoStateStore.mint()` has no call site. Also add a mobile layout-cache test (#2486) that asserts the cache key includes `tenant_id` and is purged on logout. | pm-mobile | Test asserts that a mint-then-callback flow succeeds and a callback without mint is rejected. Separate test asserts a user B login after user A logout cannot read A's layout. |
| medium | Un-quarantine the dispute KPI test (#2575) and add `window_start <= window_end` negative cases covering equal, inverted and missing bounds. Add a shape-assertion for the response. | pm-backend | Quarantine attribute removed and CI green. Inverted window returns 400; response schema asserted. |
| medium | Land `eslint-plugin-react-hooks` + `i18next/no-literal-string` gate on `frontend/apps/mobile` as warn-only. Then add submit-wiring smoke tests for the mobile-rn ranked-ballot form and the other fake-submit screens, asserting the mock API is called with the expected payload. | pm-frontend | Lint step runs in `frontend.yml`. Each listed form has a test that fails when the submit handler does not call the API. |

## Risks

- **Cross-tenant IDOR regression-test gap persistent** (high probability × high impact). The #3338-#3340 pattern covers only the inquiry routes; the same `add_evidence` IDOR has now appeared in two modules (disputes + violations). Mitigation: systematic sub-resource authz sweep with a shared two-org test helper; require failing-on-main test in every IDOR fix PR.
- **Cloud runner egress blocks backend+mobile CI** (high × high). #2966 (swagger-ui), #2652 (AGP/Gradle), and the RUSTSEC cargo-deny gate (#3168) mean backend/mobile-native tests cannot be verified or merged in the cloud runner. Fixes can ship without CI-confirmed tests, or stall. Mitigation: prioritize the egress unblock and a bounded cargo-deny exception; require local test evidence in each PR description until then.
- **Mobile-rn lint gap now 83 days chronic** (high × medium). A new fake-submit cluster (buttons not wired to the API) points the same way. Mitigation: adopt the lint gate as warn-only first, then make it blocking; add submit-to-API smoke tests.
- **sprint-status / #480 tracking drift** (medium × high). Every epic shown done, yet `test_hardening_batch` is in-progress and #480 (WS token in query-param logs) is `status: open` while 8a-3 is marked done. Mitigation: reconcile #480 against the live issue state; either confirm closed/deferred or revert the 8a-3 gate; add a regression test for the WS token log redaction.
- **2026-08-25 pm-qa actions test-shadow debt unmeasured** (medium × high). Prior actions (#2834 quiet-hours atomic-claim, #2838 voice OAuth encryption, #2827 CSV fuzz, #2833 AML dialog reset) have unknown status after 74 days. Mitigation: audit each for a test file in its merged PR; re-raise any still missing.

## Open questions

- Is #480 (WS auth token in query param) closed or deferred? The file shows `status: open`, yet 8a-3 is marked `done`.
- Which of the 2026-08-25 pm-qa actions (#2834, #2821, #2835, #2827, #2838, #2833) have since landed with tests?
- Did PRs #2490 (dispute `add_evidence`) and #2946 (portfolio_analytics) merge, and did they ship with regression tests?
- Is #2574 (SsoStateStore.mint() unwired) still open, and what test covers the Android SSO callback?
- Is a test-shadow CI diff-guard (a fix PR must carry a failing-first test) adopted, or still undecided as of 2026-08-25?

## Decisions needed

- Make a failing-first test file mandatory for dispatcher-spawned fix PRs, enforced by a CI diff-guard — owner: **pm-tech-lead + pm-qa**.
- Roll out the mobile-rn lint gate as warn-only for one sprint, or blocking from day one — owner: **pm-frontend + pm-qa**.
- Fund a shared two-org IDOR test helper and sub-resource authz sweep as a standing QA gate — owner: **pm-backend + pm-security**.
