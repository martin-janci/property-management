# PPT Roadmap

_Generated: 2026-06-26T22:30:00Z — PM rotation idx 5→6 (pm-security run); coverage upkeep epic-9 (only 1 story, already done)._

## State of the project

- **Stories:** 46 done · 3 partial · 0 not-started (total 49 across 13 epics)
- **Promotion lag (top 3 closed this rotation):** 6-1 #1832, 6-2 #1834, 6-3 #1835, 6-4 d7e5039, 6-5 #1844, 79-1 #1830, 10b-5 #1829, 10b-7 #1831, 80-2 #1845 (UC-38 reconcile). 9 partials → done.
- **Remaining real gaps:** 79-2 auth-flow (SSO callback e2e shipped #1822, 2 of 3 gaps closed); 80-3 mediation (PR #1846 draft, scope_drift gated); 84-5 pgvector RAG (retrieval/query unwired).
- **Screen coverage:** 14 stories without screen-map · 2 orphan epics · 29 orphan screens · 4 missing UC links.

## Ranked plan

### MVP

- [high] Implement short-lived WS ticket endpoint (POST /api/v1/ws/ticket → 30s opaque token) so JWT never appears in URL; closes #480 permanently — owner: pm-security — score: 9
- [high] Formally verify and close issue #481 (OAuth refresh-token revocation) — code fix at oauth.rs:413 has revoked_at IS NULL; unblocks 10a-1/10a-3 promotion — owner: pm-security — score: 9
- [high] Refill dispatcher action-list buffer to >=18 open (currently ~5); planner runs over remaining coverage gaps + security backlog to restore claimable>1/72 — owner: pm-tech-lead — score: 9
- [high] Produce accounting-server security ADR (tenant isolation model, auth handoff from api-server OAuth, PII/financial classification) before any backend PR merges — owner: pm-security — score: 8
- [high] Human reviewer needed on draft PR #1812 (reality-portal-rs module split) — flagged needs-human-review, blocking churn-hotspot cleanup — owner: pm-tech-lead — score: 8
- [high] Human reviewer needed on draft PR #1814 (form-rs module split) — needs-human-review — owner: pm-tech-lead — score: 8
- [high] Human review/decision needed on draft PR #1846 (80-3 mediation) — gated by scope_drift signal; unblocks 80-3 promotion to done — owner: pm-frontend — score: 8
- [medium] Coverage gap [mvp]: Authentication Flow Implementation — coverage.json here still partial; gap-79-2-auth-callback-e2e tracked as open (full e2e verification remaining) (79-2-authentication-flow) — owner: pm-frontend — score: 6
- [medium] Coverage gap [mvp]: Authentication Flow Implementation — security-sensitive (SSO/JWT/cookie Path) — note for pm-security on final sign-off (79-2-authentication-flow) — owner: pm-frontend — score: 6
- [medium] Coverage gap [mvp]: Authentication Flow Implementation — story markdown frontmatter stale ('Status: pending') (79-2-authentication-flow) — owner: pm-frontend — score: 6
- [medium] Coverage gap [mvp]: Mediation and Resolution — Mediation-resolution flow not promoted to done in sprint-status — story remains partial pending submissions integration (80-3-mediation-resolution) — owner: pm-frontend — score: 6
- [medium] Coverage gap [mvp]: Mediation and Resolution — Party submissions endpoints unwired (apiStatus stays partial per dispute-detail screen-map) (80-3-mediation-resolution) — owner: pm-frontend — score: 6
- [medium] Add IDOR integration test for voice device list-commands endpoint (existence-leak via empty list vs 403); close issue #483 — owner: pm-security — score: 6
- [medium] Add MFA brute-force/rate-limit test coverage to close issue #487 — clears 10a-1 story gate — owner: pm-security — score: 6
- [medium] Split emergency.rs (1681 LOC, top churn-hotspot, NOT yet refactored after the 6-split batch 2026-06-24) — owner: pm-backend — score: 6
- [medium] Split subscription.rs (1240 LOC, second-largest churn-hotspot pending split) — owner: pm-backend — score: 6
- [medium] Audit nginx/CDN access-log config: confirm ?token= query param is redacted for /ws upgrade paths (interim mitigation while WS ticket is built) — owner: pm-devops — score: 5
- [medium] Investigate rental.rs stable-instability (387 LOC churn over 5 PRs); class of #1008 enum decode bugs — pin invariants test — owner: pm-backend — score: 5
- [low] Add SECURITY comment to _principal discard handlers in ai/workflows.rs:490,558,738 + automation.rs:425,455 documenting global-read intent; file follow-up for adding any-tenant-member role check — owner: pm-security — score: 3

### Phase 3

- [medium] Coverage gap [phase3]: pgvector RAG Migration — no screen-map (orphan epic) (84-5-pgvector-rag) — owner: pm-backend — score: 5
- [medium] Coverage gap [phase3]: pgvector RAG Migration — No sprint-status entry and no test exercising vector similarity search (84-5-pgvector-rag) — owner: pm-backend — score: 5
- [medium] Coverage gap [phase3]: pgvector RAG Migration — RAG retrieval/query service (embedding generation + similarity search) not implemented — migration only (84-5-pgvector-rag) — owner: pm-backend — score: 5
- [medium] Coverage gap [phase3]: pgvector RAG Migration — Vector path is conditional/optional (JSONB fallback) rather than a hard pgvector dependency (84-5-pgvector-rag) — owner: pm-backend — score: 5

### Screen-map drift

- [low] Add screen-map(s) for orphan epic epic-85 (no screen has frontmatter epics: epic-85) — owner: pm-frontend
- [low] Add screen-map(s) for orphan epic epic-8a (no screen has frontmatter epics: epic-8a) — owner: pm-frontend

Buffer: 30/36 open · 0 candidates ranked but unqueued
