# PPT Roadmap Snapshot

_Generated: 2026-08-21 — Phase 1.6 upkeep. Sources: coverage.json, sprint-status.yaml, action-list.json._

## Current sprint

**Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
`epics_done = 3 / 5` unchanged. 47 / 49 tracked stories done, 2 partial.

| Epic | Stories | Status | Notes |
|---|---|---|---|
| 6 Announcements | 6/6 done | in-progress (sprint) | Full stack shipped; sprint flag stale |
| 7A Documents | 5/5 done | in-progress (sprint) | Backend + web shipped; sprint flag stale |
| 8A Notification prefs | 3/3 done | done | Mobile FCM/APNs deferred |
| 10A OAuth Provider | 3/3 done | done | Test-hardening gates all closed |
| 10B Platform Admin | 7/7 done | in-progress (sprint) | All 7 stories done in coverage |

## Extended scope (coverage-tracked, outside primary sprint)

| Epic | Coverage state | Highlights |
|---|---|---|
| 79 Frontend integration | 4/4 done | Re-checked 2026-08-21 upkeep; no changes in window |
| 80 Disputes | 3/3 done | Sprint status still "partial" (reconciliation pending) |
| 81 Reports | 2/2 done | — |
| 82 iOS SwiftUI | 5/5 done | Screen-map buildStatus drift on inquiries/account |
| 83 Portal integrations | 3/3 done | Airbnb / Booking / webhooks |
| 84 Docs & e-sign | 3/5 done, 2 partial | 84-1 direct-to-S3 wiring + 84-2 sign page |
| 85 Env / build config | 2/2 done | — |
| 8a Notifications (dup) | 3/3 done | — |
| 9 2FA | 1/1 done | — |

## Immediate priorities (next 3)

1. **[high]** Land gh-issue-2797 (retire h2 0.3.x root cause of RUSTSEC-2026-0258) — pm-security
2. **[medium]** Add QA regression tests for PR #2813 (inquiry-detail messages) and PR #2806 (voice-webhook empty-secret fail-closed) — pm-qa
3. **[medium]** Human merge for PR #2744 (self-PR blocks formal APPROVE) — pm-tech-lead

## Known open follow-ups (from action-list)

- **gh-issue-2743** — dispatcher recurrence: archive push ceiling + retry-remint ghost retry (PR #2744 in review)
- **gh-issue-2794** — voice device dedup: enforce (org,user,platform) uniqueness at DB level (follow-up to #2793)
- **gh-issue-2797** — cargo-deny advisories: RUSTSEC-2026-0258 h2 DoS blocks every backend PR (partially scoped by #2805)
- **mobile-native-kmp hygiene batch** — InquiriesResponse contract drift, HttpClient no timeout, portfolio analytics 100-cap + unbounded fanout, cancellation swallowed, SsoService untested
- **reality-web i18n gaps** — AgencyErrorState + ComparisonView untranslated (in-progress)
- **NFC SecureStore 2KB blob overflow** (mobile RN, in-progress)
- **share access log proxy IP unwind** (in-progress)
- **churn hotspot** — backend/servers/api-server/src/routes/voice_webhooks.rs (2 runs seen)

## Risk register (top items)

- **PR #2568 CSRF fix half-wired (SsoStateStore.mint no call site)** — pm-mobile, open
- **PR #2571 DELETE-by-file-key same-org reference gap** — pm-backend, open
- **Accounting MVP-loop trio reviewer starvation** (#2555 / #2558 / #2559) — pm-tech-lead, open
- **Dispute-KPIs quarantined-test-only** (#2575) — pm-backend, open
- **Analytics blindspots on shipped MVP features** — pm-data, standing
- **Metric-definition drift (FaultStatusCount vs owner/portfolio KPIs)** — pm-data, standing
