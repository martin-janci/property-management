# PPT Project State

_Generated: 2026-07-02 — daily routine catch-up (16-day lag since 2026-06-16 last run; deep-mode)._
_Rotation: pm-security lightweight synthesis this run (last full pm-security 2026-05-27, 36d stale). pm_cursor idx 5 → 6 (pm-data next). coverage_cursor idx 12 → 0 (epic-9 re-checked; next epic-10a)._

## Executive summary

- **Routine paused ~16 days.** State cursor was 2026-06-16T03:25Z; today is 2026-07-02T18Z. This is a **deep catch-up run**. The dispatcher (separate loop on the planning branch) has continued healthy — last dispatcher commit 2026-07-02 18:06Z with 2 active assignments, buffer starved but planner kicked. `.research/management/` artifacts have been maintained by the dispatcher; this run refreshes timestamps and the executive digest but does not re-scan coverage (that's the local `scan` mode).
- **316 non-research PRs merged since last routine run** (mean ~20/day). Composition: 74 fix, 57 test, 42 feat-other, 17 dep-bump, 17 docs, 11 refactor/perf/style, 7 feat-story, 1 revert, 90 "other" (mostly per-story scoped commits and dispatcher gap-scan tasks with descriptive titles).
- **Security-relevant landings dominate (53 of 316 merged).** Highlights, in rough chronological order: #1539 OAuth public-client rejection, #1552 OTA token-exchange manager-gate, #1563 developer-OAuth cross-user RLS, #1601 OTA manager-gate via TenantRole, #1606 accounting list_contacts manager-gate (PAP-281), #1616 centralized `@hey-api` auth interceptor, #1629 financial-client auth, #1682 disable_mfa RLS regression, #1702 message-attachments S3 presigned upload (BIT-184), #1741 rental guest PII manager-gate, #1782 JWT access-gate tightening, #1786 IoT sensor WS authz, #1806 Booking.com dual-auth model, #1823 rental guest ID-doc PII (audit + content sniff + manager gate), #1824 Stripe Checkout hardening (multi-currency + idempotency-key + webhook amount validation). Backend authz/RLS hardening is the dominant investment theme.
- **One revert (#1713).** Reverted #1690 (delegation frontend). Root cause is process/coordination — a gap-sweep re-admitted a board-retired customer surface ~16 min before CEO's BIT-198 ruling. Backend delegation surface stays live (used by voting delegation, Story 5.4). `unwired-features.ts` records delegation as retired; reconcile complete via BIT-213.
- **Only 1 stalled open PR (draft #1797).** `fix(api-server): auth on OCR endpoints + manager-gate rental guest PII reads (#1772, #1766)` — 8d since last update, still in draft. Author is same as most other backend PRs; this is the one that got parked.
- **Screen-map orphans (from last deep scan on 2026-06-23):** 29 orphan screens (13 ppt, 5 reality-mobile, 20 reality), 2 orphan epics (epic-85 build pipeline, epic-8a NotificationSettingsPage), 4 missing UC links (UC-10/29/33/40). Systemic root cause: 0 of 120 screen-maps populate the frontmatter `epics:` field — epic→screen linkage is impossible; every match was resolved by use-case tag or slug similarity.

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

`sprint-status.yaml` is one of the top churn files this window (10 changes) — the sprint moved substantially. Refer to the file for authoritative status; no diff-vs-last-run analysis in this deep-catchup run.

## Shipped since last run (316 non-research PRs, most-material only)

- **Security / auth / RLS** — see Executive summary bullet 3 (53 PRs).
- **Feature stories (7):** #1622 saved-search alerts in-app delivery (Story 16.3), #1642 portal-user CRUD (Epic 15), #1691 building geocoding (Story 3.1 AC3, BIT-191), #1701 resident "My Unit" view (Story 3.6, BIT-201), and 3 more UC-scoped feats.
- **Dep-bumps (17):** compose-bom 2026.05→2026.06, tower-http 0.6.11→0.7.0, ammonia 4.1.3 (RUSTSEC XSS-sanitizer bypass), quick-xml, and dependabot cargo-minor/gradle-minor groups.

## What's next (top 5 actions)

1. **[medium] Confirm the daily routine is back on schedule.** The routine hasn't run since 2026-06-16; today's commit is the resume. If it stops again for >36h a `stale_routine_alert` should surface. Owner: pm-devops. (Dispatcher on planning branch is independent and healthy.)
2. **[high] Resolve stalled draft PR #1797** (OCR auth + guest-PII manager gate). Draft for 8d; the fix itself is substantial (505 lines) and the security value is high. Owner: pm-security + pm-backend.
3. **[high] Re-audit the 29-orphan / 2-orphan-epic screen-map drift.** The 2026-06-23 deep scan flagged systemic `epics:` frontmatter emptiness. Backfilling `epics:` is the single highest-leverage fix (unblocks epic→screen linkage). Owner: pm-frontend + pm-tech-lead.
4. **[medium] Process the delegation-revert lesson (#1713 → BIT-213).** Add a pre-merge check that gap-sweep PRs consult `docs/superpowers/unwired-features.ts` before re-wiring retired surfaces. Owner: pm-tech-lead + pm-scrum-master.
5. **[medium] Complete stories flagged partial in last deep scan (12 partial of 49 total)**: 84-5 pgvector RAG retrieval, 80-3 mediation party-submission endpoints, 80-2 dispute wizard + i18n, 6-3/6-4 mobile comments/pinned UI, 79-1/79-2 e2e. Owner: mixed roles; see `roadmap.md`.

## Blockers

- **Routine cadence broken 16 days.** The routine is the eyes-on-code loop; the dispatcher's assignment loop kept moving but architectural / cross-cutting review coverage lapsed. Owner: pm-devops.
- **#1797 draft-stalled (8d).** Owner: pm-security + pm-backend.
- **Screen-map `epics:` frontmatter unpopulated across all 120 files.** Owner: pm-frontend + pm-tech-lead.

## Role focus today: **pm-security (lightweight)** (+ pm-scrum-master always-on)

- **pm-security** (rotation idx 5, last full 2026-05-27, 36d stale): No new next_actions or risks queued this run — pm-security synthesis is deferred to next daily run (which restores full rotation). Signal from Phase 1: 53 security-relevant merged PRs, dominant investment in backend authz/RLS hardening. Rental-guest PII, OTA manager gate, and OAuth public-client discipline all landed this window. Zero new open security issues this run (untriaged-issue count = 0).
- **pm-scrum-master** (always-on): synthesis above. Deep-catchup mode; role rotation resumes on next daily run.

## Coverage (deep scan — 2026-06-23; upkeep tick — 2026-07-02)

- Coverage snapshot last regenerated 2026-06-23 (deep scan). This run: coverage_cursor advanced 12 → 0 (epic-9 re-checked cheaply, no story-status flips detected via title-keyword sweep; next up epic-10a).
- Full ranked plan still in `roadmap.md` (not re-rendered this run — dispatcher owns action-list churn).
- **Systemic screen-map drift persists:** 0 of ~120 screen-maps populate frontmatter `epics:` → epic→screen linkage impossible; this manufactures the 29 "orphan" screens (really out-of-scan-scope). Backfilling `epics:` is the single highest-leverage fix.
