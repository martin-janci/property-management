# pm-qa — QA / Test lens (2026-11-06)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open issues. Previous run: 2026-08-25 (73 days stale)._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "21 PRs merged in this window materially improved tenant-isolation coverage — Round-5 brought regression suites for 11+ authz items, property_cluster_org_scope_tests, round5_sec_b_authz_tests, suite_7 person_months, and Decimal/money precision. The remaining test-shadow debt is clustered on the three 'Still open' items in #3332 (accounting by-id, webhook auth-header smuggling, onboarding ResolvedTenant) — they shipped without regression coverage because they were explicitly deferred; they need failing-on-main tests before a fix PR enters the merge queue, per the DoD policy pm-qa proposed in prior rotation.",
  "next_actions": [
    {"action": "Add regression tests for accounting-server by-id endpoints — invoice, contact, match reads must return 404 on cross-tenant id (matching the shape #3332 landed for the rest of api-server)", "priority": "high", "dependency": "rust-backend", "definition_of_done": "Test asserts cross-org id → 404 for each accounting by-id GET/DELETE/PATCH; added to backend/servers/accounting-server/tests/ or equivalent; passes against the fix; fails without it."},
    {"action": "Add webhook-headers auth-smuggling test — EnhancedWebhookSubscription create/update must reject or strip Authorization/Cookie/X-API-Key etc.; schema omits them from the read response", "priority": "high", "dependency": "rust-backend", "definition_of_done": "POST + PATCH webhook sub tests cover each known auth header name; assertion: write-path 400 or stripped; read-path schema excludes the field; redaction confirmed on a reserialize."},
    {"action": "Add onboarding ResolvedTenant regression — POST /onboarding must consult ResolvedTenant, not the X-Tenant header; a foreign org header returns 403", "priority": "high", "dependency": "rust-backend", "definition_of_done": "Onboarding test spins a user in org A, posts with X-Tenant: org-B → 403; tests role selection reflects ResolvedTenant only; both negative and positive branches covered."},
    {"action": "Add db-test suite 4 or 5 regression for audit_logs ON DELETE SET NULL conflict in normal-operation delete paths (not just seed --force) — assert the trigger behaviour matches the design decision from the pending ADR", "priority": "medium", "dependency": "pm-tech-lead", "definition_of_done": "Once the ADR lands, write a db-level integration test that deletes a row whose FK targets audit_logs; asserts the documented outcome (CASCADE, NO ACTION, or ACCESS EXCLUSIVE escape)."},
    {"action": "Consolidate the Round-5 suites under a single 'required-checks' bucket in CI — check-rls-enforcement.sh --strict + suite_7 property_cluster_org_scope_tests + round5_sec_b_authz_tests + round5_sec_a_authz_tests should be marked required on PRs touching the hardened routes", "priority": "medium", "dependency": "pm-devops", "definition_of_done": "CODEOWNERS or workflow filter maps route path globs to the authz suites; a PR that disables or skips any of them fails required checks."},
    {"action": "Add i18n regression for mobile RN Alert.alert hardcoded strings — DocumentDetailScreen.tsx:118, WidgetSettingsScreen.tsx:66/79, FeedbackScreen.tsx:216 all pass raw English through Alert.alert in a 4-locale app", "priority": "medium", "dependency": "pm-frontend", "definition_of_done": "Lint rule or vitest assertion rejects Alert.alert argv[0] that isn't a t() call or a resolved string ref; the 4 call sites refactored to use t(); sk/cs/de keys added."}
  ],
  "risks": [
    {"risk": "Three 'Still open' items in the #3332 PR body (accounting by-id, webhook headers, onboarding ResolvedTenant) are documented but have no test coverage and no issue — the merge locked in that gap. Without failing-on-main tests the next regression is silent.", "probability": "high", "impact": "high", "mitigation": "Capture as high-priority backlog vectors this run; the DoD gate (failing-on-main test in the fix PR) applies when the fix lands."},
    {"risk": "audit_logs ON DELETE SET NULL FK + append-only trigger is a design-level conflict (#3330 note). The seed --force path has an ACCESS EXCLUSIVE escape; every other delete path that could touch those FKs will 500 in prod with no test coverage.", "probability": "medium", "impact": "high", "mitigation": "Pm-tech-lead ADR drives test coverage; db-test suite_4 or _5 grows a regression once the design lands."},
    {"risk": "11 of the 21 PRs in this window are post-merge-reviewer follow-ups (gh-issue-3304..3315) — the post-merge-review cadence is working, but the volume indicates the first-pass review is under-covering tenant isolation. A future epic with less reviewer attention could ship with a comparable gap left unresolved.", "probability": "medium", "impact": "high", "mitigation": "Capture the common patterns from this follow-up wave (RLS-aware repo variants, require_manager_rls consolidation, SQL-level org isolation, manager-gate on execution lists) as explicit first-pass review checklist items in `.github/pull_request_template.md` or a pm-qa checklist doc."},
    {"risk": "Mobile RN Alert.alert hardcoded-English pattern (4 call sites) is the same regression class flagged in the 2026-08-25 rotation (3 PRs then: #2835/#2836/#2837). An i18next-hardcoded-strings lint rule still isn't in CI 73 days later — the rotation debt is high.", "probability": "high", "impact": "medium", "mitigation": "pm-frontend + pm-qa adopt i18next-hardcoded-strings as a blocking Biome rule on frontend/apps/mobile; migrate the 4 call sites in the same PR."},
    {"risk": "backend/crates/db/tests/suite_4.rs is the top churn hotspot this run (3 commits, 2 PRs — #3330 added the energy/alert_rules/seed schemas, #3332 the authz tests). High churn in a test file is healthy, but the file is approaching the 500-line threshold where it becomes hard to parse — split before it crosses 1000", "probability": "low", "impact": "low", "mitigation": "Split suite_4.rs into schema + authz + concurrency sub-modules when it crosses 800 LoC; currently acceptable."}
  ],
  "open_questions": [
    "Should the 'Still open' items from a merged PR's body auto-promote to backlog vectors? Today they fall off unless pm-qa / Scrum Master catches them.",
    "Is there a canonical 'failing-on-main' CI diff-guard proposal ready for a decision, or is it still an open DEC item from 2026-08-25?",
    "Does the required-checks consolidation (CODEOWNERS × route-path → authz suite) exist as infra today, or does it need to be built from scratch?"
  ],
  "decisions_needed": [
    "'Still open' convention — formalize as `pr-body-still-open` signal type OR require a GitHub issue filed alongside the PR OR require a deferred-test placeholder in the test file — owner: pm-tech-lead + pm-qa",
    "i18next-hardcoded-strings Biome rule — blocking on first adoption vs warn-only for one sprint? — owner: pm-frontend + pm-qa (re-raised from 2026-08-25)"
  ]
}
```

## Notes

- 73 days stale rotation — pm-qa slot had been idle since the atomic-claim + fuzz-sanitizer cycle (2026-08-25).
- The post-merge-reviewer follow-up wave (gh-issue-3304..3315) is a healthy signal that reviewer discipline is actually running; the volume (11 follow-ups in 24h) also indicates first-pass review is missing the same patterns repeatedly.
- No new blocker on CI this run (the 2026-10-28 #3187 cross-tenant-assign-404 blocker was cleared in a prior window).
