# code-review-ppt-web-ui-community-wired-nonfunctional

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review [ppt-web-ui] (2026-08-23 tier1d run)
**Confidence:** high

## Hypothesis
The `/community` routes are mounted in the production ppt-web router but every page renders a stub: `FeedPageRoute` passes `posts={[]}` with all handlers set to `() => {}`; `GroupDetailPageRoute` renders a hardcoded `mockGroup`; `CreateGroupPageRoute.onSubmit` shows a fake success toast and navigates without calling any `@ppt/api-client` mutation. Since `community` is NOT listed in `frontend/apps/ppt-web/src/features/unwired-features.ts` UNWIRED_FEATURES, this is an unintended half-built surface — paying customers see fake data and their form submissions vanish. Smallest correct fix: hide the routes behind the existing `UNWIRED_FEATURES` mechanism until the feature is wired (PAP-55 pattern), and add a failing route-test that asserts `/community` is not reachable while the flag holds.

## Evidence
- `frontend/apps/ppt-web/src/routes/groups/community.tsx:20-40` — `FeedPageRoute` passes `posts={[]}`, `total={0}`, and every handler (`onCreatePost`, `onLikePost`, `onCommentPost`, `onSharePost`, `onEditPost`, `onDeletePost`, `onLoadMore`) is a no-op `() => {}`. Community feed never loads data; no post action persists.
- `frontend/apps/ppt-web/src/routes/groups/community.tsx:73-114` — `GroupDetailPageRoute` renders a hardcoded `mockGroup` (`name: 'Sample Group'`, `description: 'A sample community group'`). `GroupsPageRoute` / `EventsPageRoute` / `MarketplacePageRoute` (community) all pass empty arrays + no-op handlers. `CreateGroupPageRoute.onSubmit` shows a fake 'Group created' toast and navigates WITHOUT calling any `@ppt/api-client` mutation — a silent input drop.
- `frontend/apps/ppt-web/src/routes/AppRoutes.tsx:51` — `communityRoutes()` is mounted in the production router (`/community`, `/community/groups`, `/community/groups/new`, `/community/groups/:groupId`, `/community/events`, `/community/marketplace`). Exposed to paying customers.
- `frontend/apps/ppt-web/src/features/unwired-features.ts:39` — `UNWIRED_FEATURES` list defines the canonical way to hide half-built surfaces (10 features already hidden this way per PAP-55). `community` is not in the list, so the shell is not gated.

## Files
- `frontend/apps/ppt-web/src/routes/groups/community.tsx`
- `frontend/apps/ppt-web/src/features/unwired-features.ts`
- `frontend/apps/ppt-web/src/routes/AppRoutes.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. `pnpm --filter @ppt/ppt-web dev` (or point a browser at the current dev deploy) and visit `/community`. Observe the feed page loads with zero posts and none of the buttons persist state.
2. Visit `/community/groups/anything` and observe the page renders `Sample Group` regardless of the URL parameter — data never comes from the API.
3. Visit `/community/groups/new`, fill in the form, submit. Observe a success toast, navigate away, and see the group was never created (`GET /api/v1/community/groups` returns the same list).

Expected after fix: `/community/*` returns a 404 (or the unwired-features gate short-circuit UI) while the flag is set, and the routes only mount once the wiring lands.

## Suggested approach
1. Add `'community'` to `UNWIRED_FEATURES` in `frontend/apps/ppt-web/src/features/unwired-features.ts:39`. Bump the `UnwiredFeature` union type derivation naturally.
2. In `frontend/apps/ppt-web/src/routes/AppRoutes.tsx` line 51, wrap `{communityRoutes()}` in the same conditional pattern used for other unwired features (e.g. `!UNWIRED_FEATURES.includes('community') && communityRoutes()`). If AppRoutes doesn't already use that guard for another feature, look at the pattern used in the router to hide unwired features and mirror it — do NOT invent a new pattern.
3. Add a Vitest route-test (`frontend/apps/ppt-web/src/routes/groups/community.route.test.tsx`) that renders the router with a memory route at `/community` and asserts the feed page is NOT reachable while `UNWIRED_FEATURES.includes('community')` is true. Copy the shape from an existing `*.route.test.tsx` under the same directory (e.g. `reports.create-schedule.route.test.tsx`).
4. Update the `community.tsx` file with a short header comment noting the file is intentionally stubbed and will be wired under Epic X — do NOT delete the mock render bodies; the shell stays so the wiring PR has a scaffold to fill in.
5. Run `pnpm -F @ppt/ppt-web check && pnpm -F @ppt/ppt-web typecheck && pnpm -F @ppt/ppt-web test` to confirm no regressions.
6. Do NOT wire the actual community endpoints in this PR — that is a separate, larger feature landing under its own Epic. This PR only removes the customer-facing exposure of the unwired surface.

## Alternatives considered
- **Wire the community feature end-to-end in this PR** — rejected because the API contract, tests, and design work for community are a multi-epic effort; the immediate risk is customer exposure to fake data, not the absence of the feature.
- **Delete `community.tsx` and the routes outright** — rejected because the scaffolding is useful when the wiring lands; hiding it behind the existing `UNWIRED_FEATURES` gate preserves the shell for the next PR while removing customer exposure now.

## Root-cause trace
1. Symptom: `/community` routes render fake data + swallow user input in production.
2. ← `frontend/apps/ppt-web/src/routes/AppRoutes.tsx:51` mounts `communityRoutes()` unconditionally.
3. ← `frontend/apps/ppt-web/src/routes/groups/community.tsx:20-114` renders stub pages instead of hitting the API, but the exposure control (`UNWIRED_FEATURES`) was never applied to `community`.
4. Origin: initial scaffold commit for the community feature (likely Epic 68 or earlier) — the shell landed without the corresponding `UNWIRED_FEATURES` entry. PAP-55 board decision explicitly requires stubs to be hidden until wired.

## Test plan
- [ ] New Vitest: `frontend/apps/ppt-web/src/routes/groups/community.route.test.tsx` — asserts `/community` renders the unwired-fallback (not the stub feed) when the flag is set. Would fail on `dev` today because the routes are unconditionally mounted.
- [ ] Manual: `pnpm -F @ppt/ppt-web dev` → navigate to `/community` and confirm the unwired-fallback renders.
- [ ] Command: `pnpm -F @ppt/ppt-web test`

## Out of scope
- Wiring the actual community endpoints (feed, groups, events, marketplace) — separate epic.
- Modifying `@ppt/api-client` community-* generated types or backend routes — no backend change is needed to hide the surface.
- Rewriting `unwired-features.ts` structure — just add one entry.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-ui-community-wired-nonfunctional.md`
- Mark the matching `backlog.json` row as `status: "done"`
