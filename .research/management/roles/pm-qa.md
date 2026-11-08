# pm-qa — QA / test readiness lens (2026-11-08)

**Summary:** All 49/49 stories are done in tracking, but the test_hardening_batch is still in-progress according to sprint-status.yaml while its referenced gate issues (#480, #484) have been CLOSED on GitHub since. Test-gap vectors that don't need mobile-native are available in the shipped-without-sibling-test set and in reality-server contract edges, so the starved buffer can be refilled from static QA signals without waiting on the KMP unblock.

*Note: this run read only `sprint-status.yaml` and the prior `roles/pm-qa.md`; blocker/batch issue statuses were not opened. The routine itself verified #480/#483/#484/#3187/#3168/#3164/#3131/#2652 are all CLOSED on GitHub this run.*

## Next actions

1. **Seed non-gated backend test vectors** — priority: high · owner: rust-backend · DoD: 3 vectors with named target test files under `backend/servers/api-server/tests/`, each with a failing-on-main assertion. *Note: original pm-qa ask referenced issues #480/#483/#484; those are now closed. Reframe this action as covering regression tests for the fixed behavior — voice-device IDOR, WS JWT handling, dispatch_to_users failure accounting — now that the fixes shipped.*
2. **Create web + reality-server coverage-gap vectors** — priority: high · owner: pm-frontend · DoD: ≥8 vectors (ppt-web, reality-web, reality-server) as backlog candidates with target file paths, none needing mobile-native, each with an AC-style assertion.
3. **Quarantine and deflake `frontend/apps/ppt-web/src/routes/groups/rentals.connections.test.tsx`** — flake noted in PR #3356 body (confirmation-dialog findByRole timeout under parallel load). Reproduce in a repeat loop, replace waitFor timing with deterministic awaits — priority: high · owner: pm-frontend · DoD: 50 consecutive runs green in CI, or `test.skip` with a linked issue.
4. **Reconcile 8a-3 status against the test-hardening gate** — all gate issues now closed; sprint-status.yaml should reflect `done` (confirmed) or revert to `review`. Also fix duplicate 80-2 entry — priority: medium · owner: pm-tech-lead.
5. **Review 7 merged PRs for assertion quality** — sample 2 of 7: check #3353 session-revocation asserts revoked-token rejection across refresh/introspect/WS, and that registry tests assert behaviour vs render snapshots — priority: medium · owner: pm-qa.
6. **Define a cloud-testable substitute for mobile-native-gated items** — OpenAPI schema diff on generated Kotlin client, reality-server API contract fixtures, mock-server harness; produce a one-page test-layer matrix; re-tag ≥3 gated backlog items as partially testable — priority: medium · owner: pm-tech-lead.

## Risks

- **Dispatcher starvation (#3357)**: 66 of 72 backlog items are gated or dep-blocked (prob high / impact high) → use QA-generated gap vectors as the non-gated feed; add test-depth metrics (not just story-done counts).
- **Flaky rentals.connections.test.tsx** (prob medium / impact medium) → deflake or quarantine with tracked issue + a flake-rate report in CI.
- **mobile-native-gated stories cannot be exercised in cloud** (prob high / impact medium) → API contract tests in cloud; device/simulator evidence required before release.
- **Tracking drift**: 8a-3 is done while the gate rule (per prior pm-qa run) required #480/#484 closed/deferred, and sprint-status.yaml has a duplicate 80-2 entry — reconcile pass + lint check.

## Decisions needed

- Confirm 8a-3 remains done now that #480 and #484 are CLOSED; fix 80-2 duplicate — owner: pm-tech-lead.
- Approve QA-generated non-gated gap vectors as an official backlog feed for the dispatcher — owner: pm-product + pm-qa.
- Flake policy — blocking vs advisory after N failures with an owner — owner: pm-tech-lead.

## Open questions

- Root cause of the flake in `rentals.connections.test.tsx`; is it the only flaky test?
- Does the sprint-status 'sprint' header (Epics 6/7A/8A/10A, started 2025-12-21) still reflect the active sprint now that all its epics are done?
- Which of the 66 gated/dep-blocked items have a cloud-testable slice?
- Are the actions from the 2026-08-25 pm-qa run (concurrency test #2834, authz test #2821, CSV fuzz, voice-token round-trip) done or still open?
