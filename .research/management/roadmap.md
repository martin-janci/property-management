# PPT Roadmap — upkeep 2026-08-24

## State of the project

- Stories: **49 done / 0 partial / 0 not-started** of 49 (13 epics). **The coverage map is fully closed for the first time.**
- Delta vs 2026-08-06 upkeep: the two carried `partial` stories were verified as **stale, not open**:
  - **84-1 (S3 presigned URLs)** — `DocumentUpload.tsx` now calls `useUploadDocumentDirect()` (in-file comment cites `gap-84-1`); `@ppt/api-client` `documents/api.ts::uploadDocumentDirect` chains `POST /api/v1/documents/upload-url` → S3 `PUT` → register, pinned by `documents/api.test.ts`. → **done**
  - **84-2 (e-signature)** — `DocumentSignPage` is routed at `/sign` (`routes/groups/documents.tsx:107` via `lazyRoutes.tsx:22`); `DocumentSignaturePanel.tsx` + `DocumentSignPage.tokenHygiene.test.tsx` + `documentSignI18n.test.ts` present; screen-map `ppt/document-sign` `buildStatus: shipped`. → **done**
- Rotating epic re-check (`coverage_cursor` idx 5 = **epic-80, Dispute Resolution**): all 3 stories still `done`; `routes/disputes.rs`, four ppt-web dispute pages with tests, and 3 screen-maps all present. **Drift found:** `sprint-status.yaml` `epics.epic-80` header still reads `status: partial / stories_completed: 1` while its own `development_status` lists 80-1/80-2/80-3 all `done`.
- **The biggest gap is no longer a story — it is map coverage itself.** 8 of the 13 PRs merged this window (AML/compliance, facilities booking, verification badge, voice assistant) map to **no coverage story at all**. The 49-story map no longer describes where the code is actually changing.
- Screen coverage: 0 stories without screen-map · 0 orphan epics · 0 orphan screens · 3 missing UC links (UC-33.1/33.2/33.3, all dispute sub-UCs — all 3 queued this run).

## Delivery signal this window (2026-08-22 → 2026-08-24)

13 PRs merged, all into `dev`, all `post-merge-reviewed`. **5 of 13 were `from-merged-review` follow-ups**, two of them fixing PRs merged in this same window:

| Chain | Original | Defect issue | Fix | Elapsed |
|---|---|---|---|---|
| held-notification drain | #2826 (per-channel bookkeeping + bounded retry) | #2831 (double delivery across replicas) | #2834 (atomic claim, at-most-once) | < 48h |
| AML decision dialogs | #2829 (prompt/alert → in-app dialogs) | #2832 (stale reason/notes across assessments) | #2833 (state reset per assessment) | < 48h |

`quiet_hours_drain.rs` and `AmlDashboardPage.tsx` were each patched twice inside 48 hours. See **Role focus** in `project-state.md` and `roles/pm-qa.md` for the root-cause read.

## Ranked plan

### 1 — Restore planning inputs (score 8-9, high)

