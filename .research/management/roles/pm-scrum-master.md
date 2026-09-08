# pm-scrum-master — 2026-09-08

_Always-on delivery synthesis. Window: 2026-09-01..2026-09-08 (~6.5 day lag since last routine run). 14 PRs merged; 5 new open issues (3 IDOR bugs + 2 infra blockers). Sprint status unchanged. Buffer starved at 8/72 claimable._

## Summary

Auto-review loop kept shipping frontend hardening (401 replay + refresh single-flight, JWT-exp cold-boot, allowlist auto-derivation, i18n externalization, PDF-renderer tests) but three cross-tenant IDOR handlers opened 2026-09-06 (#2944/#2945/#2946) are stuck: they need api-server changes that the cloud runner cannot build (#2949). PR #2950 landed a real security fix (shared-device cache leak) but merged with red CI because infra #2951 (mobile jest / RN version rot) blocks the mobile suite pre-diff.

## Return payload

```json
{
  "role": "pm-scrum-master",
  "summary": "14 PRs shipped since 2026-09-01 (frontend auth + i18n + backend test-coverage), all merged post-review; three open cross-tenant IDOR bugs cannot land because api-server cloud verify is structurally blocked (#2949). Delivery mechanism is healthy for frontend/reality-server; api-server + mobile-native + mobile-rn are cloud-verify-blocked.",
  "next_actions": [
    {"action": "Land infra #2949 (vendor swagger-ui) to unblock the IDOR trio", "priority": "high", "dependency": "pm-devops", "definition_of_done": "`cargo build -p api-server` green in cloud sandbox"},
    {"action": "Batch-fix IDOR #2944 + #2945 + #2946 as a labelled security PR set once #2949 clears", "priority": "high", "dependency": "pm-security + pm-backend (blocked on #2949)", "definition_of_done": "three PRs merged with sqlx cross-tenant regression tests; issues closed"},
    {"action": "Wire ppt-web direct-to-S3 upload (84-1) + build signer-facing document-sign page (84-2) — the last 2 partial stories in coverage, aging 4+ windows", "priority": "high", "dependency": "pm-frontend", "definition_of_done": "both stories `done` in sprint-status and coverage; MVP 49/49"},
    {"action": "Resolve infra #2951 (jest-expo/RN version rot) so mobile-rn security patches like PR #2950 stop merging red-CI", "priority": "medium", "dependency": "pm-devops", "definition_of_done": "mobile jest suite loads in cloud runner; #2951 closed; PR #2950 CI green retroactively"},
    {"action": "Resolve cargo-deny RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) — carried since 2026-08-18", "priority": "high", "dependency": "pm-security", "definition_of_done": "h2 bumped to patched release; waiver removed from .cargo-deny.toml"},
    {"action": "Advance pm-devops-unblock-mobile-native-cloud-builds (issue #2652) — 6/9 open action-list items are structurally unclaimable in cloud", "priority": "high", "dependency": "pm-devops", "definition_of_done": "mobile-native AGP builds green in cloud sandbox"}
  ],
  "risks": [
    {"risk": "Three infra blockers (#2949 api-server, #2951 mobile-rn, #2652 mobile-native) now collectively strand ~80% of the codebase from cloud verify — only ppt-web/admin-web/reality-web/reality-server ship cleanly", "probability": "high", "impact": "high", "mitigation": "Escalate infra remediation as a coordinated pm-devops batch; consider splitting the dispatcher's 36-slot buffer into cloud vs local buckets"},
    {"risk": "IDOR bugs public on the repo for 2 days without a fix pathway — reputational + regulatory exposure grows daily", "probability": "high", "impact": "high", "mitigation": "Land #2949 in the next 24h; if not feasible, author fixes locally and push via ppt-bridge"}
  ],
  "open_questions": [
    "Do the 84-1/84-2 frontend partials still make sense at high priority when the more urgent security batch is blocked?",
    "Should the auto-implementer dispatcher explicitly de-prioritize api-server tasks while #2949 is open, to avoid burning cycles on cloud-unbuildable specs?"
  ],
  "decisions_needed": [
    "Freeze api-server implementer picks until #2949 lands — owner: pm-tech-lead",
    "Set an SLA on IDOR-labelled issues (e.g. 48h fix-or-mitigate) — owner: pm-tech-lead"
  ],
  "shipped_since_last_run": [
    "#2922 code-review reality-server saved-search: typed error enum for HTTP status (pm-backend)",
    "#2925 code-review api-handlers compliance: raw DB-error leak fix via db_error (pm-backend, security)",
    "#2926 (per input list — code-review batch)",
    "#2927 (per input list — code-review batch)",
    "#2928 (per input list — code-review batch)",
    "#2931 (per input list — code-review batch)",
    "#2932 (per input list — code-review batch)",
    "#2938 gh-issue-2937 accounting-server invoice PDF renderer + handler tests (pm-qa)",
    "#2940 code-review ppt-web: externalize ConfirmationDialog loading label via i18n (pm-frontend)",
    "#2941 code-review ppt-web AuthContext: validate JWT exp at cold boot (pm-frontend, security)",
    "#2942 code-review ppt-web api.ts: single-flight refresh + one-shot 401 replay (pm-frontend, security)",
    "#2943 code-review ppt-web: enforce logout purge allowlist coverage — hand-maintained set (pm-qa, security)",
    "#2950 gh-issue-2947 mobile: purge NFC access log / QR history / offline feedback on session change (pm-frontend, security — merged red-CI due to #2951)",
    "#2952 gh-issue-2948 ppt-web: auto-discover feature-local *Keys factories in logout-purge coverage test (pm-tech-lead, security)"
  ],
  "sprint_progress": {"sprint": "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth", "epics_done": 3, "epics_total": 5},
  "blockers": [
    {"item": "IDOR trio #2944/#2945/#2946", "reason": "api-server cloud verify structurally blocked by #2949 (utoipa-swagger-ui build-script egress)", "owner_role": "pm-devops"},
    {"item": "PR #2950 (mobile security fix)", "reason": "merged red-CI; #2951 blocks mobile jest suite in cloud (jest-expo@56 ↔ RN@0.87 version rot)", "owner_role": "pm-devops"},
    {"item": "mobile-native/KMP backlog (6 items)", "reason": "standing infra #2652 blocks cloud verify", "owner_role": "pm-devops"},
    {"item": "cargo-deny RUSTSEC-2026-0258 (h2 DoS)", "reason": "unfixed advisory blocks backend PRs (waivered)", "owner_role": "pm-security"},
    {"item": "84-1 + 84-2 (aging partial stories)", "reason": "5th consecutive upkeep window without dispatcher pickup; backend shipped, frontend slice pending", "owner_role": "pm-frontend"}
  ]
}
```
