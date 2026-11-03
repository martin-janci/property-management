# pm-scrum-master

_Generated: 2026-11-03T20:36:04Z_

## Summary

Heaviest shipping day since mid-October — 7 PRs merged (#3216-3222), all dispatcher-cleared post-merge-review findings. Buffer dropped to claimable=8/72 and coverage is drained (49/49 stories done across 13 epics), so refill must come from the code-review signal stream, not coverage ranking.

## Shipped since last run
- #3216 screen-map reconcile reality/account-listing-edit
- #3217 reality-web listing-edit numeric coercion (+_number.ts helper + 7 tests, extracts page.tsx churn)
- #3218 ppt-web EvidenceUploader i18n + 6-locale key-presence test suite
- #3219 reality-server price_map.rs LIKE-metachar escape (security — anonymous endpoint, with regression test)
- #3220 ppt-web community-feature i18n (17 components/pages + 6 locales + 1 i18n-key test)
- #3221 mobile VoteDetailScreen branch-test snapshot decouple (removes shared-style churn trap)
- #3222 mobile VoteDetailScreen eligibility reason i18n + regression test — spawned follow-up issue #3223

## Sprint progress
- Sprint: **Epic 6, 7A, 8A & 10A - Announcements, Documents, Notifications & OAuth**
- Epics: 4/4

## Next actions
- [medium] **Decide the stable reason-code contract for backend vote.rs eligibility reasons (issue #3223) — English-free-text vs enum/code; interim is a cross-boundary test, long-term is a breaking DTO change** — dep: `pm-backend` — DoD: Decision recorded in decisions.md; either a cross-boundary test lands pinning the three literals or an enum/code migration is scheduled
- [high] **Drive drafts #3224-#3227 through review to merge — they already sit as dispatcher assignments and address high-value code-review items (reality-web mock profile, feed silent-fail, inquiry i18n, violations add_evidence gating)** — dep: `none` — DoD: All four drafts either merged or converted to issue+closed; dispatcher assignments.json transitions out of `review`
- [high] **Resolve or close #3187 cross-tenant assign-404 contract failure (still carried, blocks backend CI) — either backend fix or test shape adjustment** — dep: `pm-security` — DoD: Issue closed; backend.yml green on dev with cross_org_assign_violation_is_not_found passing
- [medium] **Refill the planner buffer — coverage is at 49/49 done so ranked-from-coverage yields zero candidates. Add follow-ups + role next_actions this run, and consider sourcing more candidates from post-merge review findings** — dep: `none` — DoD: action-list.json open_count rises above 18 (half the 36 target)
- [medium] **Verify PR #3220 community i18n coverage — 17 files / 3563 LOC with only one i18n-key test; spot-check representative keys render in a locale-switch test (CreateGroupPage / MarketplacePage / EventCard)** — dep: `pm-qa` — DoD: At least one locale-switching smoke test + a key-shape test lands for community pages

## Risks
- [high/medium] **Buffer drained-coverage pattern will keep recurring now that all 13 epics are done — refill depends on dispatcher continuously surfacing new review findings, which is not contractually guaranteed** — mitigation: Add a staleness alert: if open_count < 18 for 3 consecutive runs, surface in brief under `Goal violations` as a planning health signal
- [high/medium] **pm-qa rotation 70 days stale — the active-sprint QA slice has not been reviewed since 2026-08-25 even though backend CI has had a failing test (#3187) for ~6 days** — mitigation: Mandatory pm-qa run today — add staleness trigger in routine prompt (`last_run > 30d` forces rotation)

## Blockers
- **#3187 cross_org_assign_violation_is_not_found** — assign returns 403 (role-gate) instead of expected 404 (cross-tenant) — fails backend CI on dev — owner: `pm-security`
- **#3168 cargo-deny RUSTSEC advisories** — every backend PR merge gated — carried — owner: `pm-security`
- **#3164 violations add_evidence authz gap** — add_evidence sub-route still ungated — PR #3227 open (draft) — owner: `pm-backend`
- **#3131 merge pipeline deadlock** — carried — merge throughput limited — owner: `pm-backend`
- **#2652 mobile-native/KMP cloud builds** — 5+ KMP backlog items structurally unclaimable in cloud — carried — owner: `pm-devops`
- **#2951 mobile RN jest-expo version rot** — blocks RN test harness upgrades — carried — owner: `pm-devops`

## Open questions
- Should the backend vote.rs eligibility reason return stable reason codes (enum) as a DTO contract, or keep the English free-text sentences with cross-boundary tests? This gates issue #3223.

## Decisions needed
- Eligibility-reason contract: stable codes vs. free-text + cross-boundary test — owner: pm-backend
