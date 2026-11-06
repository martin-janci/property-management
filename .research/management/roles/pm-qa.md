# pm-qa — 2026-11-06

_QA/test lens. Daily PM rotation (idx 3 → 4). **Rotation came up 73 days stale** (last run 2026-08-25). Static, read-only._

## Summary

The round-5 authz suites (`round5_sec_b_authz_tests.rs` 935 lines, `r5_sec_a_authz_tests.rs` 377, `registry_read_scope_tests.rs` 287) are wired into the harness (`suite_5.rs` + `suite_7.rs`) and have no `#[ignore]`, so they are not compile-only. The sprint-status file still shows every tracked epic as `done` with nothing `in-progress`/`review`/`blocked`, so the real release risks are (a) mobile coverage blocked by the long-carried jest-expo (#2951) and KMP cloud-build (#2652) issues, and (b) auto-approval of the 5 draft screen-map-drift PRs #3333-#3337 without a required-status `/screens validate` check. The two ready mobile-native-kmp plans (`inquiries-response-contract`, `create-listing-not-wired`) still have no implementer assigned — mobile defect debt persists.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Confirm in CI logs that `suite_5` + `suite_7` execute the round5_sec_b / r5_sec_a / registry_read_scope tests with non-zero pass counts on a DB-backed job; spot-check each file for cross-tenant/wrong-role negative assertions (401/403/404), not just happy-path 200s | pm-backend | CI run link with test names and counts; note on any test lacking a negative assertion |
| high | Hold draft screen-map auto-fix PRs #3333-#3337 until `screen-map.yml` (`/screens validate`) is green; block auto-approve until the check is required-status on `dev` | pm-devops | All 5 PRs show a green screen-map check before merge or are closed |
| high | Assign implementers to the two ready mobile-native-kmp plans (`inquiries-response-contract`, `create-listing-not-wired`); each needs a named acceptance test (contract or unit) | pm-frontend | Owners named; each plan has a named test in its acceptance criteria |
| medium | Decide on `jest-expo` blocker #2951 and KMP cloud-build blocker #2652: either unblock or move mobile test gating to a local / CI-only path; state which mobile suites actually gate release | pm-devops | Each issue has an owner + decision; mobile test status reported in the next routine run |
| medium | Review churn-hotspot test PR #3339 (suspend_cascade) for flakiness / seed isolation — same class of bug as earlier `document_folder` shared-UUID issue; look for hardcoded IDs or ordering dependence | pm-backend | Review comment posted; no hardcoded IDs or ordering dependence in the new suite |
| low | Add a QA gate requiring every authz-fix PR to include a negative test that fails without the fix; spot-check by reverting the fix in a branch for one of #3330/#3331/#3332/#3338 | none | Mutation spot-check result recorded for one round-5 PR; rule added to PR template or review checklist |

## Risks

| Risk | Prob | Impact | Mitigation |
|---|---|---|---|
| Mobile RN + KMP have thin automated coverage because of jest-expo and KMP cloud-build blockers persisting since August — mobile regressions could ship undetected | high | high | Unblock #2951 and #2652, or run mobile tests locally / CI-only and report their status each run |
| Screen-map auto-fix PRs #3333-#3337 could merge without `/screens validate` running if the check is advisory — docs and screen status left failing validation | medium | medium | Require `screen-map.yml` as a required status on PRs touching `docs/screens/**`; block auto-approve on draft screen-map PRs until green |
| Round-5 authz suites may give false confidence if they primarily assert happy paths or run on a BYPASSRLS pool — cross-tenant / wrong-role negatives would go untested | medium | high | Sample assertions in each file for 401/403/404 cross-tenant cases; verify the test pool is non-BYPASSRLS (as the document-folder 404-non-leak tests assume) |

## Open questions

- Which CI jobs in `backend.yml` run `suite_5` and `suite_7`, and are they sharded with a DB service? Agent can't see live CI logs from here.
- Do the 11 `gh-issue-*` PRs (#3318-#3328) each add a regression test, or only a code fix? (The merge messages suggest tests, but a scan of each diff would confirm.)
- Is the screen-map check a required status on `dev`, or advisory only?
- What is the new active sprint, since `sprint-status.yaml` has no `in-progress`/`review`/`blocked` stories?
- Do the round-5 authz tests run against a non-BYPASSRLS DB role?

## Decisions needed

- Make the `screen-map.yml` check a required status for auto-fix PRs — owner: pm-devops.
- Define the release-gate policy for mobile (RN + KMP) while the CI blockers persist — owner: pm-orchestrator (or pm-scrum-master).
- Prioritise the two ready mobile-native-kmp plans in the next sprint — owner: pm-product (or pm-scrum-master).
