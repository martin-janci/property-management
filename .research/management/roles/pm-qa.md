# pm-qa — QA / Test lens (2026-08-25)

_Rotation idx 3 of 8. Read-only static analysis of sprint-status + merged PRs + open issues. Previous run: 2026-06-15 (71 days stale)._

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "13 PRs merged in this window fixed real defects but 7 of them lack the test that would guard against re-regression — the auto-fix loop is now producing enough follow-up-issue-and-close activity that a QA test-shadow discipline is the next lever. The atomic-claim concurrency test (#2834 / #2831 closure) and the booking-connect authz test (#2821) are the two high-priority gaps this run.",
  "next_actions": [
    {"action": "Add >1-replica concurrency integration test for quiet-hours drain atomic claim (#2834 / closed #2831) — assert at-most-once delivery under 2 racing api-server replicas contending on the same held-notification batch", "priority": "high", "dependency": "rust-backend", "definition_of_done": "Integration test spins up 2 api-server replicas against shared Postgres + Redis, both drain the same held-notification set, assertion: exactly-one send per (user, channel, notification_id)."},
    {"action": "Add authz regression test for direct-connect OTA credential writes (#2821) — assert non-manager role is rejected at the booking-connect endpoint (was hijackable before this window)", "priority": "high", "dependency": "rust-backend", "definition_of_done": "Test attempts direct-connect credential write as (tenant, staff, admin-of-other-org) roles; asserts 403; asserts manager succeeds."},
    {"action": "Add regression test for VoteDetailScreen conditional-hooks fix (#2835) — snapshot render with the empty/loading branch that triggered the rules-of-hooks violation", "priority": "medium", "dependency": "pm-frontend", "definition_of_done": "Jest-expo test renders VoteDetailScreen in the loading-then-loaded transition; asserts no React warning; snapshot stable."},
    {"action": "Add fuzz/property test for CSV export sanitizer (#2827 / closed #2822) — CR × LF × CRLF × formula-injection prefixes × every export column type", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "proptest crossing {CR, LF, CRLF, NUL, U+2028, U+2029} × {=, +, -, @, tab} × column-type; asserts no raw newline, no formula prefix escape, header preserved."},
    {"action": "Add unit + integration test for voice OAuth token encryption round-trip after centralization (#2838) — cover Alexa + Google + fallback provider; assert no plaintext at rest", "priority": "medium", "dependency": "rust-backend", "definition_of_done": "Test encrypts token → writes DB row → reads back → decrypts → equality; additional assertion: DB byte-inspection does not contain plaintext token substring."},
    {"action": "Add ppt-web test asserting AML EDD/Review dialog state resets per assessment (#2833 / closed #2832) — open dialog on assessment A, cancel, open on assessment B, assert reason/notes cleared", "priority": "medium", "dependency": "pm-frontend", "definition_of_done": "Vitest render test opens dialog with assessment A, types reason/notes, cancels, opens with assessment B; asserts inputs empty."}
  ],
  "risks": [
    {"risk": "3 of this window's mobile-rn PRs (#2835 conditional hooks / #2836 hardcoded en / #2837 hardcoded en) fixed defects that eslint-plugin-react-hooks + a no-hardcoded-strings rule would catch statically — every future mobile-rn PR risks reintroducing them", "probability": "high", "impact": "medium", "mitigation": "Adopt eslint-plugin-react-hooks + i18next-hardcoded-strings ESLint config in frontend/apps/mobile; gate CI on it."},
    {"risk": "PR #2834 (atomic claim for quiet-hours drain) closes #2831 but no test exercises the >1 api-server-replica race — the exact scenario the fix was written for is unverified", "probability": "medium", "impact": "high", "mitigation": "Add integration test with 2 racing api-server replicas contending on the same held-notification batch; assert at-most-once delivery."},
    {"risk": "PR #2838 centralized voice OAuth token encryption but there is no round-trip test asserting no plaintext at rest — a future field-rename or key-derivation change could silently store plaintext", "probability": "medium", "impact": "high", "mitigation": "Add unit + integration test covering encrypt(token) → DB row → decrypt(row) → token equality across Alexa/Google/fallback; assert stored bytes are not the plaintext token."},
    {"risk": "PR #2827 neutralizes CR/LF in the CSV export sanitizer but the test suite likely only covers the reported inputs — formula-injection prefixes (= + - @) combined with CR/LF are the classic bypass class and remain unfuzzed", "probability": "medium", "impact": "medium", "mitigation": "Add a property/fuzz test crossing {CR, LF, CRLF, NUL, U+2028, U+2029} × {=, +, -, @, tab} × every export column type."},
    {"risk": "Auto-fix loop closed 7/7 in-window follow-ups but the same-window close creates the illusion the fixes are proven — 5 of the 7 shipped without a regression test. The loop's compensating-transaction property masks a growing test-shadow debt.", "probability": "high", "impact": "medium", "mitigation": "Adopt a policy: any dispatcher-spawned fix PR requires a failing-on-main test file in the same PR before it can enter the merge queue; enforce via CI diff-guard."}
  ],
  "open_questions": [
    "Should the mobile-rn ESLint gate (react-hooks + no-hardcoded-strings) be blocking on first adoption or warn-only for one sprint to let the existing violations be fixed batch-wise?",
    "Is there an existing PPT test harness for spinning up 2 api-server replicas against shared Postgres+Redis, or does the atomic-claim concurrency test need new fixture infra?",
    "Do we have canonical fuzz-corpus discipline for sanitizers (CSV, XSS, HTML), or is each sanitizer testing its own inputs today?",
    "The 'test-shadow' pattern (fix PR without regression test) — is the pm-qa Definition-of-Done gate advisory or blocking? DEC-001-style formalization needed?"
  ],
  "decisions_needed": [
    "Should any dispatcher-spawned fix PR require a failing-on-main test file in the same PR before it can enter the merge queue (enforced via CI diff-guard)? — owner: pm-tech-lead + pm-qa",
    "Frontend mobile lint gate rollout — blocking on first adoption vs warn-only for one sprint? — owner: pm-frontend + pm-qa",
    "Fuzz-corpus discipline for shared sanitizers (CSV, XSS, HTML, JSON) — do we ship a shared proptest corpus crate under backend/crates/? — owner: pm-tech-lead"
  ]
}
```

## Notes

- Rotation idx 3 of 8; next pm-qa run ~ 2026-09-25 (assuming ~30-day-per-role cadence).
- Six pm-qa next_actions appended to `action-list.json` with `source = "pm-qa 2026-08-25"`; four risks and one carried risk-flavor appended to `risks.json`.
- Coverage epic-80 (rotation idx 5) re-checked — all 3 stories still `done`; last_checked stamped 2026-08-25. No PR in this window touched dispute routes.
- Merged-PR observations that shaped this run:
  - #2835 fixed a React `rules-of-hooks` violation in mobile-rn VoteDetailScreen — a `eslint-plugin-react-hooks` lint gate on frontend/apps/mobile would have caught this before code review.
  - #2836 / #2837 shipped hardcoded English strings in mobile-rn — same lint story, different rule (`i18next/no-literal-string`).
  - #2834 atomic-claim fix for quiet-hours drain — closes #2831 but is the exact scenario a >1-replica concurrency test should own.
  - #2838 centralized voice OAuth token encryption — should ship with a round-trip test as a matter of hygiene.
- Auto-review loop closed 7/7 in-window; net test-shadow debt grew by ~5 fixes-without-tests (raised as pm-qa risk this run).
