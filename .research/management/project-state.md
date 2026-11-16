# Project State — PPT (Property Management)

<sub>Generated: 2026-11-16T03:12:00Z — Scrum Master synthesis (Phase 1.6, rotating role: pm-security)</sub>

## Executive summary

Sprint (Epic 6 / 7A / 8A / 10A — Announcements, Documents, Notifications & OAuth) and all six tracked epics are **done**. `coverage.json` holds 49/49 stories done (0 partial or not-started), so gap-driven planning has no candidates. Work has moved to the improvement backlog (63 open action-list items after today's merge). 17 code-review and gh-issue PRs merged in this window — a continued dispatcher burst — and `coverage.json` + this file last refreshed 2026-11-13 and are stale; a scan is recommended.

## Sprint progress

- **Sprint:** Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- **Epics done:** 6 of 6 (100%)
- **Coverage (last scan 2026-11-13):** 49 of 49 stories done

## Shipped since last run (2026-11-15T11:30:00Z → 2026-11-16T03:12:00Z)

- #3558 reality-web currency-code RangeError crash
- #3559 reality-web login open-redirect (backslash)
- #3557 ppt-web errorHandler 403 mislabel
- #3555 api-core notify send-before-stamp duplicate
- #3556 api-core payment-reminders batch abort
- #3554 api-handlers delegation null-unit owner check
- #3552 reality-server listing slug is-uuid
- #3551 reality-server categories hardcoded sk labels
- #3550 ppt-web signature-modal raw error
- #3549 ppt-web reports route stub analytics
- #3548 gh-issue-3544 feedback mount (issue closed)
- #3546 ppt-web faults URL-param unvalidated cast
- #3547 ppt-web nav aria-label i18n
- #3545 mobile-rn imagecompress width-only resize
- #3543 mobile-rn feedback-manager no auth token
- #3542 gh-issue-3531 i18n sweep
- #3541 ppt-web facilities isManager hardcoded true

## What's next

1. **[high] [pm-scrum-master]** Unblock mobile-native/KMP and swagger-ui egress in the cloud runner (#2652, #2966). Claimable work is a small share of the backlog, and backend security PRs cannot compile.
2. **[high] [pm-scrum-master]** Run a coverage scan to refresh `coverage.json` and this file (both dated 2026-11-13), and confirm that epic rollups and coverage are still 100% done.
3. **[high] [pm-scrum-master]** Triage the stalled reviews #2555 and #2559 (110 days). Merge them, close them, or explicitly defer them.
4. **[high] [pm-scrum-master]** Run the cross-tenant 404 contract sweep and close the open IDOR retries #2944/#2945/#2946 once the build is unblocked.
5. **[high] [pm-security]** Audit routes with optional `unit_id` or null-scope for the missing-owner-check pattern from #3554, and add a negative cross-owner test.

## Blockers

- **All backend-touching dispatcher tasks, including IDOR retries #2944/#2945/#2946** — utoipa-swagger-ui egress 403 (#2966) makes api-server unbuildable in the cloud runner — owner: `pm-devops`
- **mobile-native KMP backlog** (create-listing, inquiries contract, http timeout, pagination cap, SSO error body) — KMP builds are unclaimable in the cloud runner (#2652) — owner: `pm-devops`
- **#3427 JWT iss/aud hardening** — labeled blocked; needs operator triage — owner: `pm-security`
- **PRs #2555 and #2559** — stalled 110 days; accounting trio, blocked on the build — owner: `pm-tech-lead`

## Role focus today

`pm-scrum-master` (always-on), `pm-security` (rotation index 5)

## Role notes

- **pm-security:** No auth/JWT changes in the last 5 backend commits. The only security-relevant change is #3554 (null-unit delegation owner check); sibling-path audit requested. Open deep-link pending singleton (mobile-rn, score 2) and KMP SSO error body (score 2) remain. #3560 (gdpr raw DB error leak) is in review.
