# PPT Project State

_Generated: 2026-06-23T10:30:00Z · run mode: rotating (pm-security)_

## Executive summary

245 PRs merged in 7 days; 12 epics in flight; 37/49 mvp-scoped stories `done`, 12 `partial`. Velocity is unsustainable — 56 open from-merged-review follow-ups plus the legacy #480-#487 test-hardening batch still gate OAuth (10a-1/10a-3), notification preferences (8a-3), and Epic 6 web stories from promotion.

## Sprint progress

- Sprint: Epics 6, 7A, 8A, 10A, 10B, 11, 14, 18, 22 (in flight; sprint-status.yaml stale)
- Epics done: 9 / 14
- Stories done: 37 / 49

## Shipped since last run

- Story 11.7 financial reports + PDF/XLSX export (#1717)
- Story 11.6 scheduler: payment reminders + auto-overdue (#1709)
- Story 14.3 sensor WebSocket realtime channel (#1644)
- Story 18 guest ID-document OCR (#1750 + follow-up #1783)
- Epic 22 messaging: N-party + attachments + camelCase (#1689,#1696,#1702,#1756,#1768)
- BIT-185 fault notifications fire on every lifecycle event (#1705)
- BIT-188 unit management UI (#1698)
- Churn-hotspot splits: forms.rs (#1700), aml_dsa.rs (#1708), document.rs (#1683), booking.rs (#1693), form.rs sort dedup (#1781)
- 245 PRs merged to dev (1440-1781); 12 dependabot bumps; 1 revert reconciled

## What's next (top 5)

- **[high]** Triage 56 open from-merged-review follow-ups (#1758-#1793); start with security-labeled #1791 attachment IDOR, #1786 sensor WS authz tests, #1791 attachment IDOR, #1783 PII audit-logging, #1785 Stripe Checkout hardening — owner: pm-security
- **[high]** Close or formally defer test-hardening batch #480-#487 — #481 OAuth refresh-token revocation bypass and #480 WS JWT in access logs gate 10a-1, 10a-3, 8a-3 promotions — owner: pm-security
- **[high]** Complete Epic 6 partial stories 6-2/6-3/6-4 — wire remaining mobile UI gaps and flip sprint-status to done once issue #486 resolves — owner: pm-frontend
- **[high]** Complete Epic 80 partials 80-2 (5-step wizard + i18n) and 80-3 (party submissions endpoints wiring) to close MVP epic — owner: pm-frontend
- **[medium]** Update sprint-status.yaml to reflect actual state (epics 7a/8a/10b stories done; add 11/14/18/22 epic blocks) — stale YAML is misdirecting planning — owner: pm-scrum-master

## Blockers

- Stories 10a-1/10a-3 OAuth provider — issue #481 (refresh-token revocation bypass) + #487 (MFA rate-limit test gap) open (owner: pm-security)
- Stories 6-2/6-5 announcement viewing + direct messaging — issue #486 (getToken bypass of axios interceptor) breaks refresh path (owner: pm-frontend)
- Story 8a-3 notification preference sync — issues #480 (WS JWT in logs) + #484 (serial FCM dispatch) open (owner: pm-security)
- Story 80-3 mediation resolution — party submissions endpoints unwired (owner: pm-frontend)
- Research dispatcher cron #1680 — env issue blocks core pipeline auto-runs (owner: pm-devops)

## Role focus today

- pm-scrum-master (always-on synthesis)
- pm-security (rotation; last run 2026-05-27, 27 days)

### pm-scrum-master

Velocity surge generated 56 follow-ups; sprint-status.yaml is materially stale vs coverage.json. Triage + sprint-status refresh are the highest-leverage moves.

### pm-security

Five legacy test-hardening items still gate sprint promotions; messaging-epic-22 N-party attachment IDOR (#1791) + sensor WS authz coverage gap (#1786) are the new regression surface from this sprint.

