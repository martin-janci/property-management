# pm-security — 2026-09-08

_Rotation slot idx 5 → 6 (last run 2026-07-21, 49d stale). Run mode: incremental (pm:pm-security). Scope this run: 3 open IDOR bugs opened 2026-09-06 (#2944/#2945/#2946), the infra blocker #2949 keeping them unlandable in the cloud runner, ppt-web auth hardening merged this window (#2941/#2942/#2943/#2952), and the shared-device cache leak fix #2950 (merged red-CI on infra #2951)._

## Summary

Three cross-tenant IDOR handlers (violations comments/evidence/payments, portfolio_properties, portfolio_analytics property metrics) are open with a public label and cannot be landed by the cloud dispatcher because the `api-server` verify gate is structurally blocked (#2949 — utoipa-swagger-ui build script egress). Frontend auth hardening was strong this window (single-flight refresh + 401 replay, cold-boot JWT-exp validation, allowlist enforcement now auto-derived) but the highest-severity finds (cross-tenant data disclosure) sit unshipped.

## Evidence I checked

- `backend/servers/api-server/src/routes/portfolio_analytics.rs:282,314` — `let _org_id = user.tenant_id.ok_or_else(...)` on `upsert_property_metrics` and `get_property_metrics`; the value is computed then discarded before the repo call — matches #2946 exactly.
- Two other `_org_id` sites (`routes/migration.rs:1472`, `routes/faults.rs:642` as `_tenant_id`) — same anti-pattern; sweep candidate.
- `routes/violations.rs` — 81 `org_id/tenant_id` occurrences (dense, but comments/evidence/payments sub-resource reads called out by #2944 need per-handler review).
- Merged this window: #2942 (401 → single-flight refresh + one-shot replay), #2941 (JWT `exp` at cold-boot, 30s skew), #2943 (allowlist enforcement test), #2952 (auto-discover feature-local `*Keys` factories), #2950 (shared-device purge of NFC log / QR history / offline feedback).
- Open infra blockers: #2949 (api-server cloud verify), #2951 (mobile jest infra), #2652 (mobile-native/KMP), RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS).

## Return payload

```json
{
  "role": "pm-security",
  "summary": "Three open cross-tenant IDOR handlers (#2944/#2945/#2946) are the top security debt; all three are cloud-unlandable behind infra #2949 (utoipa-swagger-ui egress). Frontend auth hardening (single-flight refresh, JWT-exp cold-boot check, allowlist auto-derivation, shared-device cache purge) merged cleanly this window.",
  "next_actions": [
    {"action": "Fix IDOR #2946 — apply the computed `org_id` as a scope filter in portfolio_analytics `upsert_property_metrics` + `get_property_metrics` (lines 282, 314) and add a cross-tenant sqlx regression test asserting a caller in Org B is 404 on Org A's metric row", "priority": "high", "dependency": "pm-devops (infra #2949 must unblock api-server cloud verify OR route via local/ppt-bridge)", "definition_of_done": "handlers scope by `org_id`; new sqlx test in `portfolio_analytics_rls_cross_tenant_tests.rs` fails on pre-fix, passes on fix; issue #2946 closed"},
    {"action": "Fix IDOR #2945 — verify org ownership of the target `building_id`/property in every portfolio_properties read+write handler before acting; add sqlx cross-tenant test", "priority": "high", "dependency": "pm-devops (infra #2949)", "definition_of_done": "each handler resolves target `org_id` and rejects mismatched caller tenant; regression test in `portfolio_properties_rls_cross_tenant_tests.rs`"},
    {"action": "Fix IDOR #2944 — enforce `org_id` scoping on violations comments/evidence/payments read paths AND gate internal-notes visibility behind Manager/Owner role (deny for Tenant); add sqlx cross-tenant test + role-gate unit test", "priority": "high", "dependency": "pm-devops (infra #2949)", "definition_of_done": "all three sub-resource read handlers reject cross-tenant IDs (403/404); internal-notes only serialized for permitted roles; regression tests in `violation_subresource_rls_tests.rs`"},
    {"action": "Sweep for the `_org_id`/`_tenant_id` compute-then-discard anti-pattern across api-server (already-known sites: routes/portfolio_analytics.rs:282,314; routes/migration.rs:1472; routes/faults.rs:642). Convert each into a real scope check or remove if the underlying repo already scopes.", "priority": "high", "dependency": "none", "definition_of_done": "grep `let _org_id|let _tenant_id` returns only intentional-discard sites documented with a `// SAFETY:` comment explaining why the value is unused"},
    {"action": "Land infra #2949 remediation — vendor swagger-ui as an in-repo asset via `UTOIPA_SWAGGER_UI_DOWNLOAD_URL=file://...` (offline feature). Unblocks the three IDOR fixes above and every future api-server security patch from the cloud dispatcher.", "priority": "high", "dependency": "pm-devops", "definition_of_done": "`cargo build -p api-server` succeeds in the cloud sandbox with `HTTPS_PROXY` set; #2949 closed; IDOR trio auto-implementers can produce a PR"},
    {"action": "Follow-up regression paired with PR #2950: add a device-handoff integration test that logs in Tenant A, seeds each newly-covered namespace (`@ppt/access_log`, `@ppt/qr_scan_history`, `@ppt/feedback_drafts`, `@ppt/pending_feedback`, `@ppt/faq_votes`), logs in Tenant B, and asserts no read-back. Guards against regression when a new tenant-scoped cache is added without updating `TENANT_SCOPED_EXACT_KEYS`.", "priority": "medium", "dependency": "pm-qa", "definition_of_done": "test in `resetLocalData.handoff.test.ts` fails if any tracked key survives a session change"}
  ],
  "risks": [
    {"risk": "Cross-tenant data disclosure via three open IDOR handlers (portfolio_analytics property metrics, portfolio_properties, violations sub-resources + internal-notes leak) — public issues on the repo (#2944/#2945/#2946); a determined caller with two accounts can enumerate a competitor's rows today", "probability": "high", "impact": "high", "mitigation": "Unblock #2949 (vendor swagger-ui) and land the three sqlx-tested fixes as a security batch; label as `security` and drive through a manual reviewer slot"},
    {"risk": "The `_org_id` / `_tenant_id` compute-then-discard pattern is a systemic footgun — 3 confirmed sites; the linter can't catch it because `_`-prefixed unused bindings are intentionally allowed. New handlers may add more.", "probability": "medium", "impact": "high", "mitigation": "One-time grep sweep + a custom clippy lint or `just verify` grep gate that fails on `let _org_id|let _tenant_id` outside a documented allowlist"},
    {"risk": "Cloud pipeline cannot ship api-server security patches — infra #2949 (utoipa-swagger-ui) makes every api-server verify red; a zero-day today would require a human-driven local build", "probability": "high", "impact": "high", "mitigation": "Vendor swagger-ui OR route api-server builds through the ppt-bridge MCP (mirrors the mobile-native mitigation path)"},
    {"risk": "PR #2950 (shared-device cache leak fix, closes #2947) merged with red CI because infra #2951 (jest-expo/react-native version rot) breaks the mobile suite before this diff runs — the fix is verified only on IG3 pairing; a regression would not be caught by the mobile suite until the version rot is resolved", "probability": "medium", "impact": "medium", "mitigation": "Align jest-expo with the installed RN 0.87 or pin RN back to 0.85 per CLAUDE.md; regenerate lockfile; un-draft the PR flow. Add a smoke test that runs standalone in a Node context until the harness is fixed."},
    {"risk": "cargo-deny RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) is still open (carried from 2026-08-18); every backend PR runs with the advisory ignored via `.cargo-deny.toml` waiver, so a real DoS could ship if `h2` is not bumped promptly when a patched release lands", "probability": "medium", "impact": "medium", "mitigation": "Watch h2 tracking; bump to patched version and remove waiver as soon as available"}
  ],
  "open_questions": [
    "Are #2944/#2945/#2946 exploitable pre-authentication, or do they require a valid session in a second org? (The reconstructed issue text does not enumerate reproduction steps; the original edit history is where the specifics live — recommend a manual pull of the pre-edit version before scoring severity.)",
    "Does the shared-device purge (#2950) also need to cover the *native* iOS/Android keychain entries used by mobile-native (KMP), or is the fix scoped correctly to React-Native AsyncStorage?"
  ],
  "decisions_needed": [
    "Whether to vendor swagger-ui in-repo (fix #2949) vs. wait for an upstream `utoipa-swagger-ui` offline feature — owner: pm-devops",
    "Whether to route api-server security-fix implementer sessions through ppt-bridge MCP (host build) until #2949 is resolved — owner: pm-tech-lead"
  ]
}
```
