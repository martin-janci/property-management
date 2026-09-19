# code-review-ppt-web-core-community-route-fully-stubbed

**Vector:** bug
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-11 (ppt-web-core: routes/groups/community.tsx)
**Confidence:** high

## Hypothesis
`frontend/apps/ppt-web/src/routes/groups/community.tsx` — the routed `FeedPageRoute` (lines 20-41) mounts `<FeedPage posts={[]} total={0} …>` with **every** action handler a no-op arrow (`onCreatePost`, `onLikePost`, `onCommentPost`, `onSharePost`, `onEditPost`, `onDeletePost`, `onLoadMore`, `onFilterChange`), and no `useQuery`/`useMutation` wiring. `GroupsPageRoute` and any siblings in the same file follow the same pattern. Result: the community feed is permanently empty in production and every user interaction silently discards state. Smallest fix: wire `FeedPageRoute` (and the sibling routes in the same file) to the corresponding `@ppt/api-client` hooks so the feed loads real posts and the action handlers actually call the API.

## Evidence
- `frontend/apps/ppt-web/src/routes/groups/community.tsx:20-41` — `FeedPageRoute` body, all handlers no-op, `posts={[]}`, `total={0}`.
- `frontend/apps/ppt-web/src/features/community/components/` — real `PostCard`, `EventCard`, `CreatePostForm`, `CommentList`, `GroupList` components exist; the route is stubbed while the feature code is production-grade.
- `backend/servers/api-server/src/routes/community.rs` — API endpoints exist (posts, comments, groups, RSVP, reactions) and have been through the cross-tenant IDOR audit already (see `plans/_archive/code-review-api-handlers-community-cross-tenant-idor.md` and `.../community-unauthenticated-reads.md`).
- Nothing points to intentional stubbing — no `TODO`, no feature flag, no `// stub`. It looks like the route was scaffolded and never re-visited.
- User impact: the `Community` navigation entry loads a blank page with disabled interactions. Users see no posts, and reports of "Community doesn't work" are indistinguishable from an outage.

## Files
- `frontend/apps/ppt-web/src/routes/groups/community.tsx`

## Required capabilities
- [x] C1 — Systematic debugging (bug)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `pnpm -F ppt-web dev`; sign in.
2. Navigate to `/community/feed`. Expected: recent posts render. Actual: `<FeedPage>` mounts with `posts=[]`, `total=0` — permanently empty state, whatever the backend has.
3. Click "Create post". Expected: form submits and post appears. Actual: `onCreatePost={() => {}}` — nothing happens.
4. Repeat for `/community/groups`, `/community/events`, and the other routes in the same file.

## Suggested approach
1. Import the `@ppt/api-client` community hooks — check `packages/api-client/src/community/hooks.ts` for the exported set (e.g. `useCommunityFeed`, `useCreatePost`, `useLikePost`, `useCommentOnPost`, etc.). If a specific hook is missing, prefer wiring what exists and file a follow-up for the rest — do not widen this PR to add API surface.
2. In `FeedPageRoute`, replace `posts={[]}` / `total={0}` with `const { data } = useCommunityFeed({ page, pageSize })` (paged) and pass `data?.items ?? []` + `data?.total ?? 0`.
3. Wire each action handler to the matching mutation hook: `onCreatePost` → `useCreatePost().mutate`; `onLikePost` → `useLikePost().mutate({postId})`; etc. Handle optimistic state via TanStack Query defaults; do not roll a custom queue.
4. Apply the same treatment to `GroupsPageRoute` (and any other stub route in this file). Keep changes to this one file.
5. Add a boot-time smoke `console.warn` guard behind `import.meta.env.DEV` that fires when a required community hook returns `undefined` and the route is mounted, so a future accidental un-wiring is loud.
6. Skip changes to `features/community/components/` — those are production-grade already.

## Alternatives considered
- **Delete the route and hide the nav entry until wired** — rejected because the backend endpoints work and users have a nav entry today; deleting adds a regression window and doesn't solve the underlying "routed page silently discards work".
- **Introduce a shared `useCommunityFeedController` hook that composes all the wiring** — rejected as premature abstraction; the route file is the natural home for the wiring and the feature has no other consumer.

## Root-cause trace
1. Symptom: `/community/feed` renders an empty feed; `onCreatePost` and every other handler do nothing.
2. ← `FeedPageRoute` at `routes/groups/community.tsx:20-41` calls `<FeedPage posts={[]} total={0} … onCreatePost={() => {}} …>`.
3. ← No `useQuery`/`useMutation` imports at the top of the file; the file body was a scaffold that predates the API client hooks landing.
4. Origin: initial community route scaffolding — file has never been touched since the `@ppt/api-client` community hooks became available. No PR wired it up; this plan does.

## Test plan
- [ ] Add `frontend/apps/ppt-web/src/routes/groups/community.test.tsx` — mount `FeedPageRoute` with a mocked `@ppt/api-client`, assert `useCommunityFeed` was called and rendered items reflect the mock. Assert `onCreatePost` calls the mutation with the form payload.
- [ ] Snapshot / assertion for `GroupsPageRoute` mirroring the same shape.
- [ ] `pnpm -F ppt-web test` and `pnpm -F ppt-web typecheck` locally.
- [ ] Manual smoke via `pnpm -F ppt-web dev` on a seeded stack (`stack up pm-local`) is nice-to-have but not required for CI.

## Out of scope
- Any change to `backend/servers/api-server/src/routes/community.rs` (already covered by prior audits).
- Redesign of the community feed UX / infinite-scroll behaviour.
- Adding new community API endpoints or hooks; wire only what exists.
- reality-web (portal) community wiring — different app, separate plan.

## After-merge
- Move this file to `plans/_archive/code-review-ppt-web-core-community-route-fully-stubbed.md`
- Mark the matching `backlog.json` row as `status: "done"`
- If any `@ppt/api-client` community hook was missing during implementation, file a follow-up issue naming the gap.
