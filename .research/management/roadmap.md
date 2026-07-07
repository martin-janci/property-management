# PPT Roadmap — deep scan 2026-07-07 (evening delta re-scan)

## State of the project

- Stories: **43 done / 6 partial / 0 not-started** of 49 (13 epics)
- Delta vs morning scan: +5 done (7a-2, 7a-5, 80-3, 83-3, plus 10a×3 gate-verified) and −2 (83-1, 83-2 downgraded to partial: no test coverage on OAuth/OTA flows)
- Biggest gaps:
  1. **Epic 83 integrations lack tests** — Airbnb OAuth flow and ALL Booking.com handlers (incl. 58K OTA XML module) have no test coverage; frontend integrations UI unrouted.
  2. **Sprint-status drift** — 10a-1/2/3 still `ready-for-dev` though every gate (#481/#482/#487) is closed; single reconcile task queued.
  3. **80-2 wizard redesign** in-progress (AC-4 draft persistence now shipped) and **84-5 pgvector RAG** retrieval service still unimplemented.
- Screen coverage: 31 stories without screen-map · 0 orphan epics · 15 orphan screens · 4 missing UC links (root cause: Epic-N frontmatter drift — task queued)

## Ranked plan

### mvp
- [high] Reconcile sprint-status ready-for-dev -> done for 10a-1/10a-2/10a-3 — gates #481 (closed 2026-05-26), #482, #487 all verified closed; code+screens+tests shipped — owner: pm-backend — why: mvp; sprint-status drift blocks 3 done stories from counting; auth-domain
- [high] 5-step wizard redesign still in-progress per sprint-status annotation (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: mvp partial; finish-what's-started; screen-gap *(in-flight elsewhere — not queued)*
- [high] No application handler calling search_similar_documents or embedding-write flow (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: mvp partial; finish-what's-started; screen-gap
- [high] RAG retrieval/query service (embedding generation + similarity search) not implemented in routes/repositories (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: mvp partial; finish-what's-started; screen-gap
- [medium] Frontend UI not built (OrganizationsPage.tsx not found; buildStatus=planned not shipped) (10b-1-organization-management-dashboard Organization Management Dashboard) — owner: pm-frontend — why: mvp partial; finish-what's-started

### phase2
- [medium] Create new schedule endpoint still stubbed out (81-1-report-schedule-editing Report Schedule Editing) — owner: pm-backend — why: phase2 partial; finish-what's-started; screen-gap

### phase4
- [medium] No test coverage for OAuth flow (connect/sync/disconnect) (83-1-airbnb-integration Airbnb OAuth and Sync) — owner: pm-backend — why: phase4 partial; finish-what's-started; risk:security
- [low] direct-connect and availability-sync enqueue handlers lack tests (83-1-airbnb-integration Airbnb OAuth and Sync) — owner: pm-backend — why: phase4 partial; finish-what's-started
- [low] frontend integrations UI not wired (settings/integrations not routed) (83-1-airbnb-integration Airbnb OAuth and Sync) — owner: pm-backend — why: phase4 partial; finish-what's-started
- [low] frontend property mapping wizard not wired (83-2-booking-integration Booking.com OAuth and Sync) — owner: pm-backend — why: phase4 partial; finish-what's-started
- [low] No test coverage for any Booking.com handler (83-2-booking-integration Booking.com OAuth and Sync) — owner: pm-backend — why: phase4 partial; finish-what's-started
- [low] OTA XML message parsing + generation untested (83-2-booking-integration Booking.com OAuth and Sync) — owner: pm-backend — why: phase4 partial; finish-what's-started
- [low] rate/availability push integration deferred (83-2-booking-integration Booking.com OAuth and Sync) — owner: pm-backend — why: phase4 partial; finish-what's-started

> `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86) remains superseded by this map.

Buffer: 35/36 open · 4 candidates ranked but unqueued
