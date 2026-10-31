# pm-qa

_Generated: 2026-10-31 — daily routine Phase 1.6 rotating role_

## Summary

Test discipline on research-driven PRs is strong this week (5 of 6 merged PRs shipped regression tests; #3188 is covered by earlier #3183/#2744, and #3190 is a test-only audit). The real gaps are structural. Two ready mobile-native-kmp plans cannot land in the cloud runner, 5 recently touched hotspots lack a coverage view, and #3119 plus the open test-gap backlog are going stale. Based on the supplied Phase 1 data plus sprint-status.yaml only; I did not verify PR contents, CI results, or the churned files.

## Next actions

- [high] Route the two ready mobile-native-kmp plans (inquiries-response-contract, create-listing-not-wired) to a runner with Android egress (local/operator or a CI job with AGP access), or mark them 'operator-gated' in the plan index — dependency: pm-devops — DoD: Each plan has an assigned executable runway, or is explicitly parked with an owner, so it stops counting as 'ready'
- [high] Add a JSON-contract test for /inquiries (decode a recorded server fixture with kotlinx-serialization, covering the missing field behind MissingFieldException) that runs in the commonTest/JVM layer with no AGP, or a backend OpenAPI-vs-DTO contract check — dependency: pm-backend — DoD: A test fails on the current contract mismatch and can run without the Android toolchain
- [medium] Triage #3119 (NewsScreen i18n defaultValue, open 13d, non-draft, no reviewDecision): request a review or merge, or convert to draft/close with a reason. Confirm it ships a test asserting localized strings with no defaultValue fallback — dependency: pm-dev — DoD: #3119 has a reviewDecision, or is merged or closed, within 2 days
- [medium] Gate the open drafts #3192, #3193, #3197, #3198 on a regression-test checklist before ready-for-review. Each needs a negative or edge test: appeal status guard illegal transitions, duplicate report dedup race, price=0 and NaN, RNG failure path — dependency: pm-dev — DoD: Each PR body lists the test file and the failing-before/passing-after evidence
- [medium] Add a coverage review for hotspots reality-web agency/[slug]/page.tsx (5 changes in 14d) and backend violations.rs (5 changes in 14d), plus compliance_wave1b_tests.rs. Check for a page-level test (SSR 404, locale, slug edge cases) and a role-by-endpoint authz matrix for violations (403 vs 404 per role) — dependency: pm-dev — DoD: A matrix of role x endpoint x expected status exists as tests, and a page test exists for the agency slug route
- [low] Schedule the open test-gap backlog items (comparison-parse-shape localStorage tamper crash first; then toast-setstate-after-unmount, invoice-feature-no-i18n, listing-edit-i18n-sk), and add a defensive-parse test for the tamper case — dependency: none — DoD: Comparison parse has a test with malformed localStorage and does not crash; the remaining items are ticketed with owners

## Risks

- [high / high] mobile-native-kmp has high-confidence ready plans that are structurally unlandable in the cloud runner (AGP egress blocked, #2652). Known runtime bugs (MissingFieldException on every /inquiries call, realtor submit hardwired to NotImplementedError) persist in the Reality mobile app with no test net or delivery path
  - Mitigation: Operator or local runway, JVM-only contract tests, and a release-notes caveat or feature flag until fixed
- [medium / high] Repeated authz and IDOR patching in violations.rs (5 changes in 14d, including 403->404). Regressions could re-open cross-tenant existence leaks if the role x endpoint matrix is not exhaustive
  - Mitigation: Table-driven authz tests across all violation endpoints (anonymous, tenant, other-org manager, owner), including the 404-not-403 existence-leak assertion
- [high / medium] UC-ACC Phase-A accounting has been in review for 91 days (#2555, #2559), operator-gated. Accounting involves money handling, so the longer it waits the larger the unreviewed test and diff surface and the harder the merge, and no release-readiness signal exists for it
  - Mitigation: Operator decision on merge, split or close; rebase and re-run CI; list the accounting rounding and NaN edge cases alongside the AccountingInvoiceForm guard from #3191
- [medium / medium] Coverage is reactive rather than preventive. The sprint-status.yaml header still names Epic 6/7A/8A/10A even though every epic is done (reconciled 2026-10-26), so the active sprint has no defined acceptance criteria to review for post-MVP hardening. Also, 4 drafts and #3119 sit open without explicit test gating
  - Mitigation: Define a hardening sprint goal and exit criteria, and require a regression test or a documented waiver per fix PR
- [medium / low] Client-side robustness gaps remain open: the localStorage tamper crash in reality-web comparison, the toast setState after unmount, and missing i18n in the invoice and listing-edit sk locales. These can surface as user-visible crashes or untranslated UI in the sk/cs/de markets
  - Mitigation: Fix with unit tests; add an i18n key-parity lint or test across sk, cs, de, en

## Open questions

- Is there any non-cloud runner (a self-hosted or operator machine) that can execute mobile-native Gradle builds for the two ready KMP plans, and who owns it?
- Do #3183/#2744 cover all violations.rs role/authz paths changed in the last 14 days, or only the 403->404 IDOR case?
- Does the agency/[slug]/page.tsx hotspot have any existing test (SSR, ISR, 404, locale), and what drove 5 changes in 14 days?
- What is the blocking reason for #3119 (no reviewer assigned, or CI failing) and does it include a test?
- What is the release-readiness target (a dev->main cut date) for post-MVP hardening, and are #2555/#2559 in or out of scope for it?

## Decisions needed

- Decide whether the two unlandable KMP plans move to an operator-run lane or are deprioritized, which would stop them showing as ready — owner: pm-orchestrator
- Decide whether #2555/#2559 UC-ACC Phase-A are merged, split, or closed after 91 days — owner: operator/pm-po
- Decide on a policy that fix PRs must include a regression test or an explicit waiver — owner: pm-qa/pm-dev
