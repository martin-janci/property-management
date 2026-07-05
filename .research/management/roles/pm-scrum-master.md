# pm-scrum-master — 2026-07-05

_Ran alongside the daily rotating role this run (Phase 1.6, re-fired by buffer-low signal). Static read; no compile/run._

## Summary

This run shipped 7 merged PRs (test-hardening/security follow-ups + BIT-414) and closed 8 dispatcher-follow-up issues, but the action-list buffer sits at 16/36 non-terminal items and epic-level status in `sprint-status.yaml` is stale against both story detail and `coverage.json` (notably epic-10a gated on possibly-stale test-hardening issues despite coverage-confirmed completion).

## sprint_progress

- Sprint: Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- Epics done: 1 / 6

## shipped_since_last_run

- #2094 test(db): reuse shared seed_org in favorite_alert_read
- #2095 refactor(api-server): assign_fault recipient guards
- #2096 chore(api-server): quick-xml XXE via cargo-deny
- #2097 test(messaging): un-quarantine #1771 soft-delete unread
- #2098 test(db): enum sync guard for currency/country
- #2099 refactor(api-server): SlovakAccountingExport honesty by construction
- #1979 test(api-server): outages happy-path — mint raw manager JWT, BIT-414
- 8 issues closed (#2078–#2087, dispatcher follow-ups)

## next_actions

- **[high]** Fix `document_folder_tests` CI red and flip `7a-2-folder-organization` off "review" (no action-list entry currently tracks this — foundational to closing epic-7a) — owner: pm-backend
- **[high]** Reconcile epic-10a sprint-status (10a-1/2/3 stuck at `ready-for-dev`) against `coverage.json` evidence that all three are done via merged PRs; confirm whether gate issues #481/#482/#487 are still open on GitHub — owner: pm-scrum-master (dep: pm-security)
- **[high]** Triage stalled draft PRs #1812 (reality_portal repo split, 11d) and #1797 (backend-authz-ocr-and-rental-pii, 12d — security-sensitive) — owner: pm-tech-lead / pm-security
- **[medium]** Close residual `7a-5-document-sharing` gap: web share-panel still uses `window.confirm` + no UUID validation on user-ID input (#485 residual, IDOR-adjacent) — owner: pm-frontend
- **[medium]** Address `8a-3-notification-preference-sync` residual gaps: `dispatch_to_users` is serial and FCM stub silently swallows failures (falsifies sent counts) — owner: pm-backend
- **[medium]** Seed action-list with remaining coverage gaps to close the 16→36 buffer (81-1 cron-column, 82-5 mobile UI, 85-1 env-setup docs) — owner: pm-tech-lead. **Done this run** (20 gap-candidates + 12 role next_actions added; buffer now at 40 open).

## blockers

- **7a-2-folder-organization (epic-7a):** CI red on `document_folder_tests` since PR #1316 round 1; reverted from done to review; no action-list item currently tracks it — owner: pm-backend
- **10a-1/10a-2/10a-3 (epic-10a):** sprint-status test-hardening gate (issues #481/#482/#487) holds stories at `ready-for-dev` despite `coverage.json` marking all three done via merged PRs — owner: pm-security
- **PR #1797 (`fix/backend-authz-ocr-and-rental-pii`):** Draft 12 days, touches authz/PII surfaces, unmerged and unresolved — owner: pm-security
- **PR #1812 (reality_portal repo split):** Draft 11 days, structural change with no visible reviewer progress — owner: pm-tech-lead

## risks

- **[high/medium]** Action-list buffer at 16/72 claimable (target 36) risks the dispatcher idling or re-picking low-value churn items instead of real coverage gaps.
- **[medium/high]** PR #1797 has sat in draft 12 days on high-sensitivity surfaces.
- **[high/medium]** `sprint-status.yaml` epic-level status rollups are stale.
- **[medium/high]** Test-hardening gating issue #481 (OAuth refresh-token bypass) — live if genuinely open.
- **[medium/medium]** `7a-2-folder-organization` has zero action-list coverage despite being CI-red.

## open_questions

- Are test-hardening gate issues #481/#482/#487 still open on GitHub, or superseded by the `coverage.json` "done" classification of 10a-1/10a-2/10a-3?
- Is `document_folder_tests` still red on `dev` for `7a-2-folder-organization`, or was it fixed out-of-band with no PR/action-list trace?
- What is the current review/ownership status of stalled draft PRs #1812 and #1797 — actively worked, abandoned, or blocked on rebase?
- Should `sprint-status.yaml`'s epic-level `status:` field be auto-derived from story `development_status` going forward, or is manual-only by design?
- Should "8a-3 mobile OS push" and "82-5 mobile inquiry thread/account UI" be split into their own tracked stories rather than lingering as gap notes on stories already marked done?

## decisions_needed

- Reconcile epic-level status rollups in `sprint-status.yaml` to match story-level done counts — owner: pm-scrum-master (with pm-tech-lead)
- Confirm/close or explicitly defer test-hardening gate issues #481/#482/#487 given `coverage.json` evidence — owner: pm-security
- Disposition of stalled draft PRs #1812 and #1797 (continue, reassign, or close) — owner: pm-tech-lead
- Whether to formally split deferred mobile gaps (8a-3 OS push, 82-5 inquiry thread/account UI) into new backlog stories vs keep as inline gap notes — owner: pm-scrum-master
