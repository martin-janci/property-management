# PPT Project State

_Generated: 2026-07-18 — routine upkeep (pm-security rotation + Scrum Master synthesis). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (wrapped past epic-9)._

## Executive summary

- **Merged-PR wave (28 PRs since #2356):** the follow-up sweep from the 2026-07-16 post-merge review has drained. #2372-#2421 closed 22 of the 24 `from-merged-review` follow-up issues opened in the prior wave — the remaining open issues are #2360 (per-portal view-webhook dedup on replay, still in `quarantined` PR #2377) and #2366 (direct-to-S3 upload drops `building_id`, not yet claimed).
- **Backend hardening landed:** #2421 (cross-org /rag/migrate NOBYPASSRLS gate), #2420 (poisoned scheduler-metrics mutex recovery), #2416 (government_portal cross-tenant IDOR fix), #2419 (scheduler-RLS migration-number doc drift), #2375 (/agencies/me active-membership scoping), #2374 (/ai/chat/escalated role-gate regression), #2376 (portal_webhook_events dedup net). All shipped with backend/tests siblings.
- **Frontend hardening landed:** #2408 & #2378 (DocumentSignPage token hygiene — strip token from URL + suppress Referer + reload regression), #2411 & #2384 (localize create-schedule form + backend error toast), #2409 (reconcile favorites client DTO), #2382 (AC-5 favorite price-alert), #2415 (drop orphan `packages` i18n namespace), #2407 (sitemap deterministic + drift-guard).
- **Mobile hardening landed:** #2410 (iOS HEIC/HEIF transcode), #2383 (upload allow-list drift + MIME check), #2412 (purge AsyncStorage on session change), #2372 (mobile login clears whole query cache), #2379 (KMP Decimal-as-string decode trap on sibling reality-server surfaces).
- **Blocker still open (from prior state):** PR #2377 (auto-impl gh-issue-2360, per-portal view-webhook dedup gate) `quarantined` — `fix_rounds=3` exhausted, `verdict=changes` since 2026-07-16 16:10Z. Reviewer note: `Ok(None)-dup-collapses-to-Recorded(gate-inert)+clippy-red+00219-migration-collision`. Needs human review before releasing from quarantine.
- **NEW security finding — Marketplace stored-XSS (this run, Phase 1.5):** `frontend/apps/ppt-web/src/features/marketplace/pages/ProviderDetailPage.tsx:241` renders provider-controlled `website` field as `<a href={provider.website}>` with no scheme allow-list — hostile provider can smuggle `javascript:` URIs and get stored-XSS on any manager who visits their profile. Promoted to `plans/code-review-ppt-web-ui-marketplace-website-href-xss.md` (security fast-track: confidence=high + score=2).
- **NEW findings pending (this run, Phase 1.5) — same Marketplace slice:**
  - `code-review-ppt-web-ui-marketplace-i18n-regression` (score 2, refactor) — Marketplace + advanced-notifications ship 0 `useTranslation` calls, hardcoded English regresses the same PR #2411 fix on the reports side.
  - `code-review-ppt-web-ui-marketplace-notifications-untested` (score 1, test-gap) — both slices ship without any sibling `*.test.*` files.
- **Screen-map drift signals (heuristic):** PR #2411 & #2384 touched `routes/groups/reports.tsx` without updating `docs/screens/ppt/reports.md`. PR #2373 refactored reality-web listing-detail without updating `docs/screens/reality/listing-detail.md`. Drift is a review candidate — see backlog rows.

## Sprint progress

The 2026-07-15 deep scan (13 epics · 49 stories) is still the authoritative coverage; nothing this run flips a story terminal-vs-partial. `coverage.json` unchanged (upkeep only). The story-level actions listed in `roadmap.md` remain valid — the top slot (direct-to-S3 upload wiring) is now visible in issue #2366 as `open`.

## Shipped since last run

See Executive summary. All 28 PRs are `@martin-janci` — no author-diversity signal to flag.

## What's next (top 3 actions)

1. **Release PR #2377 from quarantine** — a human needs to review the `verdict=changes` note (dup-collapse gate-inert, clippy red, 00219 migration collision) and decide: (a) hand-fix and land, (b) close-and-restart with a fresh gh-issue-2360 branch, or (c) mark stale-blocked. Owner: pm-tech-lead + pm-scrum-master.
2. **Ship the marketplace-XSS fix (`plans/code-review-ppt-web-ui-marketplace-website-href-xss.md`)** — one small helper, two tests, no infra dependency. Confidence high, security fast-track. Owner: pm-frontend.
3. **Follow-up on #2366 (direct-to-S3 upload drops `building_id`)** — no PR yet. Owner: pm-frontend.

## Blockers

- **PR #2377 `quarantined`** (see above) — the only active assignment ledger row, blocking the dispatcher's Tier-1 buffer.
- **Marketplace + advanced-notifications slices went straight to `dev` without Phase 1.5 review** — the review gate did not fire on the merge. If this is a pattern, the Phase 1.5 rotating scope needs a large-new-feature scan step (currently only churn/oldest-unreviewed). Owner: pm-scrum-master (routine spec).

## Role focus today: **pm-security** (rotation idx 5, last 2026-05-27 — 52 days stale)

- **pm-security take:** the marketplace-XSS finding (Phase 1.5, this run) is the headline. Score 2 + confidence high + vector security → security fast-track promotes it to a plan directly. The RLS backend work (#2421, #2416) plus the IDOR / role-gate coverage (#2374, #2375) means the backend security posture continues to converge. Frontend security posture — as demonstrated by the marketplace slice landing without a scheme filter — remains the weak leg. Recommendation: add `noJavascriptUrl` (or equivalent) to Biome's rule set and set severity to `error`, so a future `<a href={untrusted}>` fails CI at merge time. Follow-up issue candidate.
- **pm-scrum-master (always-on):** delivery is healthy; 22-of-24 follow-up close-out this cycle. One quarantined PR (#2377) needs human touch. No new blocked epics.

## Coverage (upkeep this run)

- Rotating epic re-checked: `epic-9` (coverage_cursor idx 12). No story terminal-status changes triggered by merged PRs this run (all 28 PRs mapped to follow-up issues, not story-completing acceptance criteria).
- `coverage_cursor` advances to `0` (wraps — 13 epics total). Next run re-checks `epic-10a`.
