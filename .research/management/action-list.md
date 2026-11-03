# PPT Action List

_Generated: 2026-11-03T14:44:03Z (routine Phase 1.6)_

**Open: 22** · Total: 22

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
| `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` | pm-frontend | medium | open | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) |
| `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` | pm-scrum-master | high | open | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur |
| `code-review-api-handlers-moderation-report-flood` | pm-backend | low | open | api-handlers /moderation/report open-to-all + no dedup/rate-limit → unbounded case rows per content |
| `repeated-churn-frontend/apps/mobile/src/screens/voting/VoteDetailScreen.test.tsx` | pm-tech-lead | low | open | Repeated churn: mobile-rn VoteDetailScreen.test.tsx — runs_seen 1→2 (PR #3189 then #3209) |
| `code-review-mobile-rn-vote-list-invalid-start-date` | pm-backend | low | open | mobile VotingScreen renders "Invalid Date" as every vote card's start date |
| `code-review-mobile-rn-vote-quorum-nan-width` | pm-backend | low | open | mobile VotingScreen quorum bar width becomes NaN% when eligible_count is null/0 |
| `code-review-mobile-rn-vote-eligibility-reason-i18n` | pm-backend | low | open | mobile VoteDetailScreen renders server eligibility.reason string raw, bypassing i18n |
| `code-review-reality-web-agent-profile-mock-data` | pm-backend | low | open | reality-web agent public profile (/realtor/[id]) renders hardcoded MOCK_AGENT for every id and its contact form fakes su |
| `code-review-reality-web-feed-sync-silent-fail` | pm-backend | low | open | reality-web FeedImport 'Sync now' fires useSyncFeedSource mutation with no error handling — sync failures are silent |
| `code-review-reality-web-inquiry-type-untranslated` | pm-tech-lead | low | open | reality-web inquiries page renders raw inquiry.type enum via .replace('_',' ') — untranslated for sk/cs/de users |
| `pm-qa-unwrap-or-default-sweep-2026-11-03` | pm-qa | medium | open | Lint sweep: deny clippy::unwrap_or_default across backend/servers/*/src/routes crates (3 fresh instances surfaced this r |
