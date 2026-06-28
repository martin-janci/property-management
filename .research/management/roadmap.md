# PPT Roadmap

_Generated 2026-06-28T03:25:00Z · scan_kind=upkeep · supersedes `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale)._

## State of the project

- **Stories:** 49 scanned across 13 epics — **37 done · 12 partial · 0 not-started**.
- **Partial/not-started by platform:** backend 9 · ppt-web 9 · mobile 4 · frontend 2
- **Screen coverage:** 14 stories without screen-map · 2 orphan epics · 29 orphan screens · 4 missing UC links.

**Top 3 gaps:**
1. **Epic 6 closeout + 8a screen-maps shipped this run** — old promotion-lag gaps (6-1…6-5, 10b-7) cleared via #1832-#1844 cluster + #1907-#1918. Remaining `partial` set is mostly 80-x dispute work and orphan-epic screen-map drift.
2. **OAuth 10A gated by security tests** — 10a-1/10a-3 stay `partial` until #481 (revocation replay test) and #480 (WS token logs) close; pm-security flagged both as release blockers.
3. **Systemic screen-map drift** — screen-maps still don't populate frontmatter `epics:` field, manufacturing the orphan_screens noise. Backfilling `epics:` remains the highest-leverage one-shot cleanup.

## Ranked plan

### mvp

- [high] sprint-status still ready-for-dev though code+PRs landed — needs status reconciliation to done (10b-5-support-data-access Support Data Access) — owner: pm-security — why: finish-what's-started; security risk; no screen-map
- [high] support data access is privacy/security-sensitive: confirm retention + access-audit posture (pm-security) (10b-5-support-data-access Support Data Access) — owner: pm-security — why: finish-what's-started; security risk; no screen-map
- [high] no screen-map (orphan epic) (10b-5-support-data-access Support Data Access) — owner: pm-security — why: finish-what's-started; security risk; no screen-map
- [high] coverage.json here still partial; gap-79-2-auth-callback-e2e tracked as open (full e2e verification remaining) (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: finish-what's-started
- [high] security-sensitive (SSO/JWT/cookie Path) — note for pm-security on final sign-off (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: finish-what's-started
- [high] sprint-status still ready-for-dev though backend+web+mobile code and PR #844 landed — needs reconciliation to done (10b-7-contextual-help-documentation Contextual Help & Documentation) — owner: pm-frontend — why: finish-what's-started; mobile-lag
- [high] sprint-status still 'review' (not flipped to done) — verification/sign-off pending (6-1-announcement-creation-targeting Announcement Creation & Targeting) — owner: pm-backend — why: finish-what's-started; mobile-lag
- [high] announcements.md screen buildStatus=in-progress, not shipped (6-1-announcement-creation-targeting Announcement Creation & Targeting) — owner: pm-backend — why: finish-what's-started; mobile-lag
- [high] sprint-status ready-for-dev — code is ahead of label but story not marked done (6-2-announcement-viewing-acknowledgment Announcement Viewing & Acknowledgment) — owner: pm-frontend — why: finish-what's-started; mobile-lag
- [high] announcements.md screen buildStatus=in-progress (6-2-announcement-viewing-acknowledgment Announcement Viewing & Acknowledgment) — owner: pm-frontend — why: finish-what's-started; mobile-lag
- [high] sprint-status ready-for-dev — code is well ahead of the label but story not marked done (6-5-direct-messaging Direct Messaging) — owner: pm-frontend — why: finish-what's-started; mobile-lag
- [high] coverage.json (last_checked 2026-05-23) still marks partial; promote commits not present on this worktree HEAD (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: finish-what's-started; no screen-map
- [high] coverage gap noted full feature-integration / e2e verification incomplete despite infra (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: finish-what's-started; no screen-map
- [high] story markdown frontmatter stale ('Status: pending') (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: finish-what's-started; no screen-map
- [high] no screen-map (orphan epic) (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: finish-what's-started; no screen-map
- [medium] sprint-status ready-for-dev — not marked done (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: finish-what's-started
- [medium] announcements.md screen buildStatus still in-progress (not shipped) (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: finish-what's-started
- [medium] No mobile comments UI observed (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: finish-what's-started
- [medium] sprint-status ready-for-dev — not marked done (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: finish-what's-started
- [medium] No mobile pinned-band UI observed (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: finish-what's-started

#### Screen-map drift

- [high] Add screen-map(s) for orphan epic epic-85 — owner: pm-frontend — why: screen-map drift
- [high] Add screen-map(s) for orphan epic epic-8a — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map ppt/article-detail — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map ppt/emergency-contacts — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map ppt/faults-list — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map ppt/news — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map ppt/report-fault — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map ppt/vote-create — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map ppt/vote-detail — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map ppt/voting — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map reality-mobile/agencies — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map reality-mobile/auth-login — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map reality-mobile/compare-listings — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map reality-mobile/realtors — owner: pm-frontend — why: screen-map drift
- [high] Reconcile or remove orphan screen-map reality-mobile/saved-searches — owner: pm-frontend — why: screen-map drift

### phase3

- [medium] RAG retrieval/query service (embedding generation + similarity search) not implemented — migration only (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: finish-what's-started; no screen-map
- [medium] Vector path is conditional/optional (JSONB fallback) rather than a hard pgvector dependency (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: finish-what's-started; no screen-map
- [medium] No sprint-status entry and no test exercising vector similarity search (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: finish-what's-started; no screen-map
- [medium] no screen-map (orphan epic) (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: finish-what's-started; no screen-map

Buffer: 28/36 open · 30 candidates ranked but unqueued
