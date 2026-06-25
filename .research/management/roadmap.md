# PPT Roadmap

_Generated 2026-06-25T03:30:00Z · scan_kind=upkeep · supersedes prior deep scan (2026-06-23)._

## State of the project

- **Stories:** 49 scanned across 13 epics — **38 done · 11 partial · 0 not-started**.
- **Partial work by platform:** backend 8 · ppt-web 8 · mobile 4 · frontend 2
- **Screen coverage:** 14 stories without screen-map · 2 orphan epics · 29 orphan screens · 4 missing UC links.

**Top 3 gaps:**
1. **Promotion lag, not missing code** — ~6-7 stories (6-1..6-5, 79-1, 10b-7) code-complete with merged PRs but stuck at sprint-status `ready-for-dev`/`review`; need status reconciliation + final sign-off.
2. **Genuinely unfinished slices** — 84-5 pgvector RAG retrieval/query service (migration only), 80-3 mediation party-submission endpoints (unwired), 80-2 dispute redesign 5-step wizard + i18n, 6-3/6-4 mobile comments/pinned UI, 79-2 final security sign-off.
3. **Three live release-blocking security gaps surfaced by pm-security rotation: unauthenticated /api/v1/ai/ocr/* endpoints, OAuth refresh-token revocation bypass (#481), third JwtService copy unmigrated (#1782).** — Three live release-blocking security gaps surfaced by pm-security rotation: unauthenticated /api/v1/ai/ocr/* endpoints, OAuth refresh-token revocation bypass (#481), third JwtService copy unmigrated (#1782).

## Ranked plan

### mvp
- [high] security-sensitive (SSO/JWT/cookie Path) — note for pm-security on final sign-off (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: security/privacy sign-off
- [high] sprint-status still 'review' (not flipped to done) — verification/sign-off pending (6-1-announcement-creation-targeting Announcement Creation & Targeting) — owner: pm-backend — why: promotion lag — sprint-status reconciliation · mobile slice
- [high] announcements.md screen buildStatus=in-progress, not shipped (6-1-announcement-creation-targeting Announcement Creation & Targeting) — owner: pm-backend — why: mobile slice
- [high] sprint-status ready-for-dev — code is ahead of label but story not marked done (6-2-announcement-viewing-acknowledgment Announcement Viewing & Acknowledgment) — owner: pm-frontend — why: promotion lag — sprint-status reconciliation · mobile slice
- [high] announcements.md screen buildStatus=in-progress (6-2-announcement-viewing-acknowledgment Announcement Viewing & Acknowledgment) — owner: pm-frontend — why: mobile slice
- [high] sprint-status ready-for-dev — code is well ahead of the label but story not marked done (6-5-direct-messaging Direct Messaging) — owner: pm-frontend — why: promotion lag — sprint-status reconciliation · mobile slice
- [high] sprint-status still ready-for-dev though backend+web+mobile code and PR #844 landed — needs reconciliation to done (10b-7-contextual-help-documentation Contextual Help & Documentation) — owner: pm-frontend — why: promotion lag — sprint-status reconciliation · mobile slice
- [medium] sprint-status ready-for-dev — not marked done (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: promotion lag — sprint-status reconciliation
- [medium] announcements.md screen buildStatus still in-progress (not shipped) (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: unfinished implementation
- [medium] No mobile comments UI observed (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: unfinished implementation
- [medium] sprint-status ready-for-dev — not marked done (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: promotion lag — sprint-status reconciliation
- [medium] No mobile pinned-band UI observed (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: unfinished implementation
- [medium] announcements.md screen buildStatus in-progress (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: unfinished implementation
- [medium] coverage.json (last_checked 2026-05-23) still marks partial; promote commits not present on this worktree HEAD (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: unfinished implementation
- [medium] coverage gap noted full feature-integration / e2e verification incomplete despite infra (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: unfinished implementation
- [medium] story markdown frontmatter stale ('Status: pending') (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: unfinished implementation
- [medium] no screen-map (orphan epic) (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: unfinished implementation
- [medium] story markdown frontmatter stale ('Status: pending') (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: unfinished implementation
- [medium] Redesigned 5-step wizard (redesignStatus: in-progress) not shipped — only single-page form is live (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: unfinished implementation
- [medium] Localized `disputes.draft*` i18n keys for 6 message bundles are a documented fast-follow (currently English-default t(key, defaultValue)) (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: unfinished implementation
- [medium] Party submissions endpoints unwired (apiStatus stays partial per dispute-detail screen-map) (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — why: unfinished implementation
- [medium] Mediation-resolution flow not promoted to done in sprint-status — story remains partial pending submissions integration (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — why: promotion lag — sprint-status reconciliation

#### Screen-map drift
- [medium] Add screen-map(s) for orphan epic epic-85 — owner: pm-frontend — why: screen-gap (orphan epic)
- [medium] Add screen-map(s) for orphan epic epic-8a — owner: pm-frontend — why: screen-gap (orphan epic)
- [medium] Link UC UC-10 to a screen-map — owner: pm-frontend — why: screen-gap (UC not linked)
- [medium] Link UC UC-29 to a screen-map — owner: pm-frontend — why: screen-gap (UC not linked)
- [medium] Link UC UC-33 to a screen-map — owner: pm-frontend — why: screen-gap (UC not linked)
- [medium] Link UC UC-40 to a screen-map — owner: pm-frontend — why: screen-gap (UC not linked)

### phase3
- [medium] RAG retrieval/query service (embedding generation + similarity search) not implemented — migration only (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: unfinished implementation
- [medium] Vector path is conditional/optional (JSONB fallback) rather than a hard pgvector dependency (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: unfinished implementation
- [medium] No sprint-status entry and no test exercising vector similarity search (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: promotion lag — sprint-status reconciliation
- [medium] no screen-map (orphan epic) (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: unfinished implementation

---
Buffer: 36/36 open · 32 candidates ranked but unqueued