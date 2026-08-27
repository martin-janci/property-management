# churn-hotspot-content-moderation-page-extract

**Vector:** refactor
**Score:** 3
**Source:** PR #2860 + PR #2856 + PR #2849 + PR #2863 (4 touches this run) + code-review-ppt-web-ui segment finding
**Confidence:** high

## Hypothesis
`ContentModerationPage.tsx` (489 lines) has been touched 4 times in the last two days as the same feature — server-side overdue filter (#2856 / #2849 / #2860 / #2863) — rippled through filters, query call, transform, dialog state and truncation-notice logic. It now mixes six filter states, two dialog states, three query hooks, three mutation hooks, an inline API→UI transform and an overdue-truncation guard in one component. The next incremental change re-touches all of that. Extract the state + queries + transforms into a `useContentModeration` hook and the filters / stats / overdue banner / cases-list JSX into presentational sub-components — mirroring the successful `useAmlDashboard` + `AmlFiltersPanel` / `AmlThresholdsSection` / `AmlCountryRisksTable` extraction landed in PR #2848 for `AmlDashboardPage.tsx` (385 → 157 lines, same feature, same test suite unchanged).

## Evidence
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx:1-489` — single-file page holds 6 filter useStates, 2 dialog useStates, 3 query hooks, 3 mutation hooks, 2 map-transforms, 3 useCallbacks and the overdue-truncation JSX guard (`stats?.overdue_count > OVERDUE_PAGE_LIMIT && cases.length === OVERDUE_PAGE_LIMIT`).
- Churn this run: PR #2849 (client overdue affordance), PR #2856 (server-side overdue filter — deletes SLA constant, changes query shape), PR #2860 (truncation notice for the 200-case cap), PR #2863 (suppress false-positive at exact 200 boundary). All four touched the same page + its test file.
- `frontend/apps/ppt-web/src/features/compliance/pages/AmlDashboardPage.tsx:1-157` + `frontend/apps/ppt-web/src/features/compliance/hooks/useAmlDashboard.ts:1-240` — the exact target shape, landed in PR #2848 with the AML dashboard test suite passing unchanged.
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.test.tsx:1-407` — 407-line test suite already covers the filter widening, overdue affordance, server-side filter parity, and truncation notice; provides the safety net for a behavior-preserving move.

## Files
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx`
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.test.tsx`
- `frontend/apps/ppt-web/src/features/compliance/components/index.ts`
- `frontend/apps/ppt-web/src/features/compliance/hooks/useAmlDashboard.ts`

## Dependencies

## Required capabilities
- [ ] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps
1. `cd frontend && pnpm -F @ppt/ppt-web test src/features/compliance/pages/ContentModerationPage.test.tsx` — passes 6/6 today.
2. `wc -l frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx` — reports 489 lines; the goal state after refactor is <200 lines with the same test output.

## Suggested approach
1. Create `frontend/apps/ppt-web/src/features/compliance/hooks/useContentModeration.ts` next to the existing `useAmlDashboard.ts`. Move into it: the 6 filter useStates + `overdueOnly`, the 2 dialog useStates (`takeActionCaseId`, `decideAppealCaseId`), the three `useModerationCases` / `useModerationStats` / `useModerationTemplates` calls, the `cases` / `stats` / `templates` API→display transforms, the three mutation hooks and the six `useCallback` handlers. Return `{ filters, setters, cases, stats, templates, overdueOnly, setOverdueOnly, dialogs, handlers, mutationsPending }`.
2. Create `frontend/apps/ppt-web/src/features/compliance/components/ModerationFiltersPanel.tsx` — pure presentational, receives the filter values + setters + a `t` function; renders the 5 `<select>`s and the `unassignedOnly` checkbox that live in the current JSX around L370–L410.
3. Create `frontend/apps/ppt-web/src/features/compliance/components/ModerationOverdueNotice.tsx` — receives `overdueOnly`, `overdueCount`, `casesLength`, `OVERDUE_PAGE_LIMIT`, renders the truncation notice with the same guard the page has today (keep the `overdue_count > OVERDUE_PAGE_LIMIT && cases.length === OVERDUE_PAGE_LIMIT` invariant intact — that guard was the subject of #2862 / #2863 so any change is a behavior break).
4. Create `frontend/apps/ppt-web/src/features/compliance/components/ModerationCasesList.tsx` — receives `cases[]` + the four handler callbacks; renders the empty-state fallback and the `<ModerationCaseCard>` list currently at L426–L448. `ModerationCaseCard` and `ModerationQueueStats` are already components; do not re-extract them.
5. Rewrite `ContentModerationPage.tsx` as a thin orchestrator: `useContentModeration()`, render `<ModerationQueueStats>`, `<ModerationOverdueBanner>` (existing behavior at L349–L365 where the overdue alert widens the filter), `<ModerationFiltersPanel>`, `<ModerationOverdueNotice>`, `<ModerationCasesList>`, `<TakeModerationActionDialog>`, `<DecideAppealDialog>`. Keep `ContentModerationPage.displayName = 'ContentModerationPage'`. Target file <200 lines.
6. Export the four new components from `frontend/apps/ppt-web/src/features/compliance/components/index.ts` alongside the existing AML additions.
7. Run `pnpm -F @ppt/ppt-web test src/features/compliance/pages/ContentModerationPage.test.tsx` — must pass 6/6 with zero test changes. If a test needs a rewrite, the extraction leaked behavior; stop and re-scope.

## Alternatives considered
- **Leave the page as-is** — rejected because the last four PRs all touched the same lines and the next similar change (e.g. a second server-side filter or an appeals-specific view) would repeat the pattern; the extraction removes the coupling before it accretes further.
- **Split the page by route (queue vs case-detail) instead of by concern** — rejected because there is no separate case-detail route today (L487 does a `window.location.href` full-page reload), so a route split would need a router refactor as prereq; the hook + presentational split is landable in isolation and doesn't block a later route split.

## Root-cause trace
N/A — refactor doesn't need backward tracing.

## Test plan
- [x] `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.test.tsx` — the existing 407-line suite (6 tests) must pass unchanged after the extraction. Any required test edit is a behavior break, not a test-authoring exercise.
- [x] Add a snapshot / smoke render for `ModerationFiltersPanel` asserting all 5 `<select>` options + `unassignedOnly` checkbox render with a stub `filters` object. Lives at `frontend/apps/ppt-web/src/features/compliance/components/ModerationFiltersPanel.test.tsx`.
- [x] Command: `cd frontend && pnpm -F @ppt/ppt-web test src/features/compliance` — the whole compliance test set (moderation + AML + DSA) stays green.

## Out of scope
- No API changes. `overdue`, `limit`, `unassigned_only` query params stay as they are today.
- No i18n key changes. Every key that lives in `messages/*.json` today is re-used verbatim by the extracted components.
- No router change. `handleViewContent`'s `window.location.href` full-page reload is preserved as-is; converting it to `useNavigate` is a separate plan.
- No test rewrite. If a test needs to change, treat that as a behavior break and back out.

## After-merge
- Move this file to `plans/_archive/churn-hotspot-content-moderation-page-extract.md`
- Mark the matching `backlog.json` row as `status: "done"`
