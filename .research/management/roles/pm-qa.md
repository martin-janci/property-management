# pm-qa — QA / Test lens (2026-11-09)

_Rotation idx 3 of 8. Previous run: 2026-08-25 (76 days stale). Read-only static analysis of sprint-status + merged PRs + fresh tier-1d signals + open issues._

## Window at a glance

- **7 PRs merged** 2026-11-06..09 (see Scrum Master). QA-adjacent lens:
  - **#3350 reality-web share-cmp guard** — merged with a Vitest fixture that stubs a malformed JSON payload; **good test shadow** (contract test shape worth replicating for other unchecked JSON parse sites).
  - **#3351/#3355/#3359 registry triad** — all three closed i18n + silent-mutation code-review rows; **test gap**: none of the three added a Biome/vitest guard against untranslated literals; the next feature folder will re-introduce the gap.
  - **#3353 password-change session revocation** — closes #3352 but; **test gap check**: need to confirm the regression test exercises the revocation-on-password-change path with an active parallel session (classic refresh-token-after-pw-change race).
  - **#3354 registry auth context fix** — pure wiring; no test added.
  - **#3358 screen-map drift re-land** — docs-only; n/a.
- **4 new xorg / isolation findings** from 2026-11-08 tier-1d (delegation, unit residents, signatures DB-error leak) — all follow the 2026-06 inquiry mark_as_read IDOR class; **none shipped with a test fixture yet** (per tier-1d signal file).

## Release-readiness view

- No release cut in this window; dev branch holds all 7 merges. CI blocker `gh-issue-3187` (cross_org_assign_violation_is_not_found) STILL present — the epic-10b violations surface is "done" in coverage but a test asserting 404 vs the implementation's 403 is red on main backend CI. **Release blocker if a release were cut today.**

## Test-shadow gap tally (this run)

| Finding / fix | Test gap | Consequence |
|---|---|---|
| `delegation-xorg-delegate` (2026-11-08) | No cross-org fixture added yet | Re-regression risk; same shape as 2026-06 #497 class |
| `unit-residents-add-xorg-user` (2026-11-08) | No cross-org fixture added yet | Same |
| `listings-rs-raw-db-error-leak` (2026-11-05, 36 call sites) | No contract test on error-body shape | Pattern re-grew across rentals/signatures/reality-server |
| `ppt-web-ui-portfolio-perf-i18n` + `-forms-feature-i18n` | No vitest/biome golden test on untranslated literals | Next feature folder will re-ship untranslated |
| `#3187` cross_org_assign_violation_is_not_found | Test asserts 404; prod returns 403 | Backend CI red on dev |

## Role JSON

```json
{
  "role": "pm-qa",
  "summary": "Test-shadow discipline is drifting again: 7 merges landed with 0 added CI assertions that would prevent the recurring classes (raw-DB-error-leak, xorg-authz, i18n-leak). Four new test-infra gaps are queued (high/medium) to lay invariants down BEFORE the implementers land the individual fixes. Backend CI on dev is still red on #3187 (release-blocker if a cut happened).",
  "next_actions": [
    {"action": "Add cross-org regression test fixture for delegation + unit_residents xorg findings BEFORE their fix PRs claim (same pattern as 2026-10-26 violations xorg sweep, PR #3183)", "priority": "high", "dependency": "none", "definition_of_done": "two test files added under crates/db/tests or backend/servers/api-server/tests/suites exercising org-B-manager-token-on-org-A-seed; both assert 403/404"},
    {"action": "Add dispatcher self-test T26 asserting every open tier-1d signal file maps to a backlog row OR action-list row (watchdog for #3357 root cause)", "priority": "high", "dependency": "gh-issue-3357", "definition_of_done": ".research/dispatcher-self-test.sh T26 passes on the current repo; CI job enforces on PRs touching .research/signals/"},
    {"action": "Add contract test on error-body shape that fails if any handler interpolates raw sqlx::Error Display (regex on response body across integration tests)", "priority": "medium", "dependency": "none", "definition_of_done": "test exists under backend/servers/api-server/tests/suites; today's 6 open findings resolve the test by fixing the handler, not by excluding it"},
    {"action": "Add frontend golden test (biome rule + vitest) that fails on untranslated user-facing string literals in features/* folders (portfolio-performance, forms, registry-rules-form)", "priority": "medium", "dependency": "none", "definition_of_done": "rule in frontend/apps/ppt-web/biome.json + test under frontend/packages/i18n or similar; fails on the three listed folders until swept"},
    {"action": "Fix gh-issue-3187: decide between (a) backend assign_violation returning 404 cross-tenant to match the test, or (b) relaxing test to accept 403 when role-gate fires first. #3183 regression suite expects 404 — the 403 is probably the real bug.", "priority": "high", "dependency": "pm-security", "definition_of_done": "backend CI on dev goes green"},
    {"action": "Audit #3353 password-change session revocation PR for a parallel-session regression test (refresh-token-after-pw-change race)", "priority": "low", "dependency": "none", "definition_of_done": "test exists; if absent, open a follow-up row"}
  ],
  "risks": [
    {"risk": "Raw sqlx::Error Display leak is a recurring CLASS (6 open findings this run) — one-off handler fixes won't stop it without a contract test", "probability": "high", "impact": "medium", "mitigation": "Ship the pm-qa-raw-sqlx-error-leak-contract-test BEFORE individual implementer rows claim"},
    {"risk": "xorg authz findings tend to ship fix WITHOUT regression test (PR #497 pattern) — the next PR will too unless the fixture is pre-attached", "probability": "high", "impact": "high", "mitigation": "Pre-attach cross-org fixtures to pm-security candidate rows before dispatcher claim"},
    {"risk": "Tier-1d signal ingest stall (#3357) means test-shadow findings themselves can go missing — QA follow-ups vanish along with implementer work", "probability": "high", "impact": "high", "mitigation": "Self-test T26 above"},
    {"risk": "i18n-leak class recurring across features/* folders — manual review is leaking it through", "probability": "high", "impact": "low", "mitigation": "Biome/vitest golden test above"},
    {"risk": "#3187 unresolved for >2 weeks (surfaced 2026-10-26) — backend CI on dev is red; any accidental release cut is blocked", "probability": "medium", "impact": "high", "mitigation": "Force a decision between test adjustment vs handler fix THIS SPRINT"}
  ],
  "open_questions": [
    "Does the password-change revocation PR (#3353) exercise the parallel-session race, or only the single-session happy path?",
    "Is the cross_org_assign_violation 404-vs-403 contract (#3187) reconciled in epics catalog, or is it a test-only disagreement?"
  ],
  "decisions_needed": [
    "Error-body shape: adopt a single ErrorResponse envelope across all backend handlers (per listings-rs-unstructured-error-body) — owner: pm-tech-lead",
    "Cross-tenant contract: 403 (role-gate first) vs 404 (RLS first) — must pick one and enforce in a shared test macro — owner: pm-security"
  ]
}
```

## Notes for the dispatcher

The 4 `pm-qa-*-2026-11-09` action-list rows are the carrier for the infra-level test gates; they SHOULD claim first in the pm-qa rotation window because they unlock the fix rows (which otherwise ship without their regression test, per the pattern above).
