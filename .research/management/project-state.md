# PPT Project State

_Generated: 2026-07-16T02:20:00Z — daily PM rotation (Scrum Master + pm-security). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-security → pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a)._

## Executive summary

- **Cross-tenant IDOR live in reality-server (release blocker).** Rotating expert review + pm-security confirmed: `POST /api/v1/agencies/{id}/invitations` at `backend/servers/reality-server/src/routes/agencies.rs:373` has no `check_agency_membership()` call, and the repo-layer `create_invitation` (`repositories/reality_portal/agencies.rs:223-248`) does a bare INSERT with zero authz. Any authenticated portal user can mint a 7-day invitation token (any email, any role) for any agency. Fix + regression test action queued (`pm-security-agency-invite-membership-check`, HIGH). Sister handlers `update_agency` (line 268) and `update_branding` (line 322) DO gate correctly — this is the outlier.
- **Systemic missing-guard pattern: 5th tenant-scoping IDOR in one 72h window.** After PRs #2289 (AI chat by-session), #2316 (news_articles), #2335 (rag/migrate RLS), #2356 (AI chat escalated/feedback). Move to type-level extractor (see `pm-security-agency-member-extractor` action).
- **Epic-84 (Backend integrations) fully closed on the frontend side.** #2345 wired ppt-web direct-to-S3 upload (closes 84-1-s3-presigned-urls partial); #2347 built signer-facing document-sign page (closes 84-2-esignature-email partial). Coverage `upkeep` marked both `done` this run; deep re-scan queued for verification.
- **Portal-webhooks churn hotspot.** `backend/servers/api-server/src/routes/portal_webhooks.rs` took 3 edits (#2286 replay-window, #2354 per-portal replay dedup) with 2 open follow-ups (#2358 duplicate-lead retry loop, #2360 syndication-stat inflation). Freeze recommended (`pm-scrum-master-portal-webhooks-stabilization`).
- **16 fresh follow-up issues from post-merge review** (#2318, #2320, #2357-2370): 14 already ingested to action-list by dispatcher; #2318, #2320, #2369, #2370 pending next tick. Two are tenant/session-isolation flavored (#2359, #2361) — treat as HIGH.
- **Backlog fully drained.** `.research/backlog.json` has 0 open items (186 total: 125 done, 60 dropped, 1 closed). Delivery signal now lives entirely in `action-list.json` (20 open items after this run's refill).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

Nominal sprint: **"Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth"** (stale). Real state via `coverage.json` deep scan (2026-07-15) + this run's upkeep:

- Stories: done=49, partial=0, not-started=0 (**49/49**)
- Epics fully done: 13/13 after 84-1/84-2 flipped this run.

The `epics:` rollup block in sprint-status.yaml is stale relative to `development_status` — reconciliation queued.

## Shipped since last run (cursor #2271 → #2356, 49 PRs)

- **Epic-84 closure:** #2345 direct-to-S3 upload ppt-web wire; #2347 signer document-sign page; #2306 screen-map for signer; #2346 mobile upload hardening
- **Cross-tenant IDOR sweep:** #2289 (AI chat by-session), #2316 (news_articles), #2335 (rag/migrate RLS), #2356 (AI chat escalated/feedback)
- **Webhook hardening:** #2286 (replay-window), #2354 (per-portal replay dedup, overflow edge), #2353 (retry-on-unmatched)
- **Reality-web resilience:** #2280 (AgencyDashboard "No Agency Found"), #2281 (SSR 500 on partial 200), #2351 (type-guard listing features/photos, 404 malformed detail), #2355 (agency /me + 404-vs-error)
- **Mobile:** #2288 (native price-tracking), #2290 (FCM+APNs combined push adapter), #2350 (KMP Decimal-as-string wire fix), #2334 (QueryErrorBanner), #2352 (JWT tenant-decode consolidation + cache clear)
- **Ops/DX:** #2296 (bump yanked spin), #2297/#2298 (screen frontmatter normalization + tracking reconcile), #2344 (synthesized epic id lower-case + suffix regex)

## What's next (top 5 actions)

1. **[high] Fix POST /api/v1/agencies/{id}/invitations IDOR** (`pm-security-agency-invite-membership-check`) — owner: rust-backend — release blocker.
2. **[high] Validate invitation.role against allow-list enum** (`pm-security-agency-invite-role-enum-validation`) — owner: rust-backend — potential privilege escalation.
3. **[high] Stabilize portal_webhooks.rs** (`pm-scrum-master-portal-webhooks-stabilization`) — owner: pm-tech-lead — bundle #2358 + #2360 into one hardening PR; freeze webhook work.
4. **[high] Close #2357** (deny-path test for AI escalated role gate) — dispatcher-ingested; already open.
5. **[high] Close #2361** (login cache eviction on org switch — tenant-data leak) — dispatcher-ingested; already open.

## Blockers

- **Reality-server agency invitation IDOR** — release blocker per pm-security. Decision needed: hotfix branch off `main` vs bundle with normal `dev` → `main` train.
- **portal_webhooks.rs stability** — 3 edits + 2 open follow-ups; recommend freezing feature work on this file until the hardening PR lands.

## Role focus today

- pm-scrum-master (always-on) — delivery synthesis, follow-up debt trend.
- pm-security (rotation index 5, last-run 2026-05-27 — 7 weeks stale) — deep security lens.

**Per-role summary:**

- **pm-scrum-master:** epic-84 closure locked; portal-webhooks + follow-up-debt velocity are the two active concerns. See `roadmap.md` for the ranked plan.
- **pm-security:** cross-tenant invitation-minting IDOR (release blocker) + 4 sibling IDOR fixes in 72h → systemic. Recommend type-level extractor + PR checklist. Full write-up at `roles/pm-security.md`.
