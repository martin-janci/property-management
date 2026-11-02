# PPT Action List

_Generated: 2026-11-02T00:40:57Z (routine Phase 1.6)_

**Open: 29** · Total: 29

| ID | Owner | Priority | Status | Action |
|---|---|---|---|---|
| `pm-devops-unblock-mobile-native-cloud-builds` | pm-devops | high | open | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimabl |
| `screen-map-drift-pr-2994-ppt` | pm-qa | low | open | ppt-web route refactor (PR #2994) landed without touching docs/screens/ppt |
| `code-review-api-handlers-violations-number-count-race` | pm-backend | low | open | violation_number / appeal_number generated via COUNT(*)+1 on the pool (no tx/sequence) collides with UNIQUE(org, number) |
| `gh-issue-3104` | pm-tech-lead | medium | open | Portfolio/owner analytics: comparisons + dashboard summary + owner portfolio runtime drift (500s) (Closes #3104) |
| `gh-issue-3116` | pm-tech-lead | medium | open | Follow-up: NewsScreen category badge lacks the i18n defaultValue fallback that ArticleDetailScreen uses (PR #3110) (Clos |
| `code-review-mobile-native-kmp-create-listing-not-wired` | pm-backend | medium | open | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded |
| `code-review-mobile-native-kmp-inquiries-response-contract` | pm-backend | medium | open | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on ever |
| `code-review-mobile-native-kmp-http-client-no-timeout` | pm-backend | low | open | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely |
| `code-review-mobile-native-kmp-my-listings-pagination-cap` | pm-backend | low | open | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) |
| `gh-issue-3164` | pm-tech-lead | medium | open | Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) |
| `pm-backend-violations-add-evidence-authz-gap-3164` | pm-backend | high | open | Gate violations add_evidence sub-resource with authz check — currently ungated allowing any authenticated user to attach |
| `churn-hotspot-backend/servers/api-server/tests/suites/violations_role_authz_tests.rs` | pm-qa | low | open | Churn hotspot: backend/servers/api-server/tests/suites/violations_role_authz_tests.rs — merged PR touched this file heav |
| `code-review-mobile-rn-vote-detail-dead-i18n-fb` | pm-frontend | low | open | VoteDetailScreen uses `t('key') ?? 'English'` throughout (L180-354) — i18next t() never returns nullish (returns the key |
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur |
| `code-review-api-handlers-appeal-no-status-guard` | pm-backend | low | open | api-handlers file_appeal has no status guard → owner can re-file after a final appeal decision |
| `code-review-api-handlers-moderation-report-flood` | pm-backend | low | open | api-handlers /moderation/report open-to-all + no dedup/rate-limit → unbounded case rows per content |
| `churn-hotspot-frontend/apps/mobile/src/screens/voting/VoteDetailScreen.test.tsx` | pm-qa | low | open | Churn hotspot: frontend/apps/mobile/src/screens/voting/VoteDetailScreen.test.tsx — ranked-ballot rendering tests reworke |
| `churn-hotspot-backend/servers/reality-server/src/routes/price_map.rs-retry1` | pm-tech-lead | low | open | Churn hotspot: backend/servers/reality-server/src/routes/price_map.rs — merged PR touched this file heavily this run [re |
| `pm-scrum-master-triage-stalled-prs-2026-11-02` | pm-scrum-master | high | open | Triage the stalled PRs #3075 (violations number race), #3108 (owner-analytics 500), #2555 and #2559 (accounting invoice  |
| `pm-scrum-master-refill-signal-scans-2026-11-02` | pm-scrum-master | high | open | Run fresh per-package code-review signal scans and a default-mode re-rank to refill the action buffer (claimable=7/72) |
| `pm-scrum-master-moderation-appeal-fix-2026-11-02` | pm-scrum-master | medium | open | Fix file_appeal status guard and rate-limit/dedup /moderation/report (addresses two score-2 open backlog items) |
| `pm-scrum-master-ktor-http-timeout-2026-11-02` | pm-scrum-master | medium | open | Add HttpTimeout to the shared Ktor client in mobile-native (resolves code-review-mobile-native-kmp-http-client-no-timeou |
| `pm-scrum-master-refresh-sprint-status-2026-11-02` | pm-scrum-master | low | open | Refresh sprint-status.yaml sprint name/goal (still describes Dec-2025 Epic 6/7A/8A/10A sprint); reconcile test_hardening |
| `pm-qa-reconcile-test-hardening-batch-2026-11-02` | pm-qa | high | open | Reconcile test_hardening_batch in sprint-status.yaml: mark #480, #483 and #484 closed (GitHub, 2026-05-26) and set the b |
| `pm-qa-kmp-inquiries-contract-tests-2026-11-02` | pm-qa | high | open | Add contract tests for code-review-mobile-native-kmp-inquiries-response-contract before merging its fix (KMP test decode |
| `pm-qa-negative-tests-flood-appeal-timeout-2026-11-02` | pm-qa | medium | open | Add negative state-transition tests (moderation-report-flood, appeal-no-status-guard) and a Ktor timeout test (http-clie |
| `pm-qa-violations-role-endpoint-matrix-2026-11-02` | pm-qa | medium | open | Audit violations_role_authz_tests.rs and violations_cross_org_idor_tests.rs for a full role x endpoint matrix; confirm # |
| `pm-qa-mobile-locale-flake-rerun-2026-11-02` | pm-qa | medium | open | Before merging #3213 and #3208, run locale-key completeness check across the 6 locales and re-run the stabilized VoteDet |
