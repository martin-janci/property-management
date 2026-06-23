# pm-scrum-master — 2026-06-23

```json
{
  "sprint_progress": {
    "sprint": "Epics 6, 7A, 8A, 10A, 10B, 11, 14, 18, 22 (in flight; sprint-status.yaml stale)",
    "epics_done": 9,
    "epics_total": 14,
    "stories_done": 37,
    "stories_total": 49
  },
  "shipped_since_last_run": [
    "Story 11.7 financial reports + PDF/XLSX export (#1717)",
    "Story 11.6 scheduler: payment reminders + auto-overdue (#1709)",
    "Story 14.3 sensor WebSocket realtime channel (#1644)",
    "Story 18 guest ID-document OCR (#1750 + follow-up #1783)",
    "Epic 22 messaging: N-party + attachments + camelCase (#1689,#1696,#1702,#1756,#1768)",
    "BIT-185 fault notifications fire on every lifecycle event (#1705)",
    "BIT-188 unit management UI (#1698)",
    "Churn-hotspot splits: forms.rs (#1700), aml_dsa.rs (#1708), document.rs (#1683), booking.rs (#1693), form.rs sort dedup (#1781)",
    "245 PRs merged to dev (1440-1781); 12 dependabot bumps; 1 revert reconciled"
  ],
  "next_actions": [
    {
      "action": "Triage 56 open from-merged-review follow-ups (#1758-#1793); start with security-labeled #1791 attachment IDOR, #1786 sensor WS authz tests, #1791 attachment IDOR, #1783 PII audit-logging, #1785 Stripe Checkout hardening",
      "owner_role": "pm-security",
      "priority": "high",
      "dependency": "none"
    },
    {
      "action": "Close or formally defer test-hardening batch #480-#487 \u2014 #481 OAuth refresh-token revocation bypass and #480 WS JWT in access logs gate 10a-1, 10a-3, 8a-3 promotions",
      "owner_role": "pm-security",
      "priority": "high",
      "dependency": "none"
    },
    {
      "action": "Complete Epic 6 partial stories 6-2/6-3/6-4 \u2014 wire remaining mobile UI gaps and flip sprint-status to done once issue #486 resolves",
      "owner_role": "pm-frontend",
      "priority": "high",
      "dependency": "Issue #486 closed"
    },
    {
      "action": "Complete Epic 80 partials 80-2 (5-step wizard + i18n) and 80-3 (party submissions endpoints wiring) to close MVP epic",
      "owner_role": "pm-frontend",
      "priority": "high",
      "dependency": "none"
    },
    {
      "action": "Update sprint-status.yaml to reflect actual state (epics 7a/8a/10b stories done; add 11/14/18/22 epic blocks) \u2014 stale YAML is misdirecting planning",
      "owner_role": "pm-scrum-master",
      "priority": "medium",
      "dependency": "none"
    },
    {
      "action": "Resolve dispatcher cron env issue #1680 \u2014 unblocks automated coverage scans + Tier-2 refill",
      "owner_role": "pm-devops",
      "priority": "medium",
      "dependency": "none"
    }
  ],
  "blockers": [
    {
      "item": "Stories 10a-1/10a-3 OAuth provider \u2014 issue #481 (refresh-token revocation bypass) + #487 (MFA rate-limit test gap) open",
      "owner_role": "pm-security"
    },
    {
      "item": "Stories 6-2/6-5 announcement viewing + direct messaging \u2014 issue #486 (getToken bypass of axios interceptor) breaks refresh path",
      "owner_role": "pm-frontend"
    },
    {
      "item": "Story 8a-3 notification preference sync \u2014 issues #480 (WS JWT in logs) + #484 (serial FCM dispatch) open",
      "owner_role": "pm-security"
    },
    {
      "item": "Story 80-3 mediation resolution \u2014 party submissions endpoints unwired",
      "owner_role": "pm-frontend"
    },
    {
      "item": "Research dispatcher cron #1680 \u2014 env issue blocks core pipeline auto-runs",
      "owner_role": "pm-devops"
    }
  ],
  "decisions_needed": [
    "Test-hardening batch #480-#487 (open 4+ weeks): formally defer to security-hardening sprint or block all dependent story promotions until closed?",
    "sprint-status.yaml predates Dec 2025; replace as authority with coverage.json, or refresh in place?",
    "56 open from-merged-review issues: all must close before next release cut, or non-security items carry forward as fast-follows?",
    "Draft PR #1754 admin-web stale 404/501 fallbacks: approve and merge or close?"
  ],
  "risks": [
    {
      "risk": "Issue #1791 (attachment IDOR) on newly-shipped messaging epic 22 may allow cross-thread attachment download by org member",
      "probability": "medium",
      "impact": "high",
      "mitigation": "pm-security audit handler before any prod traffic; add cross-thread regression test",
      "owner_role": "pm-security"
    },
    {
      "risk": "Issue #481 (OAuth refresh-token revocation bypass, RFC 9700 violation) open since May 2026; any prod OAuth deploy is insecure",
      "probability": "high",
      "impact": "high",
      "mitigation": "Block 10a-1/10a-3 from prod until #481 closed; confirm regression test wired in CI",
      "owner_role": "pm-security"
    },
    {
      "risk": "Velocity of 245 PRs/7 days generated 56 follow-ups + churn hotspots; accumulating debt may slow next sprint",
      "probability": "high",
      "impact": "medium",
      "mitigation": "Reserve 20-30% of next sprint to drain from-merged-review backlog below 20 items",
      "owner_role": "pm-backend"
    },
    {
      "risk": "Issue #1783 (PII audit-logging) on guest OCR story 18 may create GDPR exposure if real PII flows without audit trail",
      "probability": "medium",
      "impact": "high",
      "mitigation": "Block story 18 prod enablement until audit logging confirmed complete",
      "owner_role": "pm-security"
    },
    {
      "risk": "WS JWT bearer tokens in query parameters (#480) leak via reverse-proxy access logs; both notification and sensor WS affected",
      "probability": "high",
      "impact": "high",
      "mitigation": "Adopt short-lived ticket exchange before WS upgrade OR query-string redaction; enforce before any WS feature ships to prod",
      "owner_role": "pm-security"
    }
  ],
  "role_focus_today": "scrum-master + pm-security rotation"
}
```
