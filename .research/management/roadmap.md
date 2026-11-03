# PPT Roadmap

_Generated: 2026-11-03T20:36:04Z — upkeep mode (coverage_cursor re-checked epic-10a)._

## State of the project

- Stories: **49 done** / 0 partial / 0 not-started (49 total across 13 epics).
- All 13 epics in coverage are 100% done. No coverage-driven candidates remain to rank; refill must come from code-review findings and role `next_actions`.
- Screen coverage: 0 orphan epics · 0 orphan screens · 3 missing UC links.

## Ranked plan

Coverage is fully done. The ranked plan is the 24 open action-list items, sorted by priority (see `action-list.md`). Highlights below.

### phase: ongoing — code-review & test-gap findings

- [high] Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimabl — owner: `pm-devops`
- [high] Add a cross-boundary test pinning the three Rust `repositories/vote.rs` eligibility-reason literals against the mobile ` — owner: `pm-qa`
- [high] Resolve backend CI #3187 — either backend fix or test shape adjustment — so backend.yml goes green on dev and unblocks e — owner: `pm-qa`
- [high] Drive drafts #3224-#3227 through review to merge — they already sit as dispatcher assignments and address high-value cod — owner: `pm-scrum-master`
- [high] Resolve or close #3187 cross-tenant assign-404 contract failure (still carried, blocks backend CI) — either backend fix  — owner: `pm-scrum-master`
- [high] Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still retur — owner: `pm-scrum-master`
- [medium] KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded — owner: `pm-backend`
- [medium] mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on ever — owner: `pm-backend`
- [medium] Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) — owner: `pm-tech-lead`
- [medium] Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — owner: `pm-frontend`

Buffer: 24/36 open · 0 candidates ranked but unqueued
