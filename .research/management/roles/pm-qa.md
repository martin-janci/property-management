# pm-qa — 2026-11-02

_Rotating role report (daily rotation slot: idx 3 → pm-qa). Previous pm-qa run: 2026-08-25 (69 days stale)._

## Summary

Sprint-status.yaml is a stale shell from 2025-12-21; every epic in it (6, 7A, 8A, 10A, 10B, 80) is `done`, no story is `in-progress`/`review`/`blocked`, and `blockers` is empty. The real work is the backlog + PR stream. `test_hardening_batch` still lists #480, #483, #484 as `open` but GitHub closed all three on 2026-05-26 — reconcile. The recent PR stream is test-bearing but KMP contract and security-negative gaps remain in the backlog.

## Tracking drift

- `test_hardening_batch` lists #480, #483, #484 as `open` → GitHub shows all three closed 2026-05-26
- Nothing in the batch notes mentions these closures (only #481, #482, #485, #486, #487 marked closed)
- 8a-3 and 8a-2 are `done` but their notes say the Epic 8A notification-dispatch work (8a-2 critical-override dispatch logic + mobile OS push) is **deferred**. Not verified whether regression coverage actually landed for #480 (WebSocket token logging + JWT-expiry re-validation) or #484 (serial dispatch, FCM stub swallowing failures).
- Closed ≠ tested. #483 is about missing voice-device IDOR tests — not verified they exist.

## Risk view for recent PRs

| Area | Risk | Priority | Test types needed |
|---|---|---|---|
| Violations handlers (#3210) | Cross-org 404 pins now exist for IDOR; role authz covered by a separate suite — matrix (role × endpoint) not verified | high | Integration matrix, negative cases |
| compliance_wave1b fixture consolidation (#3214) | Shared-fixture refactor can silently weaken or drop assertions | medium | Before/after test-count + assertion-diff check |
| Mobile-native KMP: inquiries-response-contract + create-listing-not-wired | Ready plans — contract bugs on Reality Portal path need contract tests | high | Contract tests vs. reality-server OpenAPI spec, KMP unit tests |
| `http-client-no-timeout` | Missing timeout needs a fault-injection test | medium | Ktor mock-engine timeout test |
| moderation-report-flood + appeal-no-status-guard | Flood path has no rate-limit test; status-guard gap is an invalid state-transition bug | medium | Rate-limit integration + state-transition negative |
| VoteDetailScreen (#3213 draft, #3208) | Test stabilization + dead i18n fallback removal; a dead fallback can hide missing keys in the 6 locales | medium | Locale-key completeness check, flake-rerun evidence |
| reality-web `_status` helper (#3212) + Accounting Invoice i18n (#3211) | Unit tests exist; dropping native dialogs changes behaviour; #3211 may lack a `ConfirmationDialog` a11y / keyboard test | low | Component + a11y tests |

## Release readiness

**Conditional green.** Five merged PRs all shipped test files; nothing visible blocks a release. Two things hold back a full green:
- Churn-flagged violations IDOR area still has score-1 backlog items.
- Two ready-for-dev KMP contract fixes with no test evidence yet.

## Next actions (top 5)

| Priority | Action | Owner |
|---|---|---|
| high | Reconcile `test_hardening_batch` in `sprint-status.yaml`: mark #480, #483, #484 closed (GitHub, 2026-05-26) and set the batch to `done`, or record why they were closed | pm-scrum-master |
| high | Add contract tests for `code-review-mobile-native-kmp-inquiries-response-contract` before merging its fix | mobile-native dev |
| medium | Add negative state-transition tests (moderation-report-flood, appeal-no-status-guard) + Ktor timeout test (http-client-no-timeout) to each fix | rust-backend / mobile-native dev |
| medium | Audit `violations_role_authz_tests.rs` and `violations_cross_org_idor_tests.rs` for a full role × endpoint matrix; confirm #3214 fixture consolidation kept the same test count + assertions | rust-backend |
| medium | Before merging #3213 and #3208: run locale-key completeness check across the 6 locales; re-run the stabilized `VoteDetailScreen` test repeatedly to confirm no flake | mobile dev |

## Risks (top 5)

| Risk | P | I | Mitigation |
|---|---|---|---|
| Sprint tracking is stale — no story-level acceptance criteria exist to test against; real work lives only in backlog + PRs, QA cannot gate on it | high | medium | Open a new sprint with explicit stories + acceptance criteria for the backlog plans |
| Closed hardening issues (#480 WebSocket auth, #483 voice-device IDOR, #484 dispatch) may have been closed without regression tests, batch state hides this | medium | high | Grep the test suites for each issue's scenario and record evidence in the batch |
| KMP Reality Portal contract + wiring bugs are `ready` but untested; mobile-native CI only runs Gradle unit tests, no contract check against the server | medium | high | Add contract tests against the OpenAPI spec before shipping |
| Fixture consolidation (#3214) + test-only stabilization (#3213) can mask regressions through weakened assertions or hidden flakes | low | medium | Compare assertion counts before/after; require repeated green runs |
| Moderation-report flooding + missing appeal status guard are abuse + invalid-state paths with no negative tests | medium | medium | Prioritise above the score-1 items |

## Decisions needed

- Open a new sprint or retire the current one so the backlog plans have a gating context — owner: pm-scrum-master
- Set a rule that every `ready` code-review plan must include a failing-first test before it is picked up — owner: pm-qa / tech lead
- Decide whether score-1 churn test files (violations, compliance_wave1b) get scheduled cleanup or are accepted as-is — owner: tech lead

## Open questions

- Were #480, #483, #484 closed by fixes with tests, or bulk-closed? Their closure times fall within ~15 s of each other.
- Does #3211 include a keyboard/focus test for the `ConfirmationDialog` that replaced the native dialogs?
- Does #3210 cover every violations endpoint, or only a subset?
- Is #3213 mergeable, and was the flake root-caused or only suppressed?
- Which sprint do the backlog plans belong to, given `sprint-status.yaml` shows everything done?
