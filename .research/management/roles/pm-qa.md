# pm-qa — QA / Test lens (2026-06-15)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open issues._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "Dev CI unblocked by #1379 ends a 3-day red streak; 18 fresh follow-up issues from yesterday's merge surge are the new QA front, concentrated on missing test coverage (RLS, IDOR, OAuth, realtime sync, payment atomicity).",
  "next_actions": [
    {"action": "Add regression test for record_payment non-atomic check-then-insert (#1361) — concurrent double-pay scenario", "priority": "high", "dependency": "rust-backend", "definition_of_done": "Failing-on-main test landed; passes once #1361 fix lands."},
    {"action": "Audit allowed_pet_types enum decode paths + add unit test for unknown variants (#1363, #1366)", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "Decoder tests pin known variants and error-path for unknown."},
    {"action": "Add iOS UI test for SearchView stale-response guard preserving pagination (#1365)", "priority": "medium", "dependency": "pm-frontend", "definition_of_done": "UI test confirms older paginated response doesn't clobber newer page-N state."},
    {"action": "Add dispute draft auto-save tests — i18n key presence + re-render race (#1360, #1364)", "priority": "medium", "dependency": "pm-frontend", "definition_of_done": "Frontend test asserts autosave fires once per debounce window; i18n key lookups present."},
    {"action": "Add concurrency test for record_reserve_transaction atomicity + COALESCE on budget aggregates (#1371)", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "Concurrent-tx test asserts no negative-balance and COALESCE-guarded aggregates."},
    {"action": "Pin cron validator surface with fixture-based test (#1368) to guard against #616 regression", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "Validator test fixture covers known-good + known-bad expressions; Epic 81 promotion gates on it."}
  ],
  "risks": [
    {"risk": "18 follow-up issues (#1360–#1377) from the post-merge review of 2026-06-14 merges remain untriaged; backlog grows faster than burn-down without owner assignment.", "probability": "high", "impact": "medium", "mitigation": "pm-scrum-master triages the batch this run — assign owner or close as won't-fix."},
    {"risk": "record_payment handler does a check-then-insert without serializable isolation or unique-constraint guard (#1361); concurrent retries can double-write a payment.", "probability": "medium", "impact": "high", "mitigation": "Wrap in serializable tx OR add (idempotency_key, payment_id) unique constraint; add concurrency regression test."},
    {"risk": "Cron validator drift (#1368) could silently reintroduce regression #616 (Epic 81 promotion blocker) — current tests don't pin the validator surface.", "probability": "medium", "impact": "medium", "mitigation": "Pin a fixture-based test for the cron validator; gate Epic 81 promotion on it."},
    {"risk": "Dispatcher meta-issue #1380: stale gap-scan buffer feeds no-op claims + Tier-2 escalation endpoint misconfigured — wastes implementer cycles claiming gap stories already shipped.", "probability": "high", "impact": "medium", "mitigation": "pm-devops or dispatcher owner refreshes gap-scan buffer; verify Tier-2 endpoint config in dispatcher settings."},
    {"risk": "Booking.com OAuth/credential connect flow lacks secure replacement on re-connect (#1362, #1374) — old credentials can linger and be used post-rotation.", "probability": "medium", "impact": "high", "mitigation": "Implement atomic credential swap + add OAuth handler/CSRF test coverage."}
  ],
  "open_questions": [
    "Does #1377 (document download/preview test gap) require new presigned-URL minting test infra, or can it be folded into the existing forms RLS suite?",
    "Should the pre-push fmt/clippy gate (#1375) be local-only or also enforced as a CI status check?",
    "Is the realtime preference-sync publish leg (#1376) coverable as a deterministic integration test, or does it need a flake-tolerant smoke test?",
    "Are stale draft PRs #1316 (verify-document-folder-organization-backend-promote) and #1197 (test-oauth-authorization-server-integration) salvageable, or should they be closed and re-opened?"
  ],
  "decisions_needed": [
    "Pre-push fmt/clippy gate (#1375): local hook only, CI-status, or both? — owner: pm-tech-lead",
    "Triage protocol for the 18 follow-up issues #1360-#1377: bulk-assign by theme to per-role queues, or per-issue triage? — owner: pm-scrum-master",
    "Promotion-gate policy: should each high-severity coverage gap (atomicity, IDOR, RLS) block its source epic's done-promotion until a failing-on-main test exists? — owner: pm-tech-lead + pm-qa"
  ]
}
```

## Notes

- Rotation idx 3 of 8; next pm-qa run ~ 2026-07-06 (assuming 1-per-day cadence with 8 roles).
- Five new pm-qa next_actions appended to `action-list.json` with `source = "pm-analysis 2026-06-15"`.
- Five risks dedup-checked against existing pm-qa risk IDs and appended.
- Coverage epic-85 (rotation idx 10) refreshed: gap-85-2 evidence added from PR #1383; "app icon variants" + "app.config.ts" gaps removed.
- Phase 1.5 finding (vote partial_cmp NaN) already tracked in prior run via `pm-qa-vote-partial-cmp-nan-fuzz` — kept open.
- Phase 1.5 finding (`OsRng.try_fill_bytes().expect()` low-sev in crypto.rs) noted; below threshold for new action this run.
