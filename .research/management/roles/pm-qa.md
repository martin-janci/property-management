# pm-qa — 2026-11-08

<sub>Rotating role for this run (pm_cursor.next_index was 3). Static review only (no `gh` queries; context fed by orchestrator).</sub>

## Summary

The sprint-status.yaml sprint (Epics 6/7A/8A/10A) is fully done, so pipeline-stall remediation and the orphaned-PR re-lands sit outside any tracked story. The mobile auth screens have no dedicated tests — no test files alongside `LoginScreen.tsx`, `RegisterScreen.tsx`, `ForgotPasswordScreen.tsx`, or `TwoFactorScreen.tsx` in `frontend/apps/mobile/src/screens/auth/`. Release readiness is conditional until the re-lands have regression evidence.

## Next actions

| Priority | Action | Dependency | Definition of done |
|----------|--------|------------|--------------------|
| high | Re-land the three orphaned PRs (#3301 / #3333 / #3356) as fresh branches from current dev. Do not cherry-pick onto stale bases. Run `just verify` + full CI suites for each touched package (`backend.yml`, `frontend.yml`) before merging. | implementer / pm-dev | Each PR is rebased or re-authored on current dev, CI is green, and the diff against the original approved intent is reviewed line by line, since the conflicts were non-mechanical. |
| high | Add a post-re-land semantic-drift check. For each PR, list the tests the original PR added and confirm they still exist and run after conflict resolution. Check that no test was dropped or silently `#[ignore]`d / skipped. Re-run the originals' targeted tests by name. | pm-qa / implementer | A table of original test names against post-merge status shows 100% present and passing. |
| high | Define and test acceptance criteria for the Tier-1d signal generator dead-end fix. Cover: signal emitted, signal consumed, and an explicit failure path (logged error, retry or alert) in place of silent drop. Add one negative test where the consumer is absent and the signal must surface as an error or metric. | pm-dev / implementer | The fix has tests for the happy path and the no-consumer path, and a stalled signal produces an observable artifact such as a log line, metric or issue. |
| high | Write test suites for the four mobile auth screens. Start with `LoginScreen` and `TwoFactorScreen` (highest risk, security-critical). Cases: validation errors, submit loading and disabled state, API error and lockout/rate-limit message, 2FA invalid or expired code, backup-code path, navigation on success, and accessibility labels. Replace the App-level null mocks with a real-screen render using mocked auth context. | mobile-rn specialist | Four `*.test.tsx` files exist under `frontend/apps/mobile/src/screens/auth/` (or the repo's existing test location) and pass in jest-expo. Each covers happy, validation and API-error paths, with 2FA covering invalid code and backup code. |
| medium | Add a pipeline-health regression guard. A CI or scheduled check should flag approved PRs unmergeable for more than N hours, and flag `dev` history rewrites that orphan open PR bases. Document the force-push rule for `dev`. | devops / pm-dev | A check or runbook exists. A simulated stuck approved PR raises a visible alert. |
| medium | Supply fresh non-gated planned test-hardening vectors for the dispatcher buffer. Candidates: mobile auth-screen tests (split into 4 per-screen items), mobile announcement comment-thread read/post UI (follow-up named in the 6-3 note), mobile document-detail preview frame (7a-4 follow-up), and ppt-web document permission authoring UI tests (7a-3 note). | pm-dev / orchestrator | At least 6 non-mobile-native-gated items are queued with ACs and test types. |

## Risks

| Probability | Impact | Risk | Mitigation |
|-------------|--------|------|------------|
| medium | high | Non-mechanical conflict resolution in the re-lands silently drops or alters behavior or tests from the approved PRs, so approval no longer reflects what ships. | Require re-review of the re-landed diffs, check the original-vs-post-merge test inventory, and avoid auto-approve on re-landed PRs. |
| medium | high | The Tier-1d silent dead-end keeps the stall invisible after the PR fix. The failure could recur with no signal. | Add an explicit failure path and observability (log, metric or issue) with a test, and verify it before closing #3357. |
| medium | high | The four mobile auth screens (login, registration, password reset, 2FA; 979 LoC) have no tests, so a regression in the mobile auth flow could ship unnoticed. The auth flow is security-sensitive, and TwoFactorScreen depends on the MFA backend (epic 9). | Prioritize `LoginScreen` and `TwoFactorScreen` tests. Also add a contract check against the generated api-client types. |
| medium | medium | A dev history rewrite can orphan more open PR bases, and the pipeline can stall again. | Announce and gate dev rewrites, then rebase open PRs afterward or auto-comment on them. |
| high | medium | The sprint-status.yaml sprint is stale (every epic done, started 2025-12-21, test_hardening_batch still in-progress). QA cannot give a reliable release signal from it, and test_hardening_batch items such as #480 (WebSocket JWT in query param, marked high severity) still show open in the file. | Reconcile test_hardening_batch item statuses against actual GitHub state and add a current sprint covering the stall remediation. |

## Open questions

- Which exact PRs and files do #3301 / #3333 / #3356 touch (backend, frontend or mobile)? That decides the required regression suites, and I did not verify this.
- Is test_hardening_batch item #480 (WebSocket token logged in query param, severity high) actually still open? sprint-status.yaml shows `status: open`, but the other batch gates were closed in May.
- What is the intended behavior of the Tier-1d signal generator when no consumer exists: retry, alert or drop? This is needed to write the ACs.
- Where do mobile jest tests live (colocated or `__tests__`)? I found no existing test file next to the four auth screens, and I did not check for a separate directory.
- Is release readiness being assessed for a pending `dev → main` cut? If so, which version and which of the re-land PRs are in scope?

## Decisions needed

- Decide whether the re-landed PRs need a fresh human or agent review pass or whether the original approvals stand after non-mechanical conflict resolution — owner: pm-dev / orchestrator.
- Decide whether the Tier-1d fix and the mobile auth tests block the next `dev → main` release — owner: pm-dev.
- Decide on a policy for `dev` history rewrites (allowed or not, and with what notice to open PRs) — owner: devops / orchestrator.
