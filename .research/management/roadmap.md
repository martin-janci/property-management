# PPT Roadmap — upkeep 2026-07-23

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). Unchanged since 2026-07-15 deep scan.
- Delta vs 2026-07-15: no story-status flips this window. Recent PRs are security hardening (dispute IDOR, document authz, OAuth test backfill, ammonia bump), test infrastructure (nextest partitioning, 206→8 test-binary consolidation), and post-merge follow-ups (#2483/#2484/#2485/#2486). Evidence entries added to 8 done stories.
- Remaining gaps (the last 2 partial stories, both frontend slices on shipped APIs):
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); prior implementer attempt failed no-PR.
- Screen coverage: 0 orphan screens · 0 validation errors · 3 missing UC links (UC-33.x dispute sub-UCs).
- Buffer: **36/36 open** (refilled this run from 8/36). Score ceiling = 8 (mvp partial finish-what's-started) — natural cap when only 2 partials remain; the rest of the buffer is post-merge follow-ups + KPI/analytics gaps + security cross-cutting audits.

## Ranked plan

### mvp / finish-what's-started (highest score, 8)
- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url: api-client binding + UploadDocument integration + screen-map note (84-1 partial; backend #2309 shipped) — owner: pm-frontend
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery (84-2 partial; prior attempt failed no-PR) — owner: pm-frontend

### security cross-cutting (score 7)
- [high] Cross-cutting webhook hardening audit — booking / airbnb / esignature / layout — #2485 shows layout webhook lacks replay guard; unknown parity elsewhere — owner: pm-integration
- [high] Verify layout publish webhook uses HMAC signature verification (parity with esignature webhook) — feeds #2485 fix design — owner: pm-security

### post-merge follow-ups (score 6, medium priority)
- [medium] Follow-up #2483: add_evidence dispute sub-resource cross-tenant-writable (PR #2490 open) — owner: pm-tech-lead
- [medium] Follow-up #2484: announce fan-out real-SQL integration test (currently pure-Rust re-model) — owner: pm-tech-lead
- [medium] Follow-up #2485: layout publish webhook timestamp/replay protection — owner: pm-tech-lead
- [medium] Follow-up #2486: mobile LAYOUT_CACHE_KEY not tenant-scoped, survives logout — owner: pm-tech-lead
- [medium] Follow-up #2366 (retry 1/2): direct-to-S3 upload drops building_id — owner: pm-tech-lead
- [medium] Post-#2485 layout webhook integration test — owner: pm-security
- [medium] Post-#2486 mobile layout cache tenant-scope test — owner: pm-qa
- [medium] Post-#2483 dispute add_evidence audit event — owner: pm-data

### pm-data KPI gap wave (score 4-5, medium/low)
- [medium] Define layout publish/webhook analytics events (published_by, layout_version, target_tenant_count)
- [medium] Define dispute-lifecycle KPI set (funnel + TTR percentiles + evidence-per-dispute)
- [medium] Instrument announcement fan-out with delivered/read/ack per targeting scope
- [medium] Support-staff read audit event schema (open decision from 2026-05-28)
- [medium] Publish data-retention policy for support-data / audit trail
- [medium] Formalize FaultStatusCount canonical definition (open decision from 2026-05-28)
- [medium] Instrument signup/onboarding-tour completion funnel (10b-6)
- [low] OAuth token issuance/refresh/revocation analytics per client (10a)
- [low] Reality-portal listing-view + search + inquiry analytics events
- [low] Mobile-native (Reality KMP) event tracking parity audit
- [low] pgvector RAG observability (retrieval latency, top-k, empty-rate)

### chore / small
- [medium] Link UC-33.1/33.2/33.3 (dispute sub-UCs) to dispute screen-maps' use-cases frontmatter — owner: pm-frontend
- [medium] Backfill screen-map frontmatter epics: field (unblocks future deep scans) — owner: pm-frontend
- [medium] Unblock stale PR #2433 (mobile-native iOS resolved-layout, 2d) — owner: pm-scrum-master
- [medium] Backfill regression test for scheduler malformed target_ids parse (#2436 fix) — owner: pm-qa
- [medium] cargo-audit sweep after #2446 ammonia bump — owner: pm-security
- [low] Follow-through on #2464 LayoutEditorPage style extraction (remaining components) — owner: pm-frontend
- [low] Document nextest partition + 206→8 consolidation as contributor runbook — owner: pm-devops
- [low] Confirm `just verify` gate adoption on open PRs (#2478/#2481/#2482/#2490/#2491) — owner: pm-qa
- [low] Review + merge dependabot cargo-minor-patch batch #2473 — owner: pm-devops
- [low] Bulk-triage untriaged issues #749-#779 backlog (30+ items in seen_signals) — owner: pm-scrum-master

### churn hotspots (score 1, low)
- [low] docs/screens/ppt/dashboard.md (3 touches) — owner: pm-tech-lead
- [low] docs/repo-map.md (4 touches) — owner: pm-tech-lead
- [low] backend/crates/db/src/models/mod.rs (retry 2/2, 12 commits) — owner: pm-tech-lead

Buffer: **36/36 open** · project at 47/49 — backlog genuinely converging on the 2 remaining frontend slices; the rest of the buffer is technical-debt / security-cross-cutting / analytics-KPI catch-up, not new-feature work. Treat underflow as success, not starvation.
