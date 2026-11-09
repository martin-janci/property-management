# pm-scrum-master — 2026-11-09 Phase 1.6

**Summary**: Delivery is healthy this window — 7 PRs merged in 3 days across a coherent frontend-registry theme (5 ppt-web registry PRs #3351/#3354/#3355/#3359 landing i18n + auth + mutation-toast + supporting tests), one reality-web share-comparison validation hardening (#3350 with runtime schema), one backend session-revocation fix (#3353 closes #3352), and one screen-map docs reconcile (#3358). The dispatcher surfaced a buffer-low alert because the ONLY claimable rows left are mobile-native-gated (4) or carry `UNRESOLVED:` dependency sentinels (2); this is a planner-refill problem, not a shipped-volume problem.

## shipped_since_last_run
- PR #3350 — reality-web share-comparison listing-body schema validation at the API boundary (fixes unchecked JSON cast class; parallel to the SSO/reality-server finding this routine surfaced).
- PR #3351 — ppt-web registry pet/vehicle forms wired through react-i18next (6 locale catalogues + 4 components; strings-only, no logic).
- PR #3353 — api-server authenticated password-change now revokes other sessions (closes issue #3352; includes `change_password_session_revocation_tests.rs`).
- PR #3354 — ppt-web registry-page auth context on registry requests (fixes anonymous 401 class).
- PR #3355 — ppt-web RegistryPage chrome translated; `registryPageI18n.test.ts` regression guard.
- PR #3358 — docs-only re-land of screen-map drift reconcile for PR #3293 (realtor-detail i18n) on current dev after the 2026-11-08 history-rewrite orphaned #3333.
- PR #3359 — ppt-web registry destructive-action outcomes surfaced via `useToastOptional` + i18n keys; re-land after #3356 add/add test-suite conflict.

## sprint_progress
{"sprint": "Epic 6, 7A, 8A & 10A — Announcements, Documents, Notifications & OAuth", "epics_done": 13, "epics_total": 13}

_All catalogued epics have `status: done` in coverage.json (49/49 stories). Sprint-status.yaml rollup caught up on 2026-10-26; nothing changed this delta. Forward work is improvement-track (backlog.json) + delivery hygiene (action-list), not epic progression._

## next_actions
- [high] **Clear the two `UNRESOLVED:` sentinels in action-list.json** (`pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28` → pm-qa accepts via today's pm-qa next-action; `pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28` → pm-security still owes). Both are 12 days stale. Owner: pm-scrum-master (coordination), DoD: both rows carry resolved `depends_on` and `.research/dispatcher-self-test.sh` T25 is clean.
- [high] **Decide the mobile-native cloud-build unblock keystone (#2652)** — 4 KMP backlog rows + 1 action-list row are gated on it; it has been carried as "high priority" on multiple roadmaps without landing. Bring this as a formal decision_needed: either commit to shipping the pm-devops-unblock keystone in this routine's next 7-day window, or explicitly deprioritise it and drop the gated KMP rows to `deferred`. Owner: pm-devops, DoD: pm-devops returns either a merged keystone PR or an explicit deferral in action-list.
- [medium] **Promote the two reality-server findings surfaced by today's Phase 1.5** (SSO OAuth JSON parse unbounded + public-listing-active resolve dup) into landable backend plans to directly address the buffer-low fire. Owner: pm-backend (via routine Phase 3). DoD: one or both plans promoted this run.
- [medium] **Add dispatcher self-test for orphaned bases** (issue #3357 class) — any assignment whose base commit is not an ancestor of origin/dev must hard-fail. Owner: pm-qa (co: pm-devops). DoD: T31 added to dispatcher-self-test.sh.

## blockers
- {"item": "mobile-native-kmp code-review backlog (4 rows)", "reason": "cloud-build unblock keystone #2652 unshipped; `mobile-native-gate.sh` skips any id containing `mobile-native`", "owner_role": "pm-devops"}
- {"item": "pm-frontend-mobile-rn-lint-gates-adopt-2026-10-28", "reason": "UNRESOLVED:pm-qa 12 days stale; today's pm-qa next-action accepts ownership — awaiting the next upkeep to resolve", "owner_role": "pm-qa"}
- {"item": "pm-scrum-master-violations-xtenant-404-contract-sweep-2026-10-28", "reason": "UNRESOLVED:pm-security 12 days stale; needs pm-security to adopt or drop", "owner_role": "pm-security"}

## open_questions
- Should the routine start auto-closing orphaned-from-dev PRs (per issue #3357), given the dispatcher has started skipping Tier-1d to conserve tokens? A nightly orphan-sweep would be cheap.
- Is the `coverage.json` "all done" signal (49/49) a true steady state, or does it just mean the catalogue is stale vs. the still-running improvement track? If the latter, a full `scan` is due.
- The dispatcher is skipping Tier-1d because the signal files are not being drained — but drained by whom? The research routine (Phase 2/3) or a new Phase 2.6? pm-tech-lead should weigh in.

## decisions_needed
- Formal decision on the pm-devops-unblock-mobile-native-cloud-builds keystone timeline (ship by date vs explicit deferral). — owner: pm-devops
- Churn-policy for frontend files hitting ≥4 PRs in a rolling week (integration-test gate?). — owner: pm-tech-lead (surfaced by pm-qa today)

## role_focus
- pm-scrum-master (always-on)
- pm-qa (rotating, cursor idx 3 → 4)