- [high] **Run the LOCAL `/ppt-project-management scan`** to rebuild `coverage.json` — the map is 49/49 done, the gap ranker has zero story candidates, and the majority of merged work is outside the map — owner: pm-tech-lead — why: without it the dispatcher buffer has no story-derived refill source.
- [high] **Resolve the UC-ACC-05 accounting trio (#2555 / #2558 / #2559)** — 26 days open, zero reviewer engagement, untouched since last run; assign a reviewer and merge, or close and re-plan — owner: pm-tech-lead — why: the entire accounting/invoice MVP loop is frozen behind it and it has been the named top blocker since 2026-07-30 with no movement.

### 2 — Close the pre-merge review gap (score 7-8, high) — pm-qa rotation focus

- [high] **Gate migration PRs on a DB-backed test**: any diff touching `backend/crates/db/migrations/**` must add ≥1 `#[sqlx::test]` in the same diff — owner: pm-qa — why: #2826 shipped migration 00234 + repo changes with 8 pure in-process `#[test]` and zero DB-backed tests, and regressed 48h later; the fix (#2834) added exactly the two `#[sqlx::test]` cases that would have caught it.
- [high] **Add a risk-class → required-test-level table to `pr-reviewer-prompt.md`** and carve concurrency / cross-process / component-lifecycle changes out of its current *"Test files … Skim: read assertions but don't deeply audit fixtures"* rule — owner: pm-qa — why: both regressions were test-**level** mismatches, so a "has tests?" check passes them.
- [medium] **Require a "re-open for a different subject" remount case for every ppt-web dialog holding `useState`** — owner: pm-qa — why: this is the #2832 defect class verbatim; #2833 added exactly two such cases.

### 3 — Test-floor debt on the churn hotspots (score 5-6, medium)

- [medium] Add encrypt/decrypt round-trip + wrong-key reject + legacy-plaintext-read tests for the voice OAuth token encryption centralized by #2838 — owner: pm-qa — why: single-file crypto refactor, 1 test marker, on a repeat churn hotspot; next most likely follow-up chain.
- [medium] Set a test floor for `ppt-web/features/compliance` — 3 pages + 7 components behind one test file, and the highest-churn regulated UI area (`ReviewAssessmentDialog`, `InitiateEddDialog`, `ContentModerationPage` first) — owner: pm-qa.
- [medium] Instrument a **post-merge rework rate** metric in the routine digest (`from-merged-review` PRs ÷ merged PRs; `with_issues` ÷ `prs_scanned`) — owner: pm-qa — why: it moved 0/52 (08-06..08-14) → 8/36 (08-20..08-23) → 5/13 this window and nothing surfaces the trend.

### 4 — Delivery hygiene (score 3-5, medium/low)

- [medium] Batch-triage the 13 open dependabot PRs against `dev` (none touched since last run) so open-PR count reflects real human work — owner: pm-devops.
- [medium] Decide on draft PR #2744 (dispatcher un-wedge, 10d, still DRAFT) — the issue it tracked (#2743) is closed; land it or close it — owner: pm-tech-lead.
- [low] Reconcile `sprint-status.yaml` `epics.epic-80` header (`partial` / 1-of-3) with its own `development_status` (all 3 done) — owner: pm-tech-lead.

### Screen-map drift (score 3-4)

- [low] Link **UC-33.1** to a dispute screen-map (`docs/screens/ppt/disputes.md` | `dispute-detail.md` | `file-dispute.md`) — owner: pm-frontend.
- [low] Link **UC-33.2** to a dispute screen-map — owner: pm-frontend.
- [low] Link **UC-33.3** to a dispute screen-map — owner: pm-frontend.

### Carried (unchanged this window)

- [medium] Follow-ups #2573 (DELETE-by-file-key same-org reference gap), #2574 (Android SSO CSRF half-wired), #2575 (`/disputes/kpis` window validation) — owner: pm-backend / pm-mobile.
- [medium] Cross-cutting webhook hardening audit (booking / airbnb / esignature / layout) — owner: pm-integration.
- [medium] SECURITY: Alexa voice webhook `verify_alexa_signature` never checks the signature — owner: pm-security.
- [medium] `gh-issue-2797`: cargo-deny RUSTSEC-2026-0258 (h2 empty-DATA-frame DoS) on `dev` — owner: pm-security.
- [low] repeated-churn `auth.rs` (2950 lines) module split; scheduler retention/prune extraction — owner: pm-tech-lead / pm-backend.
- 9 in-progress dispatcher items (mobile-native KMP, reality-web i18n, reality-server) carried unchanged.

Buffer: **20/36 open · 0 candidates ranked but unqueued** — the shortfall is NOT a triage backlog, it is map exhaustion: `coverage.json` is 49/49 done so the gap ranker produced only the 3 UC-link tasks. Refill must come from a deep `scan` or the dispatcher's Tier-1d dev-review generator.
