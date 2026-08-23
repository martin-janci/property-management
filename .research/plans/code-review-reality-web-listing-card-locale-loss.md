# code-review-reality-web-listing-card-locale-loss

**Vector:** bug
**Score:** 3
**Source:** PR #<none — surfaced by rotating expert review of reality-web scope segment on 2026-08-23; verified against dev HEAD 334c221>
**Confidence:** high

## Hypothesis
`ListingCard` imports the plain `Link` from `next/link`, so every click on a listing card
navigates to the raw `/listings/<slug>` path without the current-locale prefix. Reality-web
runs next-intl with `localePrefix: 'as-needed'` and `defaultLocale: 'sk'`, so a visitor
browsing `/cs/listings` (or `/en/…`, `/de/…`, `/pl/…`, `/hu/…`) gets bounced onto the
Slovak default the moment they open a card. The smallest change is to import the
locale-aware `Link` that `src/i18n/routing.ts` already re-exports via
`next-intl/navigation`; the component is otherwise unchanged.

## Evidence
- `frontend/apps/reality-web/src/components/listings/ListingCard.tsx:11` — `import Link from 'next/link';` (should be locale-aware).
- `frontend/apps/reality-web/src/components/listings/ListingCard.tsx:36` — `<Link href={`/listings/${listing.slug}`} …>` navigates from every ListingCard on the search results grid, home page, and favorites list.
- `frontend/apps/reality-web/src/i18n/routing.ts:1-12` — canonical locale-aware `Link` re-exported via `createNavigation(routing)`; already consumed by CookieConsentBanner, PriceAlerts, login page, not-found, sell page, etc.
- Sibling components using the locale-aware `Link` (`grep -l "from '@/i18n/routing'"` returns 5+ hits under `frontend/apps/reality-web/src/`), confirming the intended pattern.

## Files
- `frontend/apps/reality-web/src/components/listings/ListingCard.tsx`
- `frontend/apps/reality-web/src/components/listings/ListingCard.test.tsx`

## Dependencies

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `pnpm --filter @ppt/reality-web dev` and open `http://localhost:3000/cs/listings` (or any non-default locale).
2. Click any listing card in the grid.
3. Expected: URL becomes `/cs/listings/<slug>` (locale preserved).
   Actual: URL becomes `/listings/<slug>` — reality-web serves the Slovak default and the visitor loses their language mid-flow.

## Suggested approach
1. In `frontend/apps/reality-web/src/components/listings/ListingCard.tsx` replace `import Link from 'next/link';` with `import { Link } from '@/i18n/routing';`. The `href` string stays the same — next-intl's `Link` prepends the active locale automatically when `localePrefix: 'as-needed'` requires it.
2. Do not change `import Image from 'next/image';` (the image optimizer is orthogonal to routing and must stay on the vanilla import).
3. In `frontend/apps/reality-web/src/components/listings/ListingCard.test.tsx` add a regression case that renders `ListingCard` inside `NextIntlClientProvider` with a non-default locale (`en`) and asserts the rendered anchor `href` is `/en/listings/<slug>`, not `/listings/<slug>`. Follow the render helper pattern already used by `ListingDetailContent.test.tsx` if it exists, otherwise use next-intl's `getRequestConfig` test seam.
4. `pnpm -F @ppt/reality-web test -- ListingCard` to confirm the new case fails on the current `next/link` import and passes after the swap.
5. `pnpm check` + `pnpm typecheck` for the reality-web workspace.

## Alternatives considered
- **Wrap the raw `next/link` with a locale-prefix helper inside `ListingCard`** — rejected because it duplicates the exact concern `next-intl` already solves via `createNavigation(routing)`; every other reality-web link uses the routing-table `Link`, so a local helper would drift.
- **Change `localePrefix` to `always`** — rejected because that is an app-wide URL contract change (redirects every default-locale URL, breaks SEO on already-indexed non-prefixed Slovak URLs, and would rewrite the entire sitemap). The bug is one wrong import, not a routing-policy problem.

## Root-cause trace
1. Symptom: opening a listing card on any non-default locale silently drops the locale prefix — visitor lands on the Slovak default translation and downstream links/breadcrumbs stay Slovak.
2. ← `frontend/apps/reality-web/src/components/listings/ListingCard.tsx:36` — `<Link href={`/listings/${listing.slug}`}>` renders an `<a>` with the raw href because the imported `Link` has no locale awareness.
3. ← `frontend/apps/reality-web/src/components/listings/ListingCard.tsx:11` — `import Link from 'next/link';` shadows the locale-aware `Link` that `src/i18n/routing.ts` exports.
4. Origin: the component was authored before the `next-intl` navigation helpers were adopted in `src/i18n/routing.ts` (Epic 44 initial ListingCard vs. later i18n rollout); the migration swept most components but missed this one because `ListingCard.tsx` was not touched by the i18n PRs.

## Test plan
- [ ] `frontend/apps/reality-web/src/components/listings/ListingCard.test.tsx` — new case `preserves the active locale prefix in the card href` that renders `ListingCard` under `NextIntlClientProvider` with `locale="en"` and asserts the anchor `href` is `/en/listings/<slug>`.
- [ ] Regression: existing tests in `ListingCard.test.tsx` still pass unchanged (the swap is source-compatible — `next-intl`'s `Link` accepts the same `href` string).
- [ ] Local command: `pnpm -F @ppt/reality-web test -- ListingCard` (must first fail on current `main`, then pass after the import swap).

## Out of scope
- Auditing other components that may still import `Link` from `next/link` — that is a separate scan; this plan fixes the ListingCard occurrence surfaced by the review.
- Changing `localePrefix` policy, the sitemap, or SEO redirects.
- Refactoring `next/image` usage or the card layout.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-listing-card-locale-loss.md`
- Mark the matching `backlog.json` row (`code-review-reality-web-listing-card-locale-loss`) as `status: "done"`
