# code-review-ppt-web-core-community-route-fully-stubbed

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review (dispatcher Tier-1d 2026-09-11 ppt-web-core groups/community)
**Confidence:** high

## Hypothesis
The `/community` feed route is not wired to the community API. `FeedPageRoute` in `routes/groups/community.tsx` hands `<FeedPage>` an empty `posts` array, a hardcoded `total=0`, and every action handler as a no-op — so posts never render, no pagination fetches, and every Create / Like / Comment / Share / Edit / Delete click is silently dropped. The `usePosts` / `useCreatePost` / `useLikePost` / `useCreatePostComment` / `useDeletePost` hooks already exist in `@ppt/api-client`, so this is a wiring gap, not a missing feature. Fixing it means calling those hooks inside `FeedPageRoute` and forwarding real data + mutation callbacks down into `FeedPage`.

## Evidence
- `frontend/apps/ppt-web/src/routes/groups/community.tsx:20-41` — `FeedPageRoute` returns `<FeedPage posts={[]} total={0} … onCreatePost={() => {}} onLikePost={() => {}} onCommentPost={() => {}} onSharePost={() => {}} onEditPost={() => {}} onDeletePost={() => {}} onLoadMore={() => {}} />`, no `useQuery`/`useMutation` call anywhere in the route file.
- `frontend/packages/api-client/src/community/hooks.ts:138-141` — `usePosts(params)` already wraps `api.listPosts`; unused by any caller in `ppt-web/src/routes/`.
- `frontend/packages/api-client/src/community/hooks.ts:161`, `184`, `195`, `219` — `useCreatePost`, `useDeletePost`, `useLikePost`, `useCreatePostComment` exist and are unused from this route.
- `frontend/apps/ppt-web/src/features/community/pages/FeedPage.tsx` — the presentational page already expects `posts` / `total` / handlers as props, so wiring is a route-layer change only.

## Files
- `frontend/apps/ppt-web/src/routes/groups/community.tsx`
- `frontend/apps/ppt-web/src/features/community/pages/FeedPage.tsx`
- `frontend/packages/api-client/src/community/hooks.ts`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

**Execution mode (auto-derived from the ticks):** `cloud-ok`

Mode: cloud-ok

## Repro steps
1. Boot ppt-web (`pnpm -F @ppt/ppt-web dev`) with the api-server running against seed data that contains at least one `community_post`.
2. Sign in as any org member and open `/community` in the browser.
3. Expected: the feed renders the seeded post, and clicking "Create post" opens the composer and issues `POST /api/v1/community/posts`.
4. Actual: the feed is permanently empty (`posts=[]`, `total=0` hardcoded), and every action handler (`onCreatePost`, `onLikePost`, `onCommentPost`, `onSharePost`, `onEditPost`, `onDeletePost`, `onLoadMore`) is a `() => {}` no-op — no network request is made regardless of what the user clicks.

## Suggested approach
1. In `routes/groups/community.tsx`, replace `FeedPageRoute`'s hardcoded props with a real query: `const { data, isLoading, error, fetchNextPage, hasNextPage } = usePosts({ page: 1, per_page: 20, feed: 'all' })` — pass `posts = data?.items ?? []` and `total = data?.total ?? 0` into `<FeedPage>`.
2. Wire the mutation callbacks to the hooks: `const createPost = useCreatePost();` and forward `onCreatePost={(input) => createPost.mutate(input)}`; same for `useDeletePost`, `useLikePost`, `useCreatePostComment` (fed via `onCommentPost`). Reject `onEditPost` / `onSharePost` as out-of-scope only if the api-client has no matching hook; otherwise wire them the same way.
3. Forward `isLoading` / `error` states from `usePosts` into `<FeedPage>` so the presentational layer can render its existing loading/empty/error variants instead of the current perpetually-empty state.
4. Wire `onLoadMore` to `fetchNextPage()` (or a `page += 1` re-query if the API isn't paginated as `useInfiniteQuery` — pick whichever pattern the rest of `ppt-web` already uses, e.g. the leases feed).
5. Route `onFilterChange` back into `usePosts` params so switching the filter re-queries.
6. Add a Vitest test file `frontend/apps/ppt-web/src/routes/groups/community.test.tsx` that mounts `FeedPageRoute` with `MemoryRouter` and a `QueryClientProvider`, mocks `api.listPosts` to return a fixed post, and asserts the post title renders. Add a second case that clicks "Create post" and asserts `api.createPost` is called with the composer payload.
7. Run `pnpm -F @ppt/ppt-web test community` and `pnpm -F @ppt/ppt-web check` before committing.

## Alternatives considered
- **Delete the route entirely** — rejected because Epic 42 (Community Posts) is shipped on the backend and hooks exist; the feature is intended, the route just wasn't wired.
- **Move state into `FeedPage` itself** — rejected because the presentational split is intentional (mirrors `GroupsPage`, `EventsPage` patterns in the same file); mixing query state into the presentational component would drift from the rest of the community route group.

## Root-cause trace
1. Symptom: `/community` renders an empty feed and every button is inert; no network request fires on interaction.
2. ← `frontend/apps/ppt-web/src/routes/groups/community.tsx:20-41` — `FeedPageRoute` never calls `usePosts` and hands every handler a `() => {}` no-op.
3. ← Epic 42 rollout landed the presentational `<FeedPage>` and the api-client hooks, but the routing shell was extracted from `App.tsx` (see file header docstring) with placeholder wiring that was never replaced.
4. Origin: the routes/groups/community.tsx extraction commit — the placeholder stubs were meant as a temporary shim and were forgotten.

## Test plan
- [ ] `frontend/apps/ppt-web/src/routes/groups/community.test.tsx` — mount `FeedPageRoute`, mock `api.listPosts` to return one post, assert the post title renders (would fail on `posts={[]}`).
- [ ] Same file — click "Create post", assert `api.createPost` is called (would fail on `onCreatePost={() => {}}`).
- [ ] `pnpm -F @ppt/ppt-web test community` — runs the new test file.
- [ ] `pnpm -F @ppt/ppt-web check` — Biome lint + typecheck on the changes.

## Out of scope
- Redesigning the FeedPage UI or filters.
- Wiring the `Marketplace` / `Events` / `GroupDetail` routes in the same file — they have the same stub pattern but are separate stories; note them in the follow-up review, do not expand this PR.
- Adding a create-post modal (the button routes to a composer that already exists).

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-community-route-fully-stubbed.md`
- Mark the matching `backlog.json` row as `status: "done"`
