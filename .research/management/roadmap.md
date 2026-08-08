# PPT Roadmap — upkeep 2026-08-08

## State of the project

- Stories: **47 done / 2 partial / 0 not-started** of 49 (13 epics). No story-status flips this window — the 2 remaining partials (84-1 direct-to-S3 upload wiring, 84-2 e-signature sign page) are unchanged since 2026-07-15/07-30.
- Delta vs 2026-07-30 upkeep: 6 PRs merged (#2712, #2711, #2709, #2707, #2706, #2708) — mostly security/DoS hardening on the workflow-automation surface (api_call.rs response-body cap + fail-closed RAG batching + non-finite condition rejection) plus a dispute add_evidence access-audit event (epic-80) and a reality-web i18n fix. Only #2712 and #2706 map onto a tracked coverage story (80-1, 84-5) — both were already `done`; evidence appended, no status change.
- Biggest gaps, unchanged:
  1. **84-1** — ppt-web still uploads via server proxy; direct-to-S3 endpoint (#2309) has no frontend consumer.
  2. **84-2** — signer-facing document-sign page not built (screen-map planned, API complete); a prior implementer attempt failed with no PR.
  3. **Documentation drift, not delivery** — `sprint-status.yaml` epic-80 rollup still says `stories_completed: 1` while all 3 stories (80-1/80-2/80-3) have been `done` since 2026-06-25→07-15. Flagged as a decision this run rather than a coverage change.
- Screen coverage: 25 stories without a screen-map entry (mostly backend-only or already-shipped slices with no dedicated UI-mapping need) · 0 orphan epics · 0 orphan screens · 3 missing UC links (UC-33.1/2/3, dispute sub-UCs — all 3 re-queued this run).
- Buffer: **23/36 open** (up from 6/36 pre-run — 17 new items added: 12 role next_actions + 5 coverage-gap/screen-gap candidates; 2 in-progress items resolved to done by this window's merged PRs). The coverage-gap candidate pool is now genuinely thin (only 2 partial stories + 3 UC-link tasks remain in `coverage.json`) — closing the remaining gap to 36 depends on the dispatcher's own `backlog.json`-sourced refill (separate mechanism) or a fresh `/ppt-project-management scan` surfacing new gaps.

## Ranked plan

### mvp / finish-what's-started (highest score, 8)

- [high] Wire ppt-web direct-to-S3 upload via POST /api/v1/documents/upload-url — api-client binding + UploadDocument integration + regression test (84-1 partial) — owner: pm-frontend
- [high] Build signer-facing document-sign page in ppt-web against shipped signing API; flip screen-map ppt/document-sign buildStatus planned→shipped; verify signature-request email delivery (84-2 partial) — owner: pm-frontend

### process blockers surfaced this window (score 7-8, high priority)

- [high] Shepherd/unblock accounting MVP-loop trio (#2555 invoice lifecycle, #2558 invoice PDF, #2559 PAY-by-square QR) — 8+ days, zero reviewer engagement (escalated from 2 days at the 2026-07-30 check-in) — owner: pm-tech-lead
- [medium] Resolve CI-red on draft PR #2705 (dependabot rust-toolchain 1.94.1→1.100.0) — fix-forward or close — owner: pm-devops
- [medium] Reconcile sprint-status.yaml epic-80 rollup (1/3) against coverage.json / per-story development_status (3/3 done) — owner: pm-scrum-master

### QA regression follow-ups on this window's security/DoS fixes (score 6-7)

- [medium] Regression test: PR #2707 workflow api_call 8 MiB response-body cap (boundary + over-cap) — owner: pm-qa
- [medium] Regression test: PR #2706 RAG embedding partial-batch fail-closed path — owner: pm-qa
- [medium] Regression test: PR #2708 non-finite (NaN/Infinity) rejection in workflow condition compare — owner: pm-qa
- [medium] Release-readiness: full epic-80 (Dispute Resolution) regression pass before treating the epic as fully shipped — owner: pm-qa
- [medium] Risk-based regression suite for workflow_executor.rs condition evaluation (repeated-churn hotspot, 2 bug classes fixed this window) — owner: pm-qa
- [low] Regression coverage: PR #2712 dispute add_evidence access-audit event emission/payload — owner: pm-qa

### security / churn follow-ups (score 5-6)

- [medium] Follow-up hardening audit on services/actions/api_call.rs beyond #2707/#2710 (repeated churn hotspot, 139 lines this window) — owner: pm-security
- [low] Audit sibling reality-web forms for the same missing-i18n pattern just fixed in ListingForm by #2709 — owner: pm-frontend

### dependency / DX coordination (score 3-4)

- [low] Batch-triage 12 pending dependabot chore(deps) PRs (4 stalled 8d expo-*, 8 fresh 1d) — owner: pm-devops
- [low] PR #2385 (dependabot rust-toolchain, retry 2/2) closed unmerged — likely superseded, needs final disposition — owner: pm-devops
- [low] Bulk-triage carried untriaged issues #749-#779 backlog (retry 2/2) — owner: pm-scrum-master

### Screen-map drift (score 3-4)

- [medium] Link UC-33.1 to a dispute screen-map (missing_use_cases from coverage) — owner: pm-frontend
- [medium] Link UC-33.2 to a dispute screen-map (missing_use_cases from coverage) — owner: pm-frontend
- [medium] Link UC-33.3 to a dispute screen-map (missing_use_cases from coverage) — owner: pm-frontend

### carried backend follow-ups (score 3-5, in-progress / dispatcher-owned)

- [medium] gh-issue-2703: SSRF DNS-rebinding TOCTOU in api_call.rs — being fixed via draft PR #2710 — owner: pm-tech-lead (in-progress)
- [low] Churn hotspot: backend/servers/api-server/src/routes/layout/admin.rs — being fixed via draft PR #2713 — owner: pm-tech-lead (in-progress)
- [medium] gh-issue-2612: fire-once scheduled announcement/vote notifications drop on transient error — owner: pm-tech-lead (in-progress, retry 1/2)
- [low] mobile-native-kmp: getPortfolioAnalytics() truncates realtor portfolio at 100 listings — owner: pm-backend
- [medium] reality-server: InquiryNotifier seam not reached on the live send_contact_message path — owner: pm-backend (depends on the inquiry-email-stub item; PR #2696 pending rebase)
- [low] reality-server: inquiry-email-stub — shipped-but-non-functional notification path — owner: pm-backend (PR #2696 approved-but-dirty, needs rebase)

Buffer: **23/36 open** · project at 47/49 stories done — the 2 remaining partial stories (84-1, 84-2) plus a mix of process blockers (accounting trio, CI-red dependency bump), QA regression debt on this window's security fixes, and dependency triage make up the ranked plan. Candidate pool from `coverage.json` is thin (5 gap/screen-gap tasks total); reaching the full 36-item buffer depends on the dispatcher's separate `backlog.json`-sourced refill or a fresh deep `scan`.
