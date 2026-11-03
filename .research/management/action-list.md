# PPT Action List

_Generated: 2026-11-03T20:36:04Z_

| Priority | Owner | Action | Dep | DependsOn | Status | Source |
|---|---|---|---|---|---|---|
| high | pm-devops | Unblock mobile-native/KMP builds in the cloud runner (issue #2652) — still 5+ open backlog items structurally unclaimable in cloud; now comp | none | — | open | pm-analysis 2026-08-31 (carried) |
| high | pm-qa | Add a cross-boundary test pinning the three Rust `repositories/vote.rs` eligibility-reason literals against the mobile `ELIGIBILITY_REASON_K | none | — | open | pm-analysis 2026-11-03 |
| high | pm-qa | Resolve backend CI #3187 — either backend fix or test shape adjustment — so backend.yml goes green on dev and unblocks every backend PR | none | — | open | pm-analysis 2026-11-03 |
| high | pm-scrum-master | Drive drafts #3224-#3227 through review to merge — they already sit as dispatcher assignments and address high-value code-review items (real | none | — | open | pm-analysis 2026-11-03 |
| high | pm-scrum-master | Resolve or close #3187 cross-tenant assign-404 contract failure (still carried, blocks backend CI) — either backend fix or test shape adjust | none | — | open | pm-analysis 2026-11-03 |
| high | pm-scrum-master | Sweep remaining violations mutation handlers for the 403-vs-404 cross-tenant contract (#3187 revealed assign still returns 403) — pattern au | pm-security | UNRESOLVED:pm-security | open | pm-analysis 2026-10-28 |
| medium | pm-backend | KMP realtor CreateListingScreen onSubmit is a NotImplementedError stub — form data discarded | — | — | open | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-backend | mobile-native-kmp InquiriesResponse required page_size mismatches reality-server `limit` — MissingFieldException on every real /inquiries +  | — | — | open | dispatcher-backlog-refill 2026-10-21T22:25:48Z (score=3 conf=high vector=bug) |
| medium | pm-tech-lead | Follow-up: violations add_evidence left ungated while delete_evidence is manager-gated (PR #3072) (Closes #3164) | — | — | open | dispatcher-issue-ingest 2026-10-25T22:26:31Z (#3164) |
| medium | pm-frontend | Adopt eslint-plugin-react-hooks + i18next/no-literal-string in frontend/apps/mobile (blocking, warn-only for one sprint) — closes the chroni | pm-qa | UNRESOLVED:pm-qa | open | pm-analysis 2026-10-28 |
| medium | pm-qa | Add at least one locale-switching smoke test for the community pages (ppt-web EventCard or MarketplacePage) — PR #3220 ships 3563 LOC with o | none | — | open | pm-analysis 2026-11-03 |
| medium | pm-qa | Audit the four open draft PRs (#3224-#3227) for regression tests on the fix path before merge — three of them target silent-UX-failure bugs  | none | — | open | pm-analysis 2026-11-03 |
| medium | pm-qa | Extend PR #3222's VoteDetailScreen.test.tsx eligibility-reason branch to cover all three mapped reasons (today only `voteNotAcceptingBallots | none | — | open | pm-analysis 2026-11-03 |
| medium | pm-qa | Resolve #2951 jest-expo version rot to unlock mobile RN test harness upgrades (carried) | none | — | open | pm-analysis 2026-11-03 |
| medium | pm-scrum-master | Decide the stable reason-code contract for backend vote.rs eligibility reasons (issue #3223) — English-free-text vs enum/code; interim is a  | none | — | open | pm-analysis 2026-11-03 |
| medium | pm-scrum-master | Refill the planner buffer — coverage is at 49/49 done so ranked-from-coverage yields zero candidates. Add follow-ups + role next_actions thi | none | — | open | pm-analysis 2026-11-03 |
| medium | pm-scrum-master | Verify PR #3220 community i18n coverage — 17 files / 3563 LOC with only one i18n-key test; spot-check representative keys render in a locale | none | — | open | pm-analysis 2026-11-03 |
| low | pm-backend | Shared Ktor HttpClient has no HttpTimeout - mobile API calls can hang indefinitely | — | — | open | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=2 conf=medium vector=bug) |
| low | pm-backend | Reality mobile MyListings + portfolio analytics silently cap at 100 listings (no pagination) | — | — | open | dispatcher-backlog-refill 2026-10-25T08:29:08Z (score=1 conf=medium vector=bug) |
| low | pm-backend | mobile VotingScreen renders "Invalid Date" as every vote card's start date | — | — | open | dispatcher-backlog-refill 2026-11-03T06:25:58Z (score=2 conf=medium vector=bug) |
| low | pm-backend | mobile VotingScreen quorum bar width becomes NaN% when eligible_count is null/0 | — | — | open | dispatcher-backlog-refill 2026-11-03T06:25:58Z (score=2 conf=medium vector=bug) |
| low | pm-backend | reality-web agent public profile (/realtor/[id]) renders hardcoded MOCK_AGENT for every id and its contact form fakes submit (no API call) | — | — | open | dispatcher-backlog-refill 2026-11-03T08:26:12Z (score=2 conf=medium vector=bug) |
| low | pm-backend | reality-web FeedImport 'Sync now' fires useSyncFeedSource mutation with no error handling — sync failures are silent | — | — | open | dispatcher-backlog-refill 2026-11-03T08:26:12Z (score=2 conf=medium vector=bug) |
| low | pm-tech-lead | reality-web inquiries page renders raw inquiry.type enum via .replace('_',' ') — untranslated for sk/cs/de users | — | — | open | dispatcher-backlog-refill 2026-11-03T08:26:12Z (score=1 conf=medium vector=i18n) |
