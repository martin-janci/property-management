# PPT Project State

_Generated: 2026-11-09 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (76-day stale slot; pm-cursor idx 3 → 4). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a OAuth re-checked, 3/3 stories still `done`, no PRs touched OAuth AS/token-mgmt surfaces in window). pm-scrum-master always-on._

## Delta since last routine run (2026-11-06T14:58:01Z → 2026-11-09)

- **7 PRs merged** — none flip coverage; all polish on already-`done` surfaces.
  - `#3350` reality-web share-cmp unchecked JSON guard
  - `#3351` ppt-web registry pet/vehicle forms i18n
  - `#3353` auth password-change session revocation (closes issue #3352)
  - `#3354` ppt-web registry auth context fix
  - `#3355` ppt-web registry page i18n
  - `#3358` screen-map drift reality realtor-detail (re-land of #3333)
  - `#3359` ppt-web registry mutation toast (re-land of #3356)
- **4 action-list rows resolved** (merged PRs): `code-review-reality-web-share-cmp-unchecked-json` (#3350), `code-review-ppt-web-ui-registry-no-i18n` (#3351), `code-review-ppt-web-ui-registry-page-i18n-gap` (#3355), `code-review-ppt-web-ui-registry-mutation-silent-fail` (#3359).
- **1 critical new issue: #3357** — `Dispatcher pipeline stalled: 3 approved PRs orphaned from dev; Tier-1d signals 2026-11-04..08 never ingested into action-list.json`. Buffer dropped to **claimable=6/72** before this run.
- **Buffer refill this run**: ingested 34 unqueued tier-1d signals (2026-11-04..08, backend/web only — mobile-native-kmp excluded per #2652 cloud-build block) + 4 pm-qa + 1 pm-scrum-master next-action rows ⇒ **action-list `open` 9 → 47** (≥36 target hit with 11-row cushion).

## Executive summary

- **Delivery: 49/49 stories done across 13 epics** in coverage. No status flips this run; `last_checked` refreshed on epic-10a (idx 0 → 1 — OAuth AS, client registration, token management — nothing in window touched those surfaces; `#3353` was session revocation, session layer not OAuth client/token).
- **Ops incident of the week — #3357**: tier-1d signal ingestion stall. 3 approved PRs orphaned from dev; the dispatcher consumed the buffer to 6 rows while 19+ fresh backend/web signals sat unqueued. This routine run is the manual patch; the durable fix (self-test T26 + dispatcher ingestion watchdog) is now queued as two high-priority rows.
- **pm-qa rotation came up 76 days stale** — last run 2026-08-25. Seven merged PRs landed with **0 added CI assertions** against the three recurring defect classes (raw-DB-error-leak, xorg-authz, i18n-leak); four pm-qa test-infra rows are now queued to lay down invariants BEFORE the implementer fix rows claim.
- **Backend CI on dev still red** on `#3187` (cross_org_assign_violation_is_not_found — role-gate 403 vs expected 404). Release-blocker if a cut happened. Carried.
- **Still blocked**: `#3357` (ops), `#2652` (KMP cloud builds), `#3187` (dev backend CI), `#3168` (cargo-deny RUSTSEC carried), `#2951` (RN jest-expo).

## What's next (top 5)

1. **[high · pm-tech-lead]** Fix dispatcher tier-1d signal ingestion (#3357) — re-ingest 2026-11-04..08 signals (done by this run), identify & land the 3 orphaned approved PRs, add watchdog self-test T26.
2. **[high · pm-security]** Close `code-review-api-handlers-delegation-xorg-delegate` + `code-review-api-handlers-unit-residents-add-xorg-user` WITH attached cross-org regression fixtures (prevents the 2026-06 PR #497 "fix without test" pattern).
3. **[high · pm-qa]** Pre-attach cross-org regression test fixtures to the two xorg security rows (`pm-qa-tier1d-security-no-regression-tests-2026-11-09`) so the implementer PRs ship with the test, not after it.
4. **[high · pm-backend]** Close `code-review-api-core-accounting-nonatomic-statement-ingest` (wrap bank-statement upload in sqlx transaction — data-integrity finding on financial ledger) and `code-review-api-core-accounting-matcher-ignores-currency` (gate amount corroboration on currency).
5. **[high · pm-security]** Resolve `#3187` cross_org_assign 403-vs-404 — pick between handler change (RLS-first) vs test change (role-gate-first); unblocks dev backend CI.

## Role focus today: `pm-qa` + `pm-scrum-master`

### pm-qa (one-line summary)

Test-shadow discipline is drifting again. 7 merges landed with 0 added CI assertions against the recurring classes (raw-DB-error-leak, xorg-authz, i18n-leak). 4 test-infra rows queued (high/medium) to lay down invariants BEFORE the implementers claim the individual fix rows. Full report: `.research/management/roles/pm-qa.md`.

### pm-scrum-master (one-line summary)

Buffer-low week driven by #3357 dispatcher ingestion stall, not a real work drought. Refilled 38 rows this run; added self-test + ingestion-sweep actions so the stall is detected next time. Full report: `.research/management/roles/pm-scrum-master.md`.

## Blockers

| Item | Why | Owner |
|---|---|---|
| `gh-issue-3357` | Dispatcher pipeline stall: tier-1d signals not ingested; 3 approved PRs orphaned | pm-tech-lead |
| `gh-issue-2652` | KMP cloud builds structurally blocked; 5+ mobile-native action-list rows parked unclaimable | pm-devops |
| `gh-issue-3187` | Backend CI on dev: cross_org_assign_violation_is_not_found (role-gate 403 vs expected 404) | pm-security |
| `gh-issue-3168` | cargo-deny RUSTSEC-2026-0368/0369 (zip 8.6.0 via rust_xlsxwriter) — merge gate | pm-devops |
| `gh-issue-2951` | React Native jest-expo environment broken — mobile test runs unreliable | pm-frontend |

## Cursors

- `state.pm_cursor`: next_index **3 → 4** (next: `pm-devops`). `role_last_run[pm-qa] = 2026-11-09`.
- `state.coverage_cursor`: next_index **0 → 1** (next re-check: `epic-10b` Platform Administration, 7 stories).
