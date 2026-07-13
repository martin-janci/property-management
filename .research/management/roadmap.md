# PPT Roadmap — deep scan 2026-07-13

## State of the project

- Stories: **40 done / 9 partial / 0 not-started** of 49 (13 epics)
- Delta vs 2026-07-07 scan (43/6/0): 81-2 and 83-1 promoted to done; 83-3 and 84-1..84-4 downgraded to partial on stricter platform-slice evidence (missing UI surfaces / transports / reindex path); 80-2, 81-1, 83-2, 84-5 remain partial with narrowed, verified gaps.
- Biggest gaps:
  1. **Epic 84 platform slices** — notification push transport (FCM/APNs), presigned-PUT uploads, e-sign UI, price-alert UI, RAG legacy reindex endpoint: backend cores shipped, product surfaces missing.
  2. **Report schedule editing (81-1)** — update_schedule leaves next_run_at stale on cron change (#2242 finding), Create-schedule UI missing.
  3. **Portal webhooks (83-3)** — replay-protection (freshness window) deferred; no manager-facing webhook status surface. Dispute draft autosave (80-2 AC-4) unwired.
- Screen coverage: 26 stories without screen-map · 0 orphan epics · 10 orphan screens · 5 missing UC links · 5 validation errors (root cause: story-id-style epic frontmatter — single normalize task queued)
- Tracking drift: 10 shipped stories (79-x, 82-x, 85-1, 9-1) still read pending/in-progress in story-md and are absent from sprint-status — single reconcile task queued.

## Ranked plan

### mvp
- [high] No dedicated price-alert subscription UI in reality-web (84-3-price-tracking Price Tracking for Favorites) — owner: pm-backend — why: mvp partial; finish-what's-started; mobile-behind; screen-gap
- [high] No mobile-native price tracking surface (84-3-price-tracking Price Tracking for Favorites) — owner: pm-backend — why: mvp partial; finish-what's-started; mobile-behind; screen-gap
- [high] reality/favorites screen-map not shipped (84-3-price-tracking Price Tracking for Favorites) — owner: pm-backend — why: mvp partial; finish-what's-started; mobile-behind; screen-gap
- [high] migrate_embeddings_to_pgvector not wired to any HTTP route — no way to reindex legacy no-provenance rows (they still mix embedding spaces in filtered search) (84-5-pgvector-rag pgvector RAG Migration) — owner: pm-backend — why: mvp partial; finish-what's-started; foundational; screen-gap
- [high] Mobile OS push (FCM/APNs) transport deferred — Push channel has no real device transport (84-4-notification-triggers Notification Trigger System) — owner: pm-backend — why: mvp partial; finish-what's-started; foundational
- [high] No frontend notification-trigger management UI (84-4-notification-triggers Notification Trigger System) — owner: pm-backend — why: mvp partial; finish-what's-started; foundational
- [medium] AC-4: useDraftStorage is NOT imported or called from FileDisputePageRoute.tsx — draft auto-save not wired into the filing flow (80-2-dispute-filing-flow Dispute Filing Flow) — owner: pm-frontend — why: mvp partial; finish-what's-started
- [medium] No presigned PUT endpoint for client-to-S3 direct upload — uploads still proxy through the server (84-1-s3-presigned-urls S3 Presigned URL Implementation) — owner: pm-backend — why: mvp partial; finish-what's-started
- [medium] No frontend UI component consuming the signature request flow (84-2-esignature-email E-Signature Email Integration) — owner: pm-integration — why: mvp partial; finish-what's-started
- [medium] No screen-map entry for a signature/e-sign screen in shipped state (84-2-esignature-email E-Signature Email Integration) — owner: pm-integration — why: mvp partial; finish-what's-started
- [medium] Reconcile tracking drift: flip story-md Status pending/in-progress -> done for verified-shipped 79-2/79-3/79-4/82-1..82-5/85-1/9-1 (code+tests verified 2026-07-07 and 2026-07-13) and add 84-x + 79-x/82-x/85-x/9-x ids to sprint-status.yaml development_status — owner: pm-backend — why: 10 shipped stories read as pending; blocks accurate coverage; cheap
- [medium] Normalize screen-map frontmatter: fix 9 validation errors (story-id-style epics like Epic-10B-7/6-5/Epic-9-1, empty epics fields, '7B') and re-point Epic-77/Epic-39 legacy refs to current epic ids; then link missing UCs (UC-33 disputes, UC-29 reports, UC-44 integrations, UC-10, UC-40) — owner: pm-frontend — why: single root cause for 10 orphan screens + 5 missing UC links

### phase2
- [medium] Frontend Create new schedule UI missing — no CreateScheduleModal/hook found (81-1-report-schedule-editing Report Schedule Editing) — owner: pm-backend — why: phase2 partial; finish-what's-started
- [medium] update_schedule does NOT recompute next_run_at when cron_expression changes — stale fire time (issue #2242 finding still present) (81-1-report-schedule-editing Report Schedule Editing) — owner: pm-backend — why: phase2 partial; finish-what's-started

### phase4
- [medium] Replay-protection (freshness/timestamp window) explicitly deferred in webhook.rs — HMAC proves authenticity but not freshness (83-3-portal-webhooks Real Estate Portal Webhooks) — owner: pm-backend — why: phase4 partial; finish-what's-started; risk:security; screen-gap
- [medium] No frontend screen for portal webhook management/status (inbound-only, no manager visibility surface) (83-3-portal-webhooks Real Estate Portal Webhooks) — owner: pm-backend — why: phase4 partial; finish-what's-started; screen-gap
- [low] Booking.com connect happy path (fetch_property outbound call) NOT exercised in tests — documented limitation, needs base-URL seam like Airbnb's (#2240 pattern) (83-2-booking-integration Booking.com OAuth and Sync) — owner: pm-integration — why: phase4 partial; finish-what's-started

> `_bmad-output/implementation-artifacts/gap-analysis-remediation.md` (Epic 86) remains superseded by this map.

Buffer: 18/36 open · 0 candidates ranked but unqueued
