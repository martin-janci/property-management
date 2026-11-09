# PPT Project State

_Generated: 2026-11-09 — routine Phase 1.6 incremental upkeep. Role focus: **pm-qa** (cursor idx 3 → 4, last run 2026-08-25 → today, 76-day stale slot cleared). Coverage `scan_kind=upkeep`; coverage_cursor idx 0 → 1 (epic-10a re-checked, all 3 OAuth stories still `done`). pm-scrum-master always-on. Trigger: dispatcher buffer-low fire — `claimable=6/72 — all open items gated(mobile-native)/dep-blocked`._

## Delta since last routine run (2026-11-06, 57h ago — above lag threshold)

- **7 PRs merged** (#3350–#3359 non-sequential): PR #3350 (reality-web share-comparison schema validation at API boundary), #3351 (ppt-web registry pet/vehicle forms i18n), #3353 (api-server password-change session revocation, closes issue #3352), #3354 (registry auth context), #3355 (RegistryPage i18n chrome), #3358 (docs-only re-land of realtor-detail screen-map drift), #3359 (registry mutation-toast re-land).
- **3 PRs closed unmerged** (#3301, #3333, #3356): all three intentionally orphaned per issue #3357 — a 2026-11-08 history rewrite dropped base commit `eac70d1c2` and the dispatcher re-landed each on current dev (#3302, #3358, #3359). Not regressions.
- **2 issues**: #3352 closed by PR #3353; #3357 OPEN (dispatcher root-cause writeup of the orphan incident, filed 2026-11-08).
- **Churn hotspots**: `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.tsx` (4 PRs in 3 days — most-churned code file this window), `frontend/apps/reality-web/src/components/comparison/ComparisonUrlHandler.tsx` (2 PRs), `backend/servers/api-server/src/routes/auth/password_reset.rs` (2 PRs).
- **Coverage upkeep**: epic-10a (OAuth Provider Foundation) re-checked — 3/3 stories still `done`, `last_checked = 2026-11-09`. No status flips this delta.

## Sprint & epic rollup

- **Sprint**: "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth".
- **Epics**: 13/13 `done` in coverage (49/49 stories). All forward work is improvement-track (backlog.json) + delivery hygiene (action-list).
- Nothing changed this delta — sprint-status.yaml rollup caught up on 2026-10-26 and stayed stable.

## What's next (top 5)

1. **[high] Clear the two UNRESOLVED: sentinels in action-list.json** — `pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` (pm-qa accepted ownership today; `depends_on` rewritten to `pm-devops-unblock-mobile-native-cloud-builds`), and `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` (pm-security still owes adoption or deferral). Both 12 days stale. — owner: pm-scrum-master
2. **[high] Formal decision on the mobile-native cloud-build unblock keystone (#2652)** — 4 KMP backlog rows + 1 action-list row are gated on it and have been carried high-priority for weeks without landing. Either commit to a 7-day ship window or drop the gated KMP rows to `deferred`. — owner: pm-devops
3. **[medium] Promote this run's two Phase 1.5 reality-server findings into landable backend plans** — `code-review-reality-server-sso-oauth-json-unbounded` (bug, +2) parallels the pattern just fixed in PR #3350, and `code-review-reality-server-public-listing-resolve-dup` (refactor, +1). Direct answer to the dispatcher's buffer-low fire. — owner: pm-backend (via routine Phase 3)
4. **[medium] Add dispatcher-self-test T31** — hard-fail when an assignment's base commit is not an ancestor of `origin/dev`. Would have caught the 2026-11-08 orphan incident before three PRs silently orphaned. — owner: pm-qa (co: pm-devops)
5. **[medium] RegistryPage integration test** — the surface has absorbed 4 PRs in 3 days with narrow PR-shaped unit tests; a single integration test covering all four mutation paths closes the cross-mutation regression window. — owner: pm-frontend

## Blockers

- **mobile-native-kmp code-review backlog (4 rows)** — gated by `mobile-native-gate.sh` until #2652 keystone lands. Carried weeks. owner: pm-devops.
- **pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28** — depends on the keystone above; pm-qa accepted today, awaiting keystone. owner: pm-qa.
- **pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28** — UNRESOLVED:pm-security 12 days stale. owner: pm-security.

## Per-role summaries (ran this run)

- **pm-scrum-master**: 7 PRs shipped (registry theme + one SSO finding parallel fix); 2 UNRESOLVED: sentinels on action-list block planning; mobile-native keystone needs a formal decision.
- **pm-qa**: test discipline on 5/7 merged PRs; churn in RegistryPage.tsx wants an integration test; three new actions queued (integration test, orphan-base self-test, sentinel escalation).
