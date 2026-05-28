---
slug: p0-10-mock-route-wiring
vector: bug
area: frontend
status: ready
created: 2026-05-23
source: dev-team-review (PR #435)
---

# P0-10: Wire 4 golden-path mock routes in ppt-web/App.tsx to real APIs

## Summary

Four production routes in `frontend/apps/ppt-web/src/App.tsx` render
hardcoded `mockX` objects instead of calling the backend. Users opening
a fault, announcement, message thread, or community group see fake
data and click no-op action buttons:

- `FaultDetailPageRoute`              (App.tsx ~line 1471)
- `ViewAnnouncementPageRoute`         (App.tsx ~line 1277)
- `EditAnnouncementPageRoute`         (App.tsx ~line 1324)
- `ThreadDetailPageRoute`             (App.tsx ~line 1400)
- `GroupDetailPageRoute`              (App.tsx ~line 1600)

(Pass 8 of the dev-team review listed 4 routes; ViewAnnouncement +
EditAnnouncement count as one functional area but two distinct
components, so the practical scope is 5 wrappers.)

## Why this wasn't done in PR #435

The hooks exist (`useFault`, `useFaultComments`, `useGroup`,
`createAnnouncementHooks`, `createMessagingHooks`). Wiring them
trips the FE-006 type-divergence finding from the same review:

```
src/App.tsx:1510 — error TS2740: Type 'FaultWithDetails' is missing
  the following properties from type 'FaultDetail': organizationId,
  buildingId, reporterId, createdAt, and 7 more.
```

The local feature components (`FaultDetailPage`, etc.) expect the
*hand-written* type layer in `frontend/packages/api-client/src/faults/types.ts`
(`FaultDetail`, `FaultComment`, `FaultAttachment` with flat field
shape). The new hooks return the *generated* type layer
(`FaultWithDetails`, etc.) which has a different shape.

PR #435 deliberately scoped to security fixes; un-tangling the
dual-API layer for these routes is a structural change in its own
right and would have ballooned that PR.

## Acceptance

A route is "done" when:

1. The component reads real backend data via the existing hook (no
   `mockX` object).
2. It handles `isLoading` (spinner), `isError` (retry CTA), and the
   empty / not-found case.
3. Action buttons trigger real mutations and invalidate the right
   query keys.
4. Slovak strings in `apps/ppt-web/messages/sk.json` cover any new
   user-visible text (the `t()` calls, not hard-coded English).
5. `apiStatus` on the corresponding screen-map flips from `stub` /
   `partial` to `complete`.

## Recommended approach

**Option A — Cheap unblock (1 PR, 0.5–1 day):**

Update the 5 local feature components to accept the api-client types
directly. Delete the parallel hand-written types in
`packages/api-client/src/faults/types.ts` and friends; re-export the
generated names if needed for back-compat. This is the FE-006 fix
folded in.

Pros: smallest delta; types stop drifting.
Cons: touches every consumer of those types, not just App.tsx.

**Option B — Adapter (1 PR, 0.5 day):**

Add a small adapter in App.tsx (`apiFaultToLocalFault(api: FaultWithDetails) → FaultDetail`)
that maps the generated shape onto the hand-written shape. The local
components stay unchanged.

Pros: contained.
Cons: leaves the dual-type problem in place; one more file to keep
in sync.

**Option C — Component rewrite (multi-PR):**

Rewrite each feature page to use api-client directly and drop the
prop-drilling pattern entirely. Move data fetching into the leaf
component. Biggest cleanup but largest blast radius.

Suggested order: A. If A is rejected as too broad, B is the
pragmatic stopgap.

## Test plan

- `pnpm -F @ppt/web typecheck` — must pass.
- `pnpm -F @ppt/web test:run` — must stay green (the
  `shared-auth.test.ts` 4-failure backlog is pre-existing TEST-203;
  don't introduce new failures).
- Manual smoke on each route via `pnpm -F @ppt/web dev`:
  - /faults/:id renders real data, retry on backend down works.
  - /announcements/:id same.
  - /messages/threads/:id same.
  - /community/groups/:id same.
- After implementation, run `/screens edit ppt/faults-detail`
  (and the other three) and flip `apiStatus` to `complete` per the
  screen-map protocol in root CLAUDE.md.

## Out of scope for this plan

- The bigger FE-006 cleanup beyond what option A touches.
- Adding new endpoints — all five routes have backend handlers.
- Mobile RN parity (separate plan).

## Suggested-approach steps

1. Pick A or B. If A, audit which files import `FaultDetail` /
   `Announcement` / `ThreadMessage` / `CommunityGroup` from
   `@ppt/api-client` and inventory the field-shape gaps.
2. Convert FaultDetailPageRoute first (smallest, hooks already in
   place). Verify typecheck, then test:run.
3. Repeat for the other four.
4. Drop the hand-written type aliases (option A) OR commit the
   adapter helper (option B).
5. Update the relevant `docs/screens/ppt/*.md` `apiStatus` fields
   and add an Agent Log entry per the screen-map self-management
   protocol in `CLAUDE.md`.
