# PPT Action List

_Generated: 2026-08-08 · 18 items · 14 open / 3 in-progress / 1 done (buffer target: 36 — below-half; scan recommended)_

| Priority | Status | Owner | ID | Action | Dep | Source |
|---|---|---|---|---|---|---|
| high | open | pm-qa | pm-qa-sequence-lock-2696-inquiry-notify | Sequence-lock: hold #2696 (inquiry-email notifier seam) merge until code-review-reality-server-inquiry-notify-route-wiring lands together — solo merge ships silent-success | code-review-reality-server-inquiry-notify-route-wiring | pm-analysis 2026-08-08 (pm-qa rotation) |
| high | open | pm-qa | pm-qa-hotfix-no-test-backfill-2707-2712 | Backfill regression tests for the two 2026-08-07 hotfix-no-test slips: PR #2707 (body-cap) and PR #2712 (dispute add_evidence audit event) | none | pm-analysis 2026-08-08 (pm-qa rotation) |
| high | open | pm-qa | pm-qa-ssrf-toctou-2710-resolver-spoof-test | Prove SSRF DNS-rebinding TOCTOU fix (draft PR #2710, closes #2703) with a spoofed-resolver regression test | gh-issue-2703 | pm-analysis 2026-08-08 (pm-qa rotation) |
| high | in-progress | pm-tech-lead | gh-issue-2703 | SSRF DNS-rebinding TOCTOU in workflow action api_call.rs — anti-rebinding gate is bypassable (Closes #2703) | none | dispatcher-issue-ingest 2026-08-07T08:10:30Z (#2703); upgraded medium→high 2026-08-08 |
| medium | open | pm-qa | pm-qa-vendor-utoipa-swagger-ui-zip | Vendor / cache the utoipa-swagger-ui zip so cargo test / clippy for api-server can run locally in the sandbox | pm-devops | pm-analysis 2026-08-08 (pm-qa rotation) |
| medium | open | pm-qa | pm-qa-announce-fanout-real-sql-test | Replace pure-Rust re-model announce fan-out test with an sqlx::test hitting the real RLS predicate — closes risk / gh-issue-2484 | gh-issue-2484 | pm-analysis 2026-08-08 (pm-qa rotation) |
| medium | open | pm-backend | code-review-reality-server-inquiry-notify-route-wiring | reality-server: live send_contact_message endpoint bypasses the InquiriesHandler seam from PR #2696 — wire it through | code-review-reality-server-inquiry-email-stub | implementer-secondary-finding PR#2696 2026-08-06T22:36:16Z |
| medium | in-progress | pm-tech-lead | gh-issue-2612-retry1 | Follow-up: scheduled announcement / vote-started notifications are fire-once — decouple dispatch (Closes #2612) [retry 1/2] | none | dispatcher-retry-remint 2026-08-08T00:08:13Z; PR #2714 opened draft 2026-08-08 |
| low | open | pm-backend | code-review-mobile-native-kmp-portfolio-analytics-caps-100 | mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings | none | dispatcher-backlog-refill 2026-08-04T04:05:19Z |
| low | in-progress | pm-backend | code-review-api-core-workflow-cond-parse-failopen | evaluate_conditions() FAILS OPEN on unparseable condition — fail closed | none | dispatcher-signal-bridge 2026-08-05T22:12:57Z; PR #2684 ready-to-merge 2026-08-08 |
| low | in-progress | pm-backend | code-review-reality-server-inquiry-email-stub | reality-server: shipped-but-non-functional notification path (send_inquiry_notification stub) | none | dispatcher-backlog-refill 2026-08-06T20:08:18Z; PR #2696 ready-to-merge 2026-08-08 — BLOCKED solo merge |
| low | in-progress | pm-scrum-master | chore-triage-untriaged-issues-750s-2026-07-23-retry2 | Bulk-triage untriaged issues #749-#779 [retry 2/2] | none | dispatcher-retry-remint 2026-08-08T00:08:13Z |
| low | in-progress | pm-devops | dx-fixme-admin-web-mobile-config-patch-endpoint-retry1 | admin-web mobile-config Save flow blocked: PATCH endpoint missing [retry 1/2] | none | dispatcher-retry-remint 2026-08-08T04:07:24Z |
| low | in-progress | pm-devops | dx-fixme-admin-web-platform-settings-patch-endpoint-retry1 | admin-web platform-settings Save blocked: PATCH endpoint missing [retry 1/2] | none | dispatcher-retry-remint 2026-08-08T04:07:24Z |
| low | in-progress | pm-tech-lead | refactor-churn-hotspot-api-server-auth-2026-07-31-retry1 | Churn hotspot: routes/auth.rs — 2950 lines / runs_seen=5 [retry 1/2] | none | dispatcher-retry-remint 2026-08-08T04:07:24Z |
| low | open | pm-tech-lead | refactor-churn-hotspot-api-server-reports-2026-07-31-retry1 | Churn hotspot: routes/reports.rs — 3329 lines [retry 1/2] | none | dispatcher-retry-remint 2026-08-08T04:07:24Z |
| low | open | pm-scrum-master | pm-qa-sprint-yaml-reconcile-coverage | Reconcile sprint-status.yaml against coverage.json: flip epics 6/7a/10b/80 in-progress → done | none | pm-analysis 2026-08-08 (pm-qa rotation) |
| low | done | pm-backend | code-review-reality-web-listingform-no-i18n | reality-web ListingForm hardcodes English throughout a next-intl app — closed by PR #2709 | none | dispatcher-backlog-refill 2026-08-07T04:07:19Z |
