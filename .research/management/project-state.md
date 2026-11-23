# Project State — PPT (Property Management)

<sub>Generated: 2026-11-23T04:10:00Z — Scrum Master synthesis + pm-qa (rotation index 3 → 4). Coverage cursor advanced idx 9 → 10 (epic-84 re-checked, next epic-85).</sub>

## Executive summary

**24 PRs merged in-window (#3713–#3737), dispatcher-driven.** The frontend slice landed #3714 (date/number locale formatters), #3715 (native confirm → in-app dialogs), #3719 (WS token-in-URL strip audit), #3720 (leases.tsx URL helpers pinned), #3721 (default ballot ranking POST), #3732 (tenant dispute flow reachable), #3733 (decimal-safe invoice payment classification), and mobile i18n for NotificationsScreen + More tab (#3716). Backend: #3725 FCM 2xx body-parse → delivered (no retry), #3726 worker Redis-requeue drop → bounded retry, #3731 push db-error leak mapped, #3730 SSO pending-session capacity cap, #3729 upload folder_id 400, #3734 voice webhook fail-closed on RLS set failure, #3735 workflow email action delivers, #3737 voice OAuth transport 503 mapping, #3727 operator roster endpoint. Dispatcher stall #3357 unblocked via #3718 (key-union i18n merge-conflict resolver). Sprint spine (6/6) remains `done`; hardening track active. **8 drafts open — 7 are i18n-flip-adjacent, 1 is #3744 (SSO O(n) sort followup).** 1 new follow-up issue (#3738).

## Sprint progress

- **Sprint:** Hardening & Refactor — Modules Under Watch, release-248 Quarantine Burn-down, Mobile-native Backlog
- **Epics done:** 6 of 6 (100%)
- **Coverage (upkeep this run):** 49/49 stories done; rotating cursor re-checked **epic-84** (5/5 S3/e-signature/price-tracking/notification-triggers/pgvector stories done; `last_checked` advanced).

## Shipped since last run (2026-11-22T03:06:30Z → 2026-11-23T04:10:00Z)

- #3713 — fix(dispatcher): dev-truth guard for signals-promote (closes #3711)
- #3714 — code-review-ppt-web-ui-date-number-formatters-no-locale
- #3715 — code-review-ppt-web-ui-native-confirm-dialogs
- #3716 — pm-frontend i18n burn-down: mobile NotificationsScreen + More tab
- #3717 — pm-frontend AUTHED_QUERY_KEY_ROOTS vs useQuery audit + regression test
- #3718 — pm-scrum-master-resolve-dispatcher-stall-3357 (i18n locale-JSON key-union resolver)
- #3719 — pm-frontend WS token-in-URL strip audit covers every ppt-web opener
- #3720 — pm-frontend characterize: pin application-list URL helpers in high-churn leases.tsx
- #3721 — pm-frontend: POST displayed default ranking for untouched ranked ballots
- #3722 — pm-scrum-master characterize: voice OAuth refresh rate limiter (#2769)
- #3723 — bug(frontend): surface swallowed mediation sessions load error in ppt-web
- #3724 — pm-backend characterize post-split push/apns + voice_webhooks/parsers
- #3725 — bug(backend): treat 2xx FCM response as delivered on body-parse failure (no retry)
- #3726 — code-review-api-core-worker-silent-requeue-drop
- #3727 — feat(api-server): operator roster endpoint for any tenant's members
- #3728 — screen-map-drift-pr-3700-ppt: reconcile lease-create screen-map with PR #3700
- #3729 — code-review-api-handlers-upload-folderid-parse-swallow (400 on malformed folder_id)
- #3730 — code-review-reality-server-sso-sessions-unbounded-map (capacity cap)
- #3731 — code-review-api-core-push-db-err-leak (stop leaking raw sqlx into push)
- #3732 — code-review-ppt-web-ui-leases-dispute-unreachable (tenant dispute flow reachable)
- #3733 — code-review-ppt-web-ui-invoice-float-status (decimal-safe cents classification)
- #3734 — code-review-api-handlers-voice-webhook-rlsconn-unreachable (fail-closed)
- #3735 — code-review-api-core-email-action-stub-drops-mail (workflow email delivered)
- #3737 — churn-hotspot-services-voice-oauth-rs (transient transport → 503)

## What's next (top 5)

1. **[high · pm-scrum-master]** i18n warn→error flip countdown T-8d (2026-12-01). Promote-or-close drafts #3740/#3741 (reality-web locale), #3739 (realtor/[id] parity), #3742 (ppt-web Registry Rules), #3676 (mobile-rn raw status) by 2026-11-29. · dep: pm-frontend
2. **[high · pm-scrum-master]** Resolve dispatcher stall #3357 — reviewer-approved PRs still blocked across two merge waves (82+ PRs). #3718 landed the mechanical resolver; non-mechanical conflicts remain. · dep: pm-tech-lead
3. **[high · pm-scrum-master]** Unblock mobile-native/KMP + swagger-ui egress in the cloud runner (#2652, #2966) — 4 KMP backlog items still structurally unclaimable · dep: pm-devops
4. **[medium · pm-qa]** Add regression tests covering the 2xx-FCM-delivered + worker-requeue-drop + push-db-err-leak fixes (#3725/#3726/#3731). All three are source-only diffs; push service has three untested behavioral changes in flight. · dep: pm-backend
5. **[medium · pm-scrum-master]** Follow-up #3738 (SSO pending-session O(n) sort under flood) — PR #3744 drafted; verify it ships before 48h stall threshold. · dep: pm-backend

## Blockers

- **dispatcher #3357** — non-mechanical conflicts remain after #3718 landed the key-union resolver. Owner: `pm-tech-lead`.
- **mobile-native KMP backlog (4 items)** — cloud-runner KMP + swagger-ui egress blocked (#2652, #2966). Owner: `pm-devops`.
- **7 draft i18n PRs + 1 followup PR** — 2026-12-01 lint flip T-8d; promote-or-close policy must be enforced by 2026-11-29. Owner: `pm-scrum-master`.

## Role focus today

`pm-scrum-master` (always-on), `pm-qa` (rotation index 3 → 4; was 14 days stale).

### pm-qa — one-line

12/24 merged PRs this window had zero test-file diffs — up from 7/7 last pm-qa cycle. Three untested single-file push-stack fixes (#3725/#3726/#3731) + unverified "test(api-server)" commit #3722 justify a push/* `#[cfg(test)]` requirement and a per-PR classification of the no-test diffs.

### pm-scrum-master — one-line

24 PRs merged, 5 2026-11-22 action items closed via #3718/#3719/#3720/#3721/#3722, 8 drafts open (7 i18n-flip-adjacent), dispatcher-stall #3357 mechanically unblocked via #3718. i18n flip T-8d is the single highest-urgency delivery concern this week.
