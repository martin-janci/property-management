# Project State — PPT (Property Management)

<sub>Generated: 2026-11-24T04:19:35Z — Scrum Master synthesis + pm-devops (rotation index 4 → 5). Coverage cursor advanced idx 10 → 11 (epic-85 re-checked, next epic-8a).</sub>

## Executive summary

**30 PRs merged in-window (#3739–#3768) — highest single-day cadence so far — dominated by the i18n warn→error flip burn-down.** 11 locale PRs landed in one day: reality-web (#3739/#3740/#3741), ppt-web (#3742 Registry Rules, #3745 Buildings, #3747 Reports, #3748 FileDispute, #3754 EditFault, #3755 Registry SPA nav, #3756 GuestReport, #3757 forms). Security micro-batch (5): #3744 SSO O(n) → amortized eviction (closes #3738), #3750 CGNAT in URL validator, #3761 feed-sub URL SSRF, #3758 in-proc ratelimit note, #3760 voice-webhook RLS release consistency. Push + mobile (6): #3746 cleared three test-shadow debts (2xx-FCM-delivered + worker-requeue-drop + push-db-err-leak) in one PR; #3765 RN useApi 401 refresh+retry; #3767 AI chat delete error surfaced; #3766 moderation type narrowing; #3768 AI workflows limit clamp; #3764 reality-web journal newsletter stop-faking. Admin (3): #3763 capability-holders endpoint + admin-web, #3762 payment-match tiebreak + admin-web capability gates, #3751 pm-qa no-test-diff audit landed. Churn (3): #3753 invoice-remaining cents, #3752 reality screen-map reconcile, #3743 OrgProvider memo retry. **Only 7 PRs remain open — 4 are new hotspot work started last night (#3769–#3772).** i18n flip is now **T-7d (2026-12-01)**; only 2 mobile-RN drafts (#3676/#3677) still gate it, both ≥22h stale.

## Sprint progress

- **Sprint:** Hardening & Refactor — Modules Under Watch, release-248 Quarantine Burn-down, Mobile-native Backlog
- **Workstreams:** 0 of 3 done (all three tracks `in-progress` / `blocked`).
- **Delivery-sprint rollup (historic, closed 2026-11-21):** 6/6 epics `done`.
- **Coverage (upkeep this run):** 49/49 stories done; rotating cursor re-checked **epic-85** (2/2 mobile env-config + build-config stories still `done`; `last_checked` advanced to 2026-11-24; no regression).

## Shipped since last run (2026-11-23T04:19:35Z → 2026-11-24T04:19:35Z)

- #3739 — test(reality-web): guard realtor/[id] i18n coverage across all locales
- #3740 — fix(reality-web): localize realtor analytics metric formatting
- #3741 — test(reality-web): pages.listingEdit cross-locale i18n parity guard
- #3742 — fix(ppt-web): route Registry Rules page + form through i18n
- #3743 — code-review-ppt-web-core-orgprovider-unmemoized-retry1
- #3744 — gh-issue-3738: O(n) amortized SSO pending-session capacity eviction
- #3745 — code-review-ppt-web-ui-buildings-pages-no-i18n
- #3746 — pm-qa-test-shadow-debt-fcm-worker-push (3-in-1 regression tests)
- #3747 — code-review-ppt-web-ui-reports-page-no-i18n
- #3748 — code-review-ppt-web-ui-file-dispute-page-no-i18n
- #3749 — code-review-ppt-web-ui-mediation-notes-swallowed-load-error
- #3750 — code-review-reality-server-url-validator-cgnat-gap
- #3751 — pm-qa-no-test-diff-sweep-rate-2026-11-23
- #3752 — screen-map-drift-pr-3695-reality
- #3753 — churn-hotspot-accounting-invoice-detail (integer cents)
- #3754 — code-review-ppt-web-ui-editfaultpage-no-i18n-retry1
- #3755 — code-review-ppt-web-ui-registry-fullpage-nav (SPA router)
- #3756 — code-review-ppt-web-ui-guest-report-submission-no-i18n
- #3757 — code-review-ppt-web-ui-forms-feature-no-i18n
- #3758 — code-review-api-handlers-inproc-ratelimit-not-distributed
- #3759 — code-review-ppt-web-core-authctx-no-token-expiry-check
- #3760 — code-review-api-handlers-voice-webhook-rls-release-inconsistent
- #3761 — code-review-reality-server-imports-feed-url-ssrf
- #3762 — fix: payment-match date tiebreak + admin-web capability gates
- #3763 — feat(api-server): capability holders endpoint + admin-web View holders
- #3764 — code-review-reality-web-journal-newsletter-fake-submit
- #3765 — code-review-mobile-rn-useapi-401-no-refresh
- #3766 — code-review-ppt-web-ui-moderation-prompt-cast
- #3767 — code-review-ppt-web-ui-ai-chat-delete-swallows-err
- #3768 — fix(api-server): clamp client limit on AI workflows/executions list

## What's next (top 5)

1. **[high · pm-scrum-master → pm-frontend]** i18n warn→error flip countdown **T-7d (2026-12-01)**. Promote-or-close the 2 remaining mobile-RN drafts #3676 (raw status) + #3677 (hardcoded button labels) by **2026-11-29 (5 days)**.
2. **[high · pm-tech-lead]** Resolve dispatcher stall **#3357** non-mechanical remainder — #3718 landed the mechanical resolver last run; reviewer-approved PRs across two merge waves (82+) still blocked.
3. **[high · pm-devops]** Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (**#2652, #2966**) — 4 KMP backlog items structurally unclaimable, carried since 2026-08.
4. **[medium · pm-backend]** Characterize new high-churn hotspots: voice_webhooks/handlers.rs (265), services/push/apns.rs + services/push/fcm.rs (open PRs #3771/#3772) — split successors to the legacy voice_webhooks.rs + push_fanout.rs hotspots.
5. **[medium · pm-devops]** Add push/* CODEOWNERS (pm-backend + pm-qa) so push-subtree PRs get automatic reviewer attachment; draft a quarantine burn-down cadence (201 rows, due 2027-02-19, 0 burned in 2 weeks).

## Blockers

- **dispatcher #3357** — non-mechanical conflicts remain after #3718 landed the key-union resolver. Owner: `pm-tech-lead`.
- **mobile-native KMP backlog (4 items)** — cloud-runner KMP + swagger-ui egress blocked (#2652, #2966). Owner: `pm-devops`.
- **mobile-RN i18n drafts #3676/#3677** — ≥22h stale, 5 days from promote-or-close; CI flips to red on 2026-12-01 if they slip. Owner: `pm-frontend`.

## Role focus today

`pm-scrum-master` (always-on), `pm-devops` (rotation index 4 → 5; was 11 days stale).

### pm-scrum-master — one-line

30 PRs merged (highest cadence so far), 11 i18n PRs cleared the ppt-web + reality-web slice, only 2 mobile-RN drafts gate the 2026-12-01 flip (both ≥22h stale), dispatcher stall #3357 non-mechanical remainder still open.

### pm-devops — one-line

CI pipeline green across the 30 merged PRs; scheduler on-time (24h delta, no drift); #2652 + #2966 still block 4 KMP items; 201 quarantined tests due 2027-02-19 have zero burn-down in two weeks (cliff risk).
