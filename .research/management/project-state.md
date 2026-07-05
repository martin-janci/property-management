# PPT Project State

_Generated: 2026-07-05 — Phase 1.6 re-fired by buffer-low signal (claimable 6/72). Scrum Master + pm-security ran (pm-security 39d overdue). Coverage `scan_kind=upkeep`; pm_cursor idx 5 → 6 (pm-data next), coverage_cursor idx 12 → 0 (epic-9 → epic-10a)._

## Executive summary

- **Buffer-low re-fire cleared:** action-list.json went from 8 open (16 non-terminal) to **40 open** — 20 coverage-gap candidates + 12 role next_actions seeded this cycle. Dispatcher now has ≥36 tasks to draw from.
- **Frontend `Authorization`-header pattern is broader than the two morning promotions.** pm-security surfaced 15+ additional `fetch()` call sites in `frontend/apps/ppt-web/src/features/news/pages/*.tsx` that drop the header — including manager-gated mutations (publish / archive / pin / comments / reactions). New backlog vector `code-review-ppt-web-core-news-feature-fetch-unauthed` (score 3, high confidence). Cannot promote to plan this run — daily cap (2) already hit.
- **Test-hardening batch governance gap:** `sprint-status.yaml`'s own rule says stories can't be promoted to `done` while their gating security issue is open, but `epic-8a` is marked `done` (8a-2, 8a-3 both `done`) while batch items #480 and #484 are still `status: open`.
- **Epic 10a (OAuth, 0/3 stories complete this sprint)** still has two open HIGH-severity gating items: #481 (refresh-token revocation bypassed) and #487 (MFA rate-limit gap). Both must close before any 10a story can leave `ready-for-dev`.
- **Stale drafts:** #1797 (backend-authz-ocr-and-rental-pii) at 12d and #1812 (reality_portal repo split) at 11d — both need an explicit disposition decision.
- **Shipped this window (7 PRs since 03:10 catch-up):** #2094–#2099 dispatcher-follow-up debt paydown + #1979 outages happy-path test rebase (BIT-414).

## Sprint progress (`_bmad-output/implementation-artifacts/sprint-status.yaml`)

- Active sprint: **Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth**
- Epics done: **1 / 6**
- Story-detail highlights: epic-7a partial (7a-2 CI red, 7a-3/4/5 residuals), epic-10a all three stories at `ready-for-dev` gated on #481/#482/#487, epic-8a marked done but #480/#484 still open, epic-6 fully shipped.

## What's next (top 5 across the roadmap + this run's role rotation)

1. **[high]** Fix `document_folder_tests` CI red and flip `7a-2-folder-organization` off "review" — owner: pm-backend (foundational to closing epic-7a)
2. **[high]** Extend the auth-header fix into a repo-wide sweep starting with `features/news/pages` (`publish`/`archive`/`pin`/`comments`/`reactions` all missing `Authorization`) — owner: pm-frontend
3. **[high]** Fix #481 OAuth refresh-token revocation bypass before any 10a story leaves `ready-for-dev` — owner: pm-backend
4. **[high]** Pick up `security-llm-doc-idor` (cross-tenant IDOR on AI listing data, plan already drafted) — owner: pm-backend
5. **[high]** Reconcile epic-10a sprint-status against `coverage.json` — confirm whether gate issues #481/#482/#487 are still open on GitHub — owner: pm-scrum-master (dep: pm-security)

## Blockers

- **7a-2-folder-organization (epic-7a):** CI red on `document_folder_tests` since PR #1316 round 1 — owner: pm-backend
- **10a-1/10a-2/10a-3 (epic-10a):** sprint-status test-hardening gate (issues #481/#482/#487) holds stories at `ready-for-dev` — owner: pm-security
- **PR #1797 (`fix/backend-authz-ocr-and-rental-pii`):** Draft 12 days on authz/PII — owner: pm-security
- **PR #1812 (reality_portal repo split):** Draft 11 days, structural change — owner: pm-tech-lead

## Role focus today

pm-scrum-master, pm-security.

### pm-scrum-master

Buffer-refill priorities plus a governance nudge: `sprint-status.yaml` epic-level rollups are stale versus story detail + coverage.json. Should be reconciled or auto-derived.

### pm-security

Auth-header pattern is a class-of-defects, not a one-off (see `roles/pm-security.md`). Epic 10a governance items #481/#487 are potentially live security holes if the "still open" flags reflect actual code state — verify against GitHub before trusting `coverage.json`'s `done` classification of 10a-*.
