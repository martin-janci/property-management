# PM Action List

_Generated: 2026-06-26_

| ID | Action | Owner | Priority | Status |
|----|--------|-------|----------|--------|
| `churn-hotspot-backend-crates-db-src-repositories-reality-portal-rs` | Churn hotspot: backend/crates/db/src/repositories/reality_portal.rs (+59/−28 in PR #1297 PAP-142 IDOR scoping) | pm-backend | low | open |
| `churn-hotspot-backend-crates-db-src-repositories-form-rs` | Churn hotspot: 53 lines in backend/crates/db/src/repositories/form.rs (PR #1379 #1332 unblock) | pm-backend | low | open |
| `6-5-direct-messaging` | Coverage gap [mvp]: Direct Messaging — verify and finish to done. Gaps: sprint-status ready-for-dev — code is well ahead | pm-frontend | medium | open |
| `80-3-mediation-resolution` | Coverage gap [mvp]: Mediation and Resolution — verify and finish to done. Gaps: Party submissions endpoints unwired (api | pm-frontend | medium | open |
| `84-5-pgvector-rag` | Coverage gap [phase3]: pgvector RAG Migration — verify and finish to done. Gaps: RAG retrieval/query service (embedding  | pm-backend | low | open |
| `sec-480-ws-ticket-endpoint` | Implement short-lived WS ticket endpoint (POST /api/v1/ws/ticket → 30s opaque token) so JWT never appears in URL; closes | pm-security | high | open |
| `sec-481-formally-close-oauth-revocation` | Formally verify and close issue #481 (OAuth refresh-token revocation) — code fix at oauth.rs:413 has revoked_at IS NULL; | pm-security | high | open |
| `sm-refill-dispatcher-buffer-claimable` | Refill dispatcher action-list buffer to >=18 open (currently ~5); planner runs over remaining coverage gaps + security b | pm-tech-lead | high | open |
| `sec-accounting-server-threat-model` | Produce accounting-server security ADR (tenant isolation model, auth handoff from api-server OAuth, PII/financial classi | pm-security | high | open |
| `sm-unblock-draft-1812-reality-portal-split` | Human reviewer needed on draft PR #1812 (reality-portal-rs module split) — flagged needs-human-review, blocking churn-ho | pm-tech-lead | high | open |
| `sm-unblock-draft-1814-form-split` | Human reviewer needed on draft PR #1814 (form-rs module split) — needs-human-review | pm-tech-lead | high | open |
| `sm-unblock-draft-1846-80-3-mediation` | Human review/decision needed on draft PR #1846 (80-3 mediation) — gated by scope_drift signal; unblocks 80-3 promotion t | pm-frontend | high | open |
| `gap-79-2-authentication-flow-coverage-json-here-still-parti` | Coverage gap [mvp]: Authentication Flow Implementation — coverage.json here still partial; gap-79-2-auth-callback-e2e tr | pm-frontend | medium | open |
| `gap-79-2-authentication-flow-security-sensitive-sso-jwt-coo` | Coverage gap [mvp]: Authentication Flow Implementation — security-sensitive (SSO/JWT/cookie Path) — note for pm-security | pm-frontend | medium | open |
| `gap-79-2-authentication-flow-story-markdown-frontmatter-sta` | Coverage gap [mvp]: Authentication Flow Implementation — story markdown frontmatter stale ('Status: pending') (79-2-auth | pm-frontend | medium | open |
| `gap-80-3-mediation-resolution-mediation-resolution-flow-not-` | Coverage gap [mvp]: Mediation and Resolution — Mediation-resolution flow not promoted to done in sprint-status — story r | pm-frontend | medium | open |
| `gap-80-3-mediation-resolution-party-submissions-endpoints-un` | Coverage gap [mvp]: Mediation and Resolution — Party submissions endpoints unwired (apiStatus stays partial per dispute- | pm-frontend | medium | open |
| `sec-483-voice-device-idor-test` | Add IDOR integration test for voice device list-commands endpoint (existence-leak via empty list vs 403); close issue #4 | pm-security | medium | open |
| `sec-487-mfa-rate-limit-tests` | Add MFA brute-force/rate-limit test coverage to close issue #487 — clears 10a-1 story gate | pm-security | medium | open |
| `sm-split-emergency-rs-1681-loc` | Split emergency.rs (1681 LOC, top churn-hotspot, NOT yet refactored after the 6-split batch 2026-06-24) | pm-backend | medium | open |
| `sm-split-subscription-rs-1240-loc` | Split subscription.rs (1240 LOC, second-largest churn-hotspot pending split) | pm-backend | medium | open |
| `gap-84-5-pgvector-rag-no-screen-map-orphan-epic` | Coverage gap [phase3]: pgvector RAG Migration — no screen-map (orphan epic) (84-5-pgvector-rag) | pm-backend | medium | open |
| `gap-84-5-pgvector-rag-no-sprint-status-entry-and-no-` | Coverage gap [phase3]: pgvector RAG Migration — No sprint-status entry and no test exercising vector similarity search ( | pm-backend | medium | open |
| `gap-84-5-pgvector-rag-rag-retrieval-query-service-em` | Coverage gap [phase3]: pgvector RAG Migration — RAG retrieval/query service (embedding generation + similarity search) n | pm-backend | medium | open |
| `gap-84-5-pgvector-rag-vector-path-is-conditional-opt` | Coverage gap [phase3]: pgvector RAG Migration — Vector path is conditional/optional (JSONB fallback) rather than a hard  | pm-backend | medium | open |
| `sec-nginx-ws-token-log-redact` | Audit nginx/CDN access-log config: confirm ?token= query param is redacted for /ws upgrade paths (interim mitigation whi | pm-devops | medium | open |
| `sm-rental-rs-stable-instability-watch` | Investigate rental.rs stable-instability (387 LOC churn over 5 PRs); class of #1008 enum decode bugs — pin invariants te | pm-backend | medium | open |
| `screen-orphan-epic-85` | Add screen-map(s) for orphan epic epic-85 (no screen has frontmatter epics: epic-85) | pm-frontend | low | open |
| `screen-orphan-epic-8a` | Add screen-map(s) for orphan epic epic-8a (no screen has frontmatter epics: epic-8a) | pm-frontend | low | open |
| `sec-ai-principal-comment` | Add SECURITY comment to _principal discard handlers in ai/workflows.rs:490,558,738 + automation.rs:425,455 documenting g | pm-security | low | open |
