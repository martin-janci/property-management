# Project State — PPT (Property Management)

<sub>Generated: 2026-11-20T03:06:10Z — Scrum Master synthesis + pm-tech-lead (rotation index 0 → 1). Coverage cursor advanced idx 6 → 7 (epic-81 re-checked, next epic-82).</sub>

## Executive summary

Sprint (Epic 6 / 7A / 8A / 10A) and the six tracked epics remain **done**; `coverage.json` holds 49/49 stories done (0 partial, 0 not-started). Delivery is a continuous hardening track. 16 PRs merged this window (#3630, #3631, #3632, #3633, #3638, #3639, #3640, #3641, #3642, #3643, #3644, #3645, #3646, #3647, #3648, #3628) — all code-review follow-ups + churn characterization + KPI definitions. Highlights: shared SSRF redirect revalidation guard (#3647), HelloSign webhook envelope binding (#3648), runtime 401 token refresh (#3646), cross-tenant rental connection IDOR regression (#3642), dead unscoped rental IDOR-footgun removal (#3641), vote-reminder throttle (#3639), WidgetSettings i18n (#3630). pm-tech-lead's 29-days-since-last-run reads surfaced a clear structural signal: the three churn hotspots this run (voice_webhooks.rs 3066 LoC / runs_seen=5, push_fanout.rs 3038 LoC / runs_seen=4, oauth_integration_tests.rs 3427 LoC / runs_seen=2) all breach a reasonable file-size ceiling and keep being re-touched; decomposition is overdue. Follow-up issue #3649 (suggestions seed-city for region `de` resolves to Wien) was opened as a direct fallout of #3628. Phase 1.5 rotating review of `api-handlers` (churn-aligned) surfaced one bug: `voice_webhooks.rs` handlers run `RlsConnection` extractor **before** signature verification — returning 401 to every live Alexa/Google webhook body.

## Sprint progress

- **Sprint:** Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth
- **Epics done:** 6 of 6 (100%)
- **Coverage (upkeep this run):** 49 of 49 stories done; rotating cursor re-checked **epic-81** (`last_checked` advanced for its stories; no status transitions).

## Shipped since last run (2026-11-19T03:18:31Z → 2026-11-20T03:06:10Z)

- #3647 — share per-hop SSRF redirect revalidation guard (reality-server import fetch)
- #3648 — bind HelloSign webhook envelope to verified provider
- #3646 — runtime 401 token refresh for the generated API client (ppt-web)
- #3645 — guard SSO error_description decode against URIError
- #3644 — gate raw ErrorBoundary message behind DEV
- #3643 — handle useAgencyBranding fetch error (reality-web — no silent colour reset)
- #3642 — regression test for cross-tenant rental connection IDOR
- #3641 — remove dead unscoped rental IDOR-footgun repo methods
- #3640 — surface swallowed idempotency commit error
- #3639 — throttle vote reminders to a ~5-minute minimum interval
- #3631 — echo clamped limit in compliance audit-log pagination
- #3630 — WidgetSettingsScreen copy routed through i18n (mobile RN)
- #3628 — derive suggestions seed region from request (opened follow-up #3649)
- #3632 — characterize role-default JSONB parsing (churn hotspot)
- #3633 — characterize digest config parsing (churn hotspot)
- #3638 — backfill shipped-story KPI definitions + story Events/KPI template section

## What's next (top 5)

1. **[high · pm-tech-lead]** Split `services/push_fanout.rs` (3038 LoC) into a `push/` module (fcm/apns/adapter/worker/config) — DoD: no file > ~800 LoC, behaviour unchanged, existing push tests pass.
2. **[high · pm-tech-lead]** Split `routes/voice_webhooks.rs` (3066 LoC, 5-run hotspot) into signature verification + idempotent event handler + provider parsers; move business logic into `services/`.
3. **[high · pm-tech-lead]** Resolve dispatcher stall #3357: rebase/resolve the 3 approved conflicting PRs; define an escalation path for non-mechanical conflicts.
4. **[high · pm-backend]** Sequence listings.rs PRs: rebase #3617 (record_view existence + per-IP throttle) and merge before #3628 follow-up; fix #3649 so region `de` does not seed Wien.
5. **[high · pm-devops]** Unblock KMP and swagger-ui egress in the cloud runner (#2652, #2966) — current claimable only ~9/72 for mobile-native.

## Blockers

- **Dispatcher pipeline (#3357)** — 3 approved PRs blocked on non-mechanical conflicts; pipeline fully stalled. Owner: `pm-tech-lead`.
- **KMP / mobile-native backlog items** (inquiries-response-contract, create-listing-not-wired, http-client-no-timeout, sso-error-body-unguarded) — Gradle/KMP builds unavailable in cloud runner (#2652, #2966); claimable ~9/72. Owner: `pm-devops`.
- **Dependabot bumps** (#3634-#3637, #3441, #3635 gradle wrapper) — open, unmerged; gradle/kotlinx bumps cannot be validated in cloud. Owner: `pm-devops`.

## Role focus today

`pm-scrum-master` (always-on), `pm-tech-lead` (rotation index 0 → 1).

### pm-tech-lead — one-line

Three oversized backend files are churning in a tight loop (voice_webhooks.rs, push_fanout.rs, oauth_integration_tests.rs). The structural risk — not feature risk — is now the top delivery blocker; decompose before more review follow-ups land.

### pm-scrum-master — one-line

Sprint spine is fully done (6/6 epics). This run was a hardening wave (16 merged code-review PRs). The pipeline is the constraint: dispatcher stall + KMP/swagger egress keep the real bottlenecks off the claimable buffer.
