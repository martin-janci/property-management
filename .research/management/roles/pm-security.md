# pm-security — 2026-06-23

```json
{
  "focus_areas": [
    "Test-hardening batch #480-#487 (legacy)",
    "Messaging epic 22 attachment IDOR (#1791)",
    "Sensor WS authz coverage (#1786)",
    "Guest OCR PII audit logging (#1783)",
    "OAuth refresh-token revocation (#481)"
  ],
  "findings": [
    {
      "area": "OAuth refresh-token revocation",
      "observation": "Regression test exists in oauth_refresh_token_tests.rs but production query coverage not re-confirmed; story gate still open for 10a-1/10a-3",
      "severity": "high",
      "evidence": "issue #481, backend/crates/db/tests/oauth_refresh_token_tests.rs"
    },
    {
      "area": "WebSocket JWT logging",
      "observation": "Notification + sensor WS still accept ?token=<jwt>; reverse proxies log query strings by default",
      "severity": "high",
      "evidence": "issue #480, sensor_ws_handler comment 'Never log params.token'"
    },
    {
      "area": "Sensor WS authz coverage",
      "observation": "Membership check is in iot.rs::sensor_ws_handler but no integration test file; refactor regression risk",
      "severity": "high",
      "evidence": "issue #1786, routes/iot.rs"
    },
    {
      "area": "N-party attachment IDOR",
      "observation": "messaging_attachments_authz_tests covers participant/cross-tenant but not >2-party cross-thread same-org IDOR",
      "severity": "high",
      "evidence": "issue #1791, routes/messaging.rs"
    },
    {
      "area": "Guest OCR PII audit",
      "observation": "Stage A stub today but audit-logging hooks absent at the seam; Stage B will flow real PII through unlogged path",
      "severity": "medium",
      "evidence": "issue #1783, id_ocr.rs"
    }
  ],
  "next_actions": [
    {
      "action": "Close #481: verify revoked_at IS NULL predicate in OAuthRepository::find_refresh_token_by_hash, gate 10a-1/10a-3 on green CI",
      "priority": "high",
      "dependency": "none"
    },
    {
      "action": "Resolve #480: move WS auth off query-string to ticket pattern OR add structured-log redaction for ws routes",
      "priority": "high",
      "dependency": "none"
    },
    {
      "action": "Add sensor_ws_authz_tests.rs covering non-member, expired-JWT, cross-org rejects before story 14.3 done",
      "priority": "high",
      "dependency": "none"
    },
    {
      "action": "Extend messaging_attachments_authz_tests with N-party cross-thread same-org case; audit thread-participant check in download handler",
      "priority": "high",
      "dependency": "none"
    },
    {
      "action": "Add PII audit-log records (actor, guest_id, document_type, ts; no bytes/PII) on guest OCR upload + extract before Stage B provider wires in",
      "priority": "medium",
      "dependency": "none"
    }
  ],
  "risks": [
    {
      "risk": "OAuth refresh-token revocation bypass (#481) \u2014 revoked tokens exchangeable, RFC 9700 violation with account-takeover potential",
      "probability": "medium",
      "impact": "high",
      "mitigation": "Confirm production query has revoked_at IS NULL; CI must run regression"
    },
    {
      "risk": "WS JWT in query string (#480) leaks via reverse-proxy access logs",
      "probability": "high",
      "impact": "high",
      "mitigation": "Ticket exchange before WS upgrade OR proxy/middleware redaction"
    },
    {
      "risk": "N-party attachment IDOR (#1791): download handler may check org not thread participant",
      "probability": "medium",
      "impact": "high",
      "mitigation": "Audit link_message_attachment + download handler; add cross-thread test"
    },
    {
      "risk": "Sensor WS authz untested (#1786): future refactor silently drops membership guard",
      "probability": "low",
      "impact": "high",
      "mitigation": "Add CI test gate before 14.3 done"
    },
    {
      "risk": "MFA brute-force test gap (#487): nested mod common may silently omit test binary",
      "probability": "medium",
      "impact": "medium",
      "mitigation": "Fix module structure, add rate-limit tests, confirm CI binary runs"
    }
  ],
  "decisions_needed": [
    "WS token transport: short-lived ticket exchange (eliminates JWT-in-URL log risk) vs proxy-layer redaction (faster but ongoing ops discipline)",
    "OAuth story promotion gate: close #481/#487 vs formally defer before 10a-1/10a-3 done"
  ]
}
```
