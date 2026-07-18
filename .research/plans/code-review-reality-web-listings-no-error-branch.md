# code-review-reality-web-listings-no-error-branch

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review reality-web 2026-07-17 (frontend expert)
**Confidence:** high

## Hypothesis
The public search-results page at `/listings` swallows every API failure into a blank empty state that is indistinguishable from "zero matches". The route calls `const { data, isLoading } = useListings(filters, currentPage)` and never destructures `error`/`isError`; the file contains zero occurrences of `error`, `isError`, or `catch`. TanStack Query's default (no `throwOnError` set in `lib/query-provider.tsx`) keeps failures in query state, and `app/[locale]/error.tsx` never fires. The grid then receives `data?.data ?? []` and renders the empty state on error — on the app's main SEO/user entry point. The fix is to destructure `isError`/`error` from `useListings` and render an explicit error branch that matches the pattern already used by favorites, inquiries, saved-searches, FeaturedListings, and agency profile.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/listings/page.tsx:145` — `const { data, isLoading } = useListings(filters, currentPage);` — no `error` / `isError` destructured.
- `frontend/apps/reality-web/src/app/[locale]/listings/page.tsx:337-341` — grid receives `data?.data ?? []`; error state is fused with empty state.
- Grep confirmed: the file contains zero `error` / `isError` / `catch` tokens.
- Sibling pages `favorites/page.tsx` and `inquiries/page.tsx` each contain multiple explicit error-branch renders — this is a clear omission, not a design choice.
- `frontend/apps/reality-web/src/lib/query-provider.tsx` sets no `throwOnError`, so `app/[locale]/error.tsx` boundary never sees these failures.

## Files
- `frontend/apps/reality-web/src/app/[locale]/listings/page.tsx`
- `frontend/apps/reality-web/src/lib/query-provider.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (always tick for bug / revert / risky-churn)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running (`stack up pm-local …` or `ppt_dev_up` via bridge)
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)  · **local-only**
- [ ] C5 — ADB device (only for mobile-touching plans)  · **local-only**
- [x] C6 — Verification before completion (always tick)
- [ ] C7 — Code-review reception (tick if you expect controversy)

Mode: cloud-ok

## Repro steps
1. Start reality-web against a reality-server that is intentionally 500-ing on `/api/v1/listings` (e.g. block the reality-server port, or return `Response::internal_server_error()` from the listings route in a dev build).
2. Load `/en/listings` (or any locale variant) with no query — expect a visible error banner directing the user to retry.
3. Actual: the page renders the standard grid empty-state ("no listings match your filters") indistinguishable from a genuine zero-result search; the user has no signal to retry.
4. Same behaviour with a network-level failure (block DevTools throttling → offline).
5. On a healthy backend, verify the fix does not regress the empty-state UX when the API legitimately returns `[]`.

## Suggested approach
1. Update `frontend/apps/reality-web/src/app/[locale]/listings/page.tsx:145` to destructure `error` and `isError` (and any other TanStack Query state — `isFetching` for the refetch spinner — the sibling pages use).
2. Add an explicit error branch above the grid render (before the "Listings Grid" section) that mirrors the pattern in `favorites/page.tsx` — a translated heading, the raw error message (with i18n fallback), and a retry button that calls `refetch()`.
3. Ensure the retry button uses the same styling / component the other reality-web pages use (grep for the existing error-retry component and reuse it rather than open-coding a new one).
4. Wrap the user-facing error text in `t()` from `next-intl`; add missing keys to `en.json`, `sk.json`, `cs.json`, `de.json`.
5. Optionally centralise the failure surface by setting `throwOnError: (err) => err.status >= 500` on the `QueryClient` in `lib/query-provider.tsx`, so `app/[locale]/error.tsx` catches server-side failures uniformly. Keep this behind the same PR only if it doesn't regress client-error UX on the other pages.
6. Add a Vitest (RTL) test that mounts `<ListingsPage>` with a `QueryClient` seeded to return `isError: true` and asserts the error branch renders, plus a retry click triggers `refetch()`.

## Alternatives considered
- **Rely on a global TanStack Query error boundary in `query-provider.tsx`** — rejected as a sole fix because the sibling pages all render inline error branches; making listings the only global-boundary consumer would introduce a UX inconsistency and hide the failure in a full-page error instead of a scoped one. The plan keeps the option as an additive safety net.
- **Log the error to Sentry and keep the empty-state UX** — rejected because the user-visible symptom (a search that appears to return zero matches when the API is down) is a search-quality bug, not merely an observability gap; observability without a visible retry surface leaves the user stranded.

## Root-cause trace
1. Symptom: `/en/listings` shows "no matches" whenever the reality-server errors, indistinguishable from a legitimate empty search.
2. ← `app/[locale]/listings/page.tsx:145` destructures only `{ data, isLoading }` from `useListings` — the error branch is inaccessible.
3. ← `app/[locale]/listings/page.tsx:338` passes `data?.data ?? []` to `<ListingGrid>`, coalescing "error" and "zero results" into the same UI.
4. ← `lib/query-provider.tsx` does not set `throwOnError`, so `app/[locale]/error.tsx` never picks up the failure either.
5. Origin: initial reality-web listings-page implementation (predates the tier1d review window; verify with `git blame frontend/apps/reality-web/src/app/[locale]/listings/page.tsx`).

## Test plan
- [ ] `frontend/apps/reality-web/src/app/[locale]/listings/page.test.tsx` — new RTL test asserts an error branch is rendered when `useListings` reports `isError: true`.
- [ ] Regression: with `useListings` returning `{ data: { data: [] }, isLoading: false, isError: false }`, the page still renders the empty-state grid (no false-error surface).
- [ ] `pnpm -F reality-web test` passes locally.
- [ ] Optional (only if `throwOnError` is added in `query-provider.tsx`): favorites/inquiries/saved-searches Vitest suites still pass — no double-render of error UI.

## Out of scope
- Redesigning the search results empty-state / no-match UI.
- Adding new i18n locales beyond en/sk/cs/de.
- Sentry / observability instrumentation for the listings API.
- Adding error handling to sibling pages that already have their own error branches.

## After-merge
- Move this file to `plans/_archive/<slug>.md`
- Mark the matching `backlog.json` row as `status: "done"`
