# PPT Roadmap

_Generated 2026-06-26T03:25:00Z · scan_kind=upkeep · supersedes `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale)._

## State of the project

- **Stories:** 49 scanned across 13 epics — **37 done · 12 partial · 0 not-started**.
- **Partial work by platform:** backend 9 · ppt-web 9 · mobile 4 · frontend 2
- **Screen coverage:** 14 stories without screen-map · 2 orphan epics · 29 orphan screens · 4 missing UC links.

**Top 3 gaps (post 2026-06-16→2026-06-26 window):**
1. **Status-reconcile flood mostly cleared** — PRs #1832 (6-1), #1834 (6-2), #1835 (6-3), #1843 (6-4), #1844 (6-5), #1845 (80-2), #1830 (79-1), #1829 (10b-5), #1831 (10b-7) all reconciled. coverage.json itself was generated 2026-06-23 and is now stale on these resolutions; next deep scan will likely flip them to `done`.
2. **Genuinely unfinished slices** — 84-5 pgvector RAG retrieval/query service (migration only; PR #1833 quarantined); 80-3 mediation party-submission endpoints (PR #1846 in review); 79-2 e2e SSO/JWT/cookie sign-off; 6-3/6-4 mobile comments/pinned UI.
3. **Security follow-up debt** — 10 open `follow-up,from-merged-review` issues with IDOR/PII/JWT theme: #1791 (message attachment file_key IDOR), #1782 (3rd JWT verify copy), #1772 (OCR unauthenticated), #1766 (booking guest PII manager-gate), #1786/#1783/#1764/#1758/#1760/#1826. pm-security rotation prioritises these.

## Ranked plan

### mvp
- [high] no screen-map (orphan epic) (10b-5-support-data-access Support Data Access) — owner: pm-security — why: security/privacy sign-off · screen-map drift
- [high] sprint-status still ready-for-dev though code+PRs landed — needs status reconciliation to done (10b-5-support-data-access Support Data Access) — owner: pm-security — why: security/privacy sign-off · status-reconcile (code ahead of label)
- [high] support data access is privacy/security-sensitive: confirm retention + access-audit posture (pm-security) (10b-5-support-data-access Support Data Access) — owner: pm-security — why: security/privacy sign-off
- [high] coverage.json here still partial; gap-79-2-auth-callback-e2e tracked as open (full e2e verification remaining) (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: unfinished implementation
- [high] security-sensitive (SSO/JWT/cookie Path) — note for pm-security on final sign-off (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: security/privacy sign-off
- [high] story markdown frontmatter stale ('Status: pending') (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: unfinished implementation
- [high] sprint-status still ready-for-dev though backend+web+mobile code and PR #844 landed — needs reconciliation to done (10b-7-contextual-help-documentation Contextual Help & Documentation) — owner: pm-frontend — why: mobile slice · status-reconcile (code ahead of label)
- [high] announcements.md screen buildStatus=in-progress, not shipped (6-1-announcement-creation-targeting Announcement Creation & Targeting) — owner: pm-backend — why: screen-map drift
- [high] sprint-status still 'review' (not flipped to done) — verification/sign-off pending (6-1-announcement-creation-targeting Announcement Creation & Targeting) — owner: pm-backend — why: status-reconcile (code ahead of label)
- [high] announcements.md screen buildStatus=in-progress (6-2-announcement-viewing-acknowledgment Announcement Viewing & Acknowledgment) — owner: pm-frontend — why: screen-map drift
- [high] sprint-status ready-for-dev — code is ahead of label but story not marked done (6-2-announcement-viewing-acknowledgment Announcement Viewing & Acknowledgment) — owner: pm-frontend — why: status-reconcile (code ahead of label)
- [high] sprint-status ready-for-dev — code is well ahead of the label but story not marked done (6-5-direct-messaging Direct Messaging) — owner: pm-frontend — why: status-reconcile (code ahead of label)
- [high] coverage gap noted full feature-integration / e2e verification incomplete despite infra (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: unfinished implementation
- [high] coverage.json (last_checked 2026-05-23) still marks partial; promote commits not present on this worktree HEAD (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: unfinished implementation
- [high] no screen-map (orphan epic) (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: screen-map drift
- [high] story markdown frontmatter stale ('Status: pending') (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: unfinished implementation
- [medium] announcements.md screen buildStatus still in-progress (not shipped) (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: screen-map drift
- [medium] No mobile comments UI observed (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: mobile slice
- [medium] sprint-status ready-for-dev — not marked done (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: status-reconcile (code ahead of label)
- [medium] announcements.md screen buildStatus in-progress (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: screen-map drift
- [medium] No mobile pinned-band UI observed (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: mobile slice
- [medium] sprint-status ready-for-dev — not marked done (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: status-reconcile (code ahead of label)
- [medium] Localized `disputes.draft*` i18n keys for 6 message bundles are a documented fast-follow (currently English-default t(key, defaultValue)) (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: unfinished implementation
- [medium] Redesigned 5-step wizard (redesignStatus: in-progress) not shipped — only single-page form is live (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: unfinished implementation
- [medium] Mediation-resolution flow not promoted to done in sprint-status — story remains partial pending submissions integration (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — why: status-reconcile (code ahead of label)
- [medium] Party submissions endpoints unwired (apiStatus stays partial per dispute-detail screen-map) (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — why: screen-map drift

### phase3
- [medium] no screen-map (orphan epic) (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: screen-map drift
- [medium] No sprint-status entry and no test exercising vector similarity search (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: status-reconcile (code ahead of label)
- [medium] RAG retrieval/query service (embedding generation + similarity search) not implemented — migration only (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: unfinished implementation
- [medium] Vector path is conditional/optional (JSONB fallback) rather than a hard pgvector dependency (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: unfinished implementation

#### Screen-map drift
- [medium] Add screen-map(s) for orphan epic epic-85 — owner: pm-frontend — why: screen-map drift
- [medium] Add screen-map(s) for orphan epic epic-8a — owner: pm-frontend — why: screen-map drift
- [medium] Link UC UC-10 to a screen-map — owner: pm-frontend — why: screen-map drift
- [medium] Link UC UC-29 to a screen-map — owner: pm-frontend — why: screen-map drift
- [medium] Link UC UC-33 to a screen-map — owner: pm-frontend — why: screen-map drift
- [medium] Link UC UC-40 to a screen-map — owner: pm-frontend — why: screen-map drift
- [medium] Reconcile or remove orphan screen-map ppt/article-detail — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map ppt/emergency-contacts — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map ppt/faults-list — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map ppt/news — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map ppt/report-fault — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map ppt/vote-create — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map ppt/vote-detail — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map ppt/voting — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/about — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/agency-branding — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/agency-dashboard — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/agency-profile — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/agent-profile — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/careers — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/compare-properties — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/contact — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/for-agents — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/help — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/inquiries — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/journal — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/listing-edit — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality-mobile/agencies — owner: pm-frontend — why: mobile slice · status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality-mobile/auth-login — owner: pm-frontend — why: mobile slice · status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality-mobile/compare-listings — owner: pm-frontend — why: mobile slice · status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality-mobile/realtors — owner: pm-frontend — why: mobile slice · status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality-mobile/saved-searches — owner: pm-frontend — why: mobile slice · status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/price-map — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/saved-searches — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift
- [medium] Reconcile or remove orphan screen-map reality/sell — owner: pm-frontend — why: status-reconcile (code ahead of label) · screen-map drift

---
Buffer: 36/36 open · 43 candidates ranked but unqueued
