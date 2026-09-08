# Action list

_Generated: 2026-09-08 — regenerated from `action-list.json`. Adds 7 rows this run (pm-security IDOR trio + swagger-ui unblock + antipattern sweep + jest infra + handoff test + h2 RUSTSEC)._

| ID | Priority | Owner | Action |
|---|---|---|---|
| `pm-devops-vendor-swagger-ui-unblock-api-server-cloud-build` | high | pm-devops | Land infra #2949 — vendor swagger-ui in-repo so `cargo build -p api-server` succeeds in the cloud sandbox. Unblocks the IDOR trio. |
| `pm-security-fix-idor-2946-portfolio-analytics-org-id-discarded` | high | pm-security | Fix IDOR #2946 — apply computed `org_id` as scope filter in portfolio_analytics `upsert_property_metrics` + `get_property_metrics` (routes/portfolio_analytics.rs:282,314). |
| `pm-security-fix-idor-2945-portfolio-properties-missing-org-verification` | high | pm-security | Fix IDOR #2945 — verify org ownership of target property/`building_id` in every portfolio_properties handler. |
| `pm-security-fix-idor-2944-violations-subresource-scoping-and-internal-notes-role-gate` | high | pm-security | Fix IDOR #2944 — org-scope violations comments/evidence/payments reads; role-gate internal-notes visibility. |
| `pm-security-sweep-org-id-compute-then-discard-antipattern` | high | pm-security | Sweep `_org_id` / `_tenant_id` compute-then-discard sites in api-server; add `just verify` grep gate. |
| `pm-security-close-rustsec-2026-0258-h2-dos` | high | pm-security | Bump h2 to patched release and remove `.cargo-deny.toml` waiver (RUSTSEC-2026-0258). Standing since 2026-08-18. |
| `pm-devops-unblock-mobile-native-cloud-builds` | high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — 6/9 open items are structurally unclaimable. |
| `pm-devops-unblock-mobile-jest-suite-issue-2951` | medium | pm-devops | Align jest-expo/RN version rot (#2951) so mobile jest loads in cloud; PR #2950 merged red-CI on this. |
| `pm-qa-shared-device-cache-purge-handoff-regression-test` | medium | pm-qa | Device-handoff regression test for PR #2950 (`TENANT_SCOPED_EXACT_KEYS` future-proofing). |
| `code-review-mobile-native-kmp-inquiries-response-contract` | medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit`. |
| `code-review-mobile-native-kmp-portfolio-analytics-caps-100` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings. |
| `code-review-mobile-native-kmp-portfolio-analytics-unbounded-fanout-retry1` | low | pm-backend | mobile-native-kmp: getPortfolioAnalytics() unbounded fan-out — up to 100 parallel GETs. |
| `code-review-mobile-native-kmp-cancellation-swallowed` | low | pm-backend | mobile-native-kmp: shared repositories swallow CancellationException. |
| `code-review-mobile-native-kmp-ssoservice-untested` | low | pm-qa | mobile-native-kmp: SsoService has zero direct tests. |
| `code-review-mobile-native-kmp-httpclient-no-timeout` | low | pm-backend | mobile-native-kmp shared Ktor HttpClient installs no HttpTimeout. |
| `code-review-mobile-native-kmp-create-listing-not-wired` | low | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub. |
| `screen-map-drift-pr-2894-reality` | low | pm-qa | screen-map-drift: PR #2894 touched reality-web routes without updating docs/screens. |
