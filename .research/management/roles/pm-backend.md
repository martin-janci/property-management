# pm-backend — 2026-11-21

<sub>Rotating backend lens; last ran 2026-10-26 (26 days). Static read; no compile/run.</sub>

## Summary

Major structural win this window: the three oversized backend files pm-tech-lead escalated last run — `services/push_fanout.rs` (3038 LoC), `routes/voice_webhooks.rs` (3066 LoC), `tests/suites/oauth_integration_tests.rs` (3427 LoC) — were all decomposed in-window (#3659 `push/` module, #3685 `voice_webhooks/`, #3686 `oauth_integration_tests/`). Accounting MVP trio progressed: #3661 landed invoice sent/cancel lifecycle + PAY-by-square QR, closing #2555 / #2559. The dependent #3666 HelloSign webhook replay and #3649 suggestions seed-region issues both closed. The remaining backend hotspots are now: post-split churn children (new modules receiving follow-up code-review items), reality-server surfaces broadened by #3669 (buyer inquiry threads + notification settings + agency dashboard), and the release-248 quarantined tests shipped by #3675.

## next_actions

1. **[high]** Sweep the new `push/` module (post-#3659) and `voice_webhooks/` children (post-#3685) for the RLS-before-verify ordering bug pm-tech-lead flagged — webhook signature verification must precede the `RlsConnection` extractor so every real Alexa/Google body does not return 401. — dep: none — DoD: handler ordering audited across push/voice modules, regression tests added for sig-fail and RLS-fail paths.
2. **[high]** Un-quarantine the release-248 tests introduced by #3675 (tenant application/forms 500 fixes + feed delete); establish a quarantine-aging policy. — dep: pm-qa — DoD: release-248 quarantine set empty or every remaining entry has an owner + deadline; CI gate on quarantine age.
3. **[medium]** Finish the accounting MVP trio: verify #2558 state after #3661 landed sent/cancel lifecycle + PAY-by-square QR; close as superseded or sequence the remaining slice. — dep: pm-integration — DoD: #2558 closed, superseded, or has a landing plan with an owner in action-list.
4. **[medium]** Address mobile-native stubs: CreateListingScreen `onSubmit` NotImplementedError + InquiriesResponse `page_size` / `limit` contract mismatch + Ktor HttpTimeout default. Package as one KMP PR per issue once #2652 egress unblocks. — dep: pm-devops — DoD: three mobile-native action-list items move to review with merged PRs; KMP CI runs for each.
5. **[medium]** Add regression coverage for #3669 reality-server portal auth fix + buyer inquiry threads + notification settings + agency dashboard so the broad surface does not regress on the next push_fanout touch. — dep: none — DoD: integration test per surface in reality-server; each has an authn negative + positive assertion.
6. **[low]** Characterize the post-split `push/` and `voice_webhooks/` child modules in sprint-status and coverage; the new file footprint should move to a "done, under watch" bucket for the next 2 runs so regressions surface. — dep: pm-scrum-master — DoD: sprint-status entry per new module, coverage.json `last_checked` updated.

## risks

1. **[prob=medium, impact=high]** Post-split modules (`push/`, `voice_webhooks/`, `oauth_integration_tests/`) carry hidden behaviour changes — a mechanical split can still move side effects across translation units; the smoke/integration suites must be exercised on dev HEAD before the next feature PR touches them. **Mitigation:** run the full backend integration suite on `origin/dev` now; freeze pre-approved feature PRs on these modules until the next pm-scrum-master run records green.
2. **[prob=high, impact=high]** The RLS-before-verify webhook ordering bug (handler runs `RlsConnection` extractor before signature verification → 401 on every live Alexa/Google body) survived a module split — mechanical refactor does not fix ordering. **Mitigation:** add an axum middleware ordering test + extractor-order lint in router registration; dedicated hardening PR after the split lands.
3. **[prob=medium, impact=medium]** Accounting MVP trio reviewer starvation finally broke this window (#3661 landed), but the underlying reviewer-slot policy is still undocumented — next large-scope PR will stall again. **Mitigation:** document reviewer-slot rotation policy for large-scope feature PRs as a follow-up to the closed risk.
4. **[prob=high, impact=medium]** Mobile-native stubs (CreateListingScreen `NotImplementedError`, InquiriesResponse contract) ship compiled-but-dead — Kotlin compile is green while the feature is broken; cloud runner cannot catch this because KMP builds are blocked (#2652). **Mitigation:** pair every mobile-native backlog item with a contract test on `jvmTest` (not `androidTest`); tag items that need a device run so pm-devops #2652 fix is prioritized.
5. **[prob=medium, impact=medium]** The `listings.rs` rebase trio (#3617 + #3628 follow-up + #3649 fix) is still open — sequencing not confirmed; rebase conflicts likely between the per-IP throttle (#3617) and the suggestions-seed region fix. **Mitigation:** rebase #3617 first (security-flavored), then #3628/#3649 fix on top; single reviewer pass over listings.rs after all three.

## open_questions

1. Did #3659 / #3685 / #3686 land with 1:1 test-coverage parity (no tests dropped or marked `#[ignore]` in the split)?
2. Where is the central webhook-signature-verification extractor now that `voice_webhooks/` is a module tree — is there one or did each provider inline it again?
3. Is #2558 still open after #3661, or did the invoice sent/cancel lifecycle land close it?
4. Which of the release-248 quarantined tests are flake vs real regressions?
5. Are the mobile-native InquiriesResponse + CreateListingScreen fixes gated only on #2652 egress, or is the fix text itself ready to land?

## decisions_needed

1. Policy: webhook route handlers must declare signature verification BEFORE any RLS/DB extractor — is this a lint, a review rule, or an axum extractor-ordering convention? — owner: pm-tech-lead
2. Quarantine-aging policy: every quarantined test must have an owner + deadline, or auto-delete at N days — owner: pm-qa
3. Reviewer-slot rotation for large-scope feature PRs (accounting MVP learning) — owner: pm-tech-lead
