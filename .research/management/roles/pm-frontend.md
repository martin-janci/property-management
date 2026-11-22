# pm-frontend — 2026-11-22

_Phase 1.6 — static, read-only. Cloud routine run._

## Summary

Frontend delivery caught up hard this run: 12 of the 26 merged PRs are frontend/mobile — including the 25-day-stale carried ppt-web logout cache-leak (#3706), the WS token-in-URL leak (#3696), two accounting-invoice UX fixes (#3697 wire format + empty-items, #3703 detail), two i18n burn-downs (#3699 WidgetSettings, #3705 list/search), and three empty-state wires (#3700 Create Lease pickers, #3707 bookfacility availability, #3695 /report attachments). But the 2026-12-01 mobile i18n lint-flip (warn→error) is 9 days out with 10+ open `code-review-*-i18n-retry1` items plus 2 open drafts (#3676/#3677) — the burn-down is on-track but not safe.

## Next actions

| Priority | Action | Dependency | Definition of done |
|---|---|---|---|
| high | Burn down the frontend-i18n slice (10+ `code-review-*-i18n-retry1` items + drafts #3676/#3677) before 2026-12-01 CI gate flip | none | `pnpm --filter mobile lint --no-warn` and reality-web / ppt-web lints exit 0 on dev for 48h before the flip |
| high | Promote drafts #3714 (ppt-web date/number locale formatters) and #3715 (ppt-web native confirm dialogs) to ready-for-review within 48h — both are from the 2026-11-19/20 action-list and are the only non-i18n frontend drafts in flight | none | both PRs non-draft, reviewed, merged |
| high | Verify the `AUTHED_QUERY_KEY_ROOTS` logout-purge pattern from #3706 is comprehensive — audit every feature-group root query key, add a unit test that asserts the allow-list matches `useQuery({queryKey: [X, ...]})` call sites | none | unit test exists; root-key allow-list = root-key call-site set |
| medium | Close out the carried 'wire-the-button-but-not-the-POST' pattern: audit `ReportFaultScreen.tsx` (RN), reality-web `/sell` wizard Publish, and `VoteDetailPage` ranked-ballot submit — the 2026-10-28 run flagged 3; #3700 and #3707 are the pattern but only for ppt-web empty-states | pm-qa | MSW/Mirage e2e-smoke tests exist for each named form; CI asserts the POST payload |
| medium | Characterize the two high-churn frontend files this week (AccountingInvoiceDetail.tsx, reality-web report/page.tsx, leases.tsx) — if each is >800 lines and touched by >3 different PRs in-window, flag as a watch-* module for the hardening sprint | pm-tech-lead | either a split plan exists or the file is marked sized-OK |
| medium | Confirm the WS token-in-URL strip from #3696 covers every WebSocket opener on ppt-web (not just notifications) — search for `new WebSocket(` call sites and assert no `?token=` | pm-security | grep for `new WebSocket(` returns no URL carrying a bearer token |

## Risks

| Prob | Impact | Risk | Mitigation |
|---|---|---|---|
| high | high | 2026-12-01 mobile i18n lint flip with 10+ open `code-review-*-i18n-retry1` items + 2 open drafts — a slip breaks CI for everyone on day 1 | Daily burn-down count on the i18n action-list slice; promote drafts; 48h pre-flip gate on dev |
| medium | high | The 'wire-the-button-but-not-the-POST' pattern remains undetected — three instances called out on 2026-10-28 are still unverified (ReportFault setTimeout, /sell wizard Publish, VoteDetail ranked ballot); #3700 + #3707 show the pattern keeps recurring | Per-screen MSW/Mirage e2e-smoke: assert the POST payload for every form with a Submit button; block merge otherwise |
| medium | medium | AccountingInvoiceDetail.tsx + leases.tsx are both ppt-web churn hotspots this run (3 PRs each touched them in window) — a single high-churn file crossing a multi-PR merge gate is where regressions live | Tag the two files in the `modules-under-watch` sprint track if either exceeds 800 LOC; otherwise set a `.codeowners`-style reviewer rotation |
| low | high | Admin-web (#3710 landed console contract) is under-tested relative to its risk — it's the ops console; a contract drift reaches real managers immediately | Add admin-web to the per-PR reviewer rotation; require contract-test coverage on every admin endpoint |

## Open questions

- Does the AUTHED_QUERY_KEY_ROOTS allow-list from #3706 ship with a unit test, or only runtime coverage?
- For the 2026-12-01 i18n flip — is there an auto-generated list of hardcoded-string offenders we can diff against `last_run` to prove burn-down progress?
- Are the three 2026-10-28 'wire-the-button' findings (ReportFault/sell-wizard/VoteDetail) in action-list.json today, or were they dropped when the rotation went 83 days stale?

## Decisions needed

- Whether to defer the 2026-12-01 mobile i18n lint flip by one sprint if burn-down is incomplete on 2026-11-29 — owner: pm-frontend + pm-qa
- Whether every form Submit handler needs an MSW e2e-smoke assertion before merge (carried from 2026-10-28, still unresolved) — owner: pm-qa + pm-tech-lead
