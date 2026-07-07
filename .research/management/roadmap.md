# PPT Roadmap — deep scan 2026-07-07

## State of the project

- Stories: **38 done / 11 partial / 0 not-started** of 49 (13 epics)
- Per platform: backend: 24d/10p · frontend: 24d/9p · mobile: 10d/1p
- Biggest gaps:
  1. **Epic 10a OAuth regressed done→partial** — test-hardening gates #481 (refresh-token revocation) / #482 (ProtectedRoute) / #487 (MFA e2e) block all 3 stories.
  2. **Epic 80 disputes UI wiring** — DisputeDetailRoute stub in App.tsx, party-submission endpoints unwired, wizard AC-4 draft persistence missing.
  3. **83-3 portal webhooks + 84-5 pgvector RAG are backend stubs** — signature-verified webhook drops inquiries; RAG migration has no retrieval service.
- Screen coverage: 31 stories without screen-map · 0 orphan epics · 15 orphan screens · 4 missing UC links
  - ⚠ Most screen 'orphans' stem from ONE root cause: `Epic-N` vs `epic-N`/`epic-7a` frontmatter format drift (see gap-screens-normalize-epic-frontmatter).

## Ranked plan

### mvp
- [high] RFC 9700 refresh-token revocation validation (issue #481) (10a-1-oauth-authorization-server OAuth 2.0 Authorization Server) [also 10a-3-oauth-token-management] — owner: pm-backend — why: mvp partial; finish-what's-started; risk:security; screen-gap
- [high] Issue #485 resolved per PR #2003, but sprint-status not yet updated to reflect completion (7a-5-document-sharing Document Sharing) — owner: pm-backend — why: mvp partial; finish-what's-started; mobile-behind; screen-gap
- [high] MFA e2e brute-force/rate-limit test coverage (issue #487) (10a-1-oauth-authorization-server OAuth 2.0 Authorization Server) — owner: pm-backend — why: mvp partial; finish-what's-started; screen-gap
- [high] CI test job failing (document_folder_tests FK/isolation); reverted from done pending green CI (7a-2-folder-organization Folder Organization) — owner: pm-backend — why: mvp partial; finish-what's-started; screen-gap
- [high] AC-4 draft persistence on 5-step wizard redesign (not single-page form) (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: mvp partial; finish-what's-started; screen-gap *(in-flight elsewhere — not queued)*
- [high] wizard redesign still in-progress (redesignStatus: in-progress) (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: mvp partial; finish-what's-started; screen-gap *(in-flight elsewhere — not queued)*
- [high] DisputeDetailRoute in App.tsx is stub (not using DisputeDetailPage.tsx) — routing wiring incomplete (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — why: mvp partial; finish-what's-started; screen-gap
- [high] party submissions endpoints unwired (apiStatus: partial) (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — why: mvp partial; finish-what's-started; screen-gap
- [high] sessions/submissions workflow not fully threaded from App.tsx (80-3-mediation-resolution Mediation and Resolution) — owner: pm-frontend — why: mvp partial; finish-what's-started; screen-gap
- [high] No application handler calling search_similar_documents or embedding-write flow (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: mvp partial; finish-what's-started; screen-gap
- [high] RAG retrieval/query service (embedding generation + similarity search) not implemented in routes/repositories (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: mvp partial; finish-what's-started; screen-gap
- [medium] ProtectedRoute role fallback fix for multi-tenant users (issue #482) (10a-2-oauth-client-registration OAuth Client Registration) — owner: pm-backend — why: mvp partial; finish-what's-started
- [medium] ProtectedRoute unit test coverage (issue #482) (10a-2-oauth-client-registration OAuth Client Registration) — owner: pm-backend — why: mvp partial; finish-what's-started
- [medium] Frontend UI not built (OrganizationsPage.tsx not found; buildStatus=planned not shipped) (10b-1-organization-management-dashboard Organization Management Dashboard) — owner: pm-frontend — why: mvp partial; finish-what's-started

#### Screen-map drift
- [medium] Normalize screen-map epic frontmatter (Epic-N vs epic-N/epic-7a drift): 31 stories unmapped, ~30 screens read as orphans; then re-run /ppt-project-management scan — owner: pm-frontend — why: screen-map drift root cause; unblocks story→screen mapping
- [low] Link UC-10 to a screen-map (appears in stories but no screen-map use-cases frontmatter) — owner: pm-frontend — why: screen-map drift; missing UC link
- [low] Link UC-29 to a screen-map (appears in stories but no screen-map use-cases frontmatter) — owner: pm-frontend — why: screen-map drift; missing UC link
- [low] Link UC-33 to a screen-map (appears in stories but no screen-map use-cases frontmatter) — owner: pm-frontend — why: screen-map drift; missing UC link
- [low] Link UC-40 to a screen-map (appears in stories but no screen-map use-cases frontmatter) — owner: pm-frontend — why: screen-map drift; missing UC link

### phase2
- [medium] Create new schedule endpoint still stubbed out (81-1-report-schedule-editing Report Schedule Editing) — owner: pm-backend — why: phase2 partial; finish-what's-started; screen-gap

### phase4
- [low] No inquiry acknowledgment or status tracking workflow (83-3-portal-webhooks Real Estate Portal Webhooks) — owner: pm-integration — why: phase4 partial; finish-what's-started *(in-flight elsewhere — not queued)*
- [low] PortalInquiry storage/retrieval not wired to handler (83-3-portal-webhooks Real Estate Portal Webhooks) — owner: pm-integration — why: phase4 partial; finish-what's-started *(in-flight elsewhere — not queued)*
- [low] Webhook handler is stub: verifies signature but doesn't store parsed inquiries to database (83-3-portal-webhooks Real Estate Portal Webhooks) — owner: pm-integration — why: phase4 partial; finish-what's-started *(in-flight elsewhere — not queued)*

> Note: `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86) is stale and superseded by this map.

Buffer: 28/36 open · 0 candidates ranked but unqueued
