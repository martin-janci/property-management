# PPT Roadmap

_Generated 2026-06-23T10:02:02Z · scan_kind=deep · supersedes `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86, stale)._

## State of the project

- **Stories:** 49 scanned across 13 epics — **37 done · 12 partial · 0 not-started**.
- **Partial work by platform:** backend 9 · ppt-web 9 · mobile 4 · frontend 2
- **Screen coverage:** 14 stories without screen-map · 2 orphan epics · 29 orphan screens · 4 missing UC links.

**Top 3 gaps:**
1. **Promotion lag, not missing code** — ~8 stories (6-1…6-5, 79-1, 10b-5, 10b-7) are code-complete (backend+web, often mobile) with merged PRs but stuck at sprint-status `ready-for-dev`/`review`. They inflate the `partial` count; most need a status reconciliation + final sign-off, not new implementation.
2. **Genuinely unfinished slices** — 84-5 pgvector RAG (migration only, no retrieval/query service); 80-3 mediation party-submission endpoints unwired; 80-2 dispute redesign 5-step wizard + i18n keys; 6-3/6-4 missing mobile comments/pinned UI; 79-1/79-2 e2e verification (79-2 is security-sensitive: SSO/JWT/cookie).
3. **Systemic screen-map drift** — 0 of ~120 screen-maps populate the frontmatter `epics:` field, so epic→screen linkage is impossible; this manufactures most of the 29 "orphan" screens. Backfilling `epics:` is the single highest-leverage fix.

## Ranked plan

### mvp
- [high] Reconcile sprint-status to done — 79-2-authentication-flow Authentication Flow Implementation — owner: pm-frontend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done; infra-dep:auth
- [high] coverage.json here still partial; gap-79-2-auth-callback-e2e tracked as open (full e2e verification remaining) (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: security/privacy sign-off
- [high] security-sensitive (SSO/JWT/cookie Path) — note for pm-security on final sign-off (79-2-authentication-flow Authentication Flow Implementation) — owner: pm-frontend — why: security/privacy sign-off
- [high] Reconcile sprint-status to done — 10b-5-support-data-access Support Data Access — owner: pm-security — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [high] support data access is privacy/security-sensitive: confirm retention + access-audit posture (pm-security) (10b-5-support-data-access Support Data Access) — owner: pm-security — why: security/privacy sign-off
- [high] Reconcile sprint-status to done — 6-1-announcement-creation-targeting Announcement Creation & Targeting — owner: pm-backend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [high] announcements.md screen buildStatus=in-progress, not shipped (6-1-announcement-creation-targeting Announcement Creation & Targeting) — owner: pm-backend — why: unfinished implementation · mobile most-behind
- [high] Reconcile sprint-status to done — 6-2-announcement-viewing-acknowledgment Announcement Viewing & Acknowledgment — owner: pm-frontend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [high] announcements.md screen buildStatus=in-progress (6-2-announcement-viewing-acknowledgment Announcement Viewing & Acknowledgment) — owner: pm-frontend — why: unfinished implementation · mobile most-behind
- [high] Reconcile sprint-status to done — 6-5-direct-messaging Direct Messaging — owner: pm-frontend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [high] Reconcile sprint-status to done — 79-1-api-client-integration API Client Integration for Core Features — owner: pm-frontend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [high] coverage gap noted full feature-integration / e2e verification incomplete despite infra (79-1-api-client-integration API Client Integration for Core Features) — owner: pm-frontend — why: unfinished implementation
- [high] Reconcile sprint-status to done — 10b-7-contextual-help-documentation Contextual Help & Documentation — owner: pm-frontend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [medium] Reconcile sprint-status to done — 6-3-announcement-comments-discussion Announcement Comments & Discussion — owner: pm-backend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [medium] announcements.md screen buildStatus still in-progress (not shipped) (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: unfinished implementation
- [medium] No mobile comments UI observed (6-3-announcement-comments-discussion Announcement Comments & Discussion) — owner: pm-backend — why: mobile slice
- [medium] Reconcile sprint-status to done — 6-4-pinned-announcements Pinned Announcements — owner: pm-backend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [medium] No mobile pinned-band UI observed (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: mobile slice
- [medium] announcements.md screen buildStatus in-progress (6-4-pinned-announcements Pinned Announcements) — owner: pm-backend — why: unfinished implementation
- [medium] Redesigned 5-step wizard (redesignStatus: in-progress) not shipped — only single-page form is live (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: unfinished implementation
- [medium] Localized `disputes.draft*` i18n keys for 6 message bundles are a documented fast-follow (currently English-default t(key, defaultValue)) (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: unfinished implementation
- [medium] Reconcile sprint-status to done — 80-3-mediation-resolution Mediation and Resolution — owner: pm-frontend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [medium] Party submissions endpoints unwired (apiStatus stays partial per dispute-detail screen-map) (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — why: unfinished implementation

#### Screen-map drift
- [medium] Backfill frontmatter `epics:` across ~120 screen-maps (0/120 populated) — restores epic→screen linkage — owner: pm-frontend — why: systemic: epic→screen first-match path currently impossible; root cause of orphan noise
- [medium] Add screen-map for orphan epic epic-85 OR mark no-UI — owner: pm-frontend — why: env/build-config epic — likely legitimately no UI surface; document the decision
- [medium] Add screen-map for orphan epic epic-8a (NotificationSettingsPage) — owner: pm-frontend — why: 8a-1 builds a ppt-web NotificationSettingsPage with no screen-map
- [low] Reconcile 29 unmapped screen-maps via epics-backfill (NOT removals) — owner: pm-frontend — why: false-positive class: out-of-scan-scope (13/25 epics) + empty `epics:`; resolved by backfill task above
- [medium] Link UC UC-10 to a screen-map — owner: pm-frontend — why: UC referenced in story files but in no screen-map's use-cases
- [medium] Link UC UC-29 to a screen-map — owner: pm-frontend — why: UC referenced in story files but in no screen-map's use-cases
- [medium] Link UC UC-33 to a screen-map — owner: pm-frontend — why: UC referenced in story files but in no screen-map's use-cases
- [medium] Link UC UC-40 to a screen-map — owner: pm-frontend — why: UC referenced in story files but in no screen-map's use-cases

### phase3
- [medium] Reconcile sprint-status to done — 84-5-pgvector-rag pgvector RAG Migration — owner: pm-backend — why: code/PRs landed but sprint-status=ready-for-dev/review; reconcile to done
- [medium] RAG retrieval/query service (embedding generation + similarity search) not implemented — migration only (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: unfinished implementation
- [medium] Vector path is conditional/optional (JSONB fallback) rather than a hard pgvector dependency (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: unfinished implementation

---
Buffer: 54/36 open · 34 candidates ranked but unqueued (buffer over-full → slots_to_fill=0, no refill)
