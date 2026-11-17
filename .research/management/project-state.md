# Project State — PPT (Property Management)

<sub>Generated: 2026-11-17T03:17:29Z — Scrum Master synthesis + pm-data (rotation index 6 → 7). Coverage cursor advanced idx 4 → 5 (epic-7a re-checked, next epic-80).</sub>

## Executive summary

Sprint (Epic 6 / 7A / 8A / 10A) and all six tracked epics remain **done**; `coverage.json` holds 49/49 stories done (0 partial or not-started), so delivery work is entirely on the improvement track. 22 PRs (#3560-#3581) merged in this window — mostly raw-DB-error sanitize sweeps and digest-subsystem hardening (#3574 abandoned-send surfacing, #3576 digest worker replica-claim, #3570 quiet-hours master gate). The Phase 1.5 rotating review of `api-core` surfaced three more digest-correctness findings (INNER-JOIN silently excludes inactive-user cohort, no `abandoned_at` terminal state, advisory-lock release leak), and two new follow-up issues (#3583 RLS super-admin context bleed, #3584 mobile biome gate downgrade) opened against just-merged work. The digest/notification data path is the main churn hotspot and the right pm-data focus.

## Sprint progress

- **Sprint:** Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- **Epics done:** 6 of 6 (100%)
- **Coverage (last upkeep 2026-11-17):** 49 of 49 stories done; rotating cursor re-checked epic-7a (Document Upload/Folder/Permissions/Download/Share — 5 stories still done).

## Shipped since last run (2026-11-16T03:12:00Z → 2026-11-17T03:17:29Z)

- 7 raw-DB-error sanitize PRs: #3560 GDPR handlers, #3563 ai-workflows, #3569 tenant-screening, #3572 ESG reporting, #3575 property valuation, plus #3565 cross-tenant 404 contract sweep on moderation mutations and #3573 /refresh swallowed-revoke error.
- Digest subsystem hardening: #3574 surface abandoned digest sends, #3576 digest worker replica claim (advisory lock), #3570 quiet-hours master gate on weekends.
- Security and authz: #3564 unit-scoped cross-owner delegation authz regression (audit found no un-fixed gap), #3571 regression test for PR #3525 peer-role privilege escalation, #3568 RN deep-link pending singleton isolated by user, #3577 tenant-config RLS clear no longer swallowed.
- ppt-web fixes: #3561 useNetworkStatus navigator/window guard, #3562 moderation View Content via SPA router.
- mobile-rn: #3566 react-hooks + i18n lint rules adopted warn-only for one sprint, #3567 ReportFault category chips via i18n.
- Planning and docs: #3578 coverage/project-state refresh, #3579 JWT iss/aud (#3427) + Dependabot majors sequenced behind the swagger-ui egress fix, #3580 + #3581 screen-map drift syncs.

## What's next

1. **[high] [pm-scrum-master]** Fix digest-candidates INNER JOIN in `granular_notification.rs find_inactive_digest_candidates` (LEFT JOIN with `COALESCE(la.last_active, u.created_at)`) and add a regression test for a user with no `refresh_tokens` row.
2. **[high] [pm-scrum-master]** Add terminal abandoned state for digests (`abandoned_at` column via new migration, set in `report_abandoned`) so abandoned rows are queryable, not log-only.
3. **[high] [pm-scrum-master]** Triage issue #3583 (RLS super-admin context bleed, follow-up to #3577) and confirm whether pooled connections can retain a super-admin RLS context.
4. **[high] [pm-devops]** Unblock the cloud-runner backend compile — utoipa-swagger-ui egress 403 (#2966) blocks api-server builds and keeps #3583 / digest fixes / #3427 unverifiable locally.
5. **[high] [pm-scrum-master]** Triage stalled PRs #2555 and #2559 (~110 days) — merge, close, or explicitly defer.

## Blockers

- utoipa-swagger-ui egress 403 (#2966) makes api-server unbuildable in the cloud runner; digest fixes and #3583 cannot be compiled or tested there — owner: pm-devops.
- mobile-native KMP builds unclaimable in the cloud runner (#2652) — owner: pm-devops.
- #3427 JWT iss/aud hardening labeled blocked and sequenced behind the egress fix by #3579 — owner: pm-security.
- PRs #2555 and #2559 (accounting trio) stalled 110+ days; no merge/close/defer decision recorded — owner: pm-tech-lead.

## Role focus today

- **pm-scrum-master** (always-on) — delivery synthesis + 6 next-actions.
- **pm-data** (rotation idx 6) — digest/analytics correctness lens; next rotation advances to **pm-integration** (idx 7).

### pm-data — one-line summary
ppt-web has a `trackEvent` sink, admin-web has onboarding analytics, mobile has no evident analytics layer, and the three Phase 1.5 digest findings are all reporting-correctness defects (cohort drop, no terminal state, no digest metrics). Full role analysis in `roles/pm-data.md`.

### pm-scrum-master — one-line summary
22 PRs merged this window on improvement-track hardening; three digest-subsystem code-review findings landed right behind #3574/#3576, so the hotspot risk is bundle-or-ship-separately rather than drift. Full role analysis via `shipped_since_last_run` + `next_actions` in `action-list.json`.
