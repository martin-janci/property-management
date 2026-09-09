# code-review-reality-web-listing-detail-suspense-bailout

**Vector:** bug
**Score:** 3
**Source:** code-review reality-web 2026-09-09 (Phase 1.5 rotating expert)
**Confidence:** high

## Hypothesis
`ListingDetailContent.tsx:88` calls `useSearchParams()` in a client component, and `[slug]/page.tsx:122` renders that component with no `<Suspense>` wrapper. In Next.js 16 App Router, `useSearchParams()` without a Suspense boundary opts the entire subtree out of static/prerender — including the JSON-LD `<script>`, breadcrumbs, LayoutSections, and agent-contact UI. Crawlers and social scrapers therefore fetch empty markup for the site's SEO-critical listing page, defeating the SSR + ISR + JSON-LD design that the file's own docstring describes. The smallest fix is to wrap the `useSearchParams()`-dependent leaf in `<Suspense fallback={...}>` so only that leaf bails; the JSON-LD and above-the-fold content remain in the prerendered HTML.

## Evidence
- `frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx:88` — `useSearchParams()` called unconditionally in a client component with no Suspense above it.
- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx:122` — renders `<ListingDetailContent ...>` directly under the RSC page; `grep -n Suspense` in either file returns zero matches.
- Adjacent search page pattern (e.g. `app/[locale]/listings/page.tsx`) already wraps client search-params consumers in `<Suspense>` — the listing-detail route is the outlier.
- Next.js 16 App Router documented behaviour: any client subtree reading `useSearchParams()` opts out of pre-rendering up to the nearest Suspense boundary.

## Files
- `frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx:88`
- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx:122`

## Dependencies
- (none)

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. `cd frontend && pnpm build -F reality-web` (or run the built app), then curl the SSR HTML for a listing route: `curl -sS http://localhost:3000/sk/listings/<slug> | grep -c 'application/ld+json'`.
2. Expected (after fix): count ≥ 1 (JSON-LD present in pre-rendered HTML). Actual (today): 0 — the JSON-LD `<script>` is only mounted client-side after hydration because the subtree bailed out of prerender.

## Suggested approach
1. Refactor `ListingDetailContent.tsx` so that the `useSearchParams()` call lives in a small inner leaf component (e.g. `ListingDetailQueryReader`) whose only job is to read/consume the query param.
2. In `[slug]/page.tsx:122`, keep the top-level `<ListingDetailContent {...props}>` (which now renders the JSON-LD `<script>`, breadcrumb, LayoutSections, agent-contact) in the prerendered tree; wrap ONLY the new leaf in `<Suspense fallback={null}>` (or a small skeleton) so its bail-out does not cascade.
3. Add a regression test at `frontend/apps/reality-web/src/components/listings/__tests__/ListingDetailContent.ssr.test.tsx` that renders the page as an RSC and asserts the JSON-LD script tag is in the initial HTML output (using `next/server` rendering helpers or a snapshot of the SSR string).
4. Update the file docstring to explicitly call out the Suspense boundary as part of the SSR contract so future edits don't reintroduce the bail-out.
5. Sanity-check other client components under `frontend/apps/reality-web/src/components/listings/**` that also read `useSearchParams()` (grep) — file follow-up backlog items if any lack a Suspense boundary but do not widen this plan.

## Alternatives considered
- **Migrate the listing-detail page to a server component that reads `searchParams` from the RSC props** — rejected because `ListingDetailContent` also owns interactive tabs/scroll-restore state that need client hooks; splitting responsibilities into a shell + reader keeps the SEO-critical markup in RSC without a full rewrite.
- **Add `export const dynamic = 'force-static'` on the route** — rejected because it does not resolve the `useSearchParams()` bail-out; Next.js will still opt the subtree out until a Suspense boundary is present, and force-static conflicts with the ISR + preview behaviour the route relies on.

## Root-cause trace
1. Symptom: JSON-LD `<script>` missing from the initial HTML of `/sk/listings/<slug>` (scrapers see no structured data); above-the-fold content that lives inside `ListingDetailContent` also arrives only after hydration.
2. ← `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx:122` renders `<ListingDetailContent>` directly, no `<Suspense>` wrapping.
3. ← `frontend/apps/reality-web/src/components/listings/ListingDetailContent.tsx:88` calls `useSearchParams()` at the component root, which by Next.js 16 App Router rules opts the entire subtree out of prerender until the closest Suspense boundary.
4. Origin: introduced when `ListingDetailContent` grew a client-side query-param reader (search-scroll / preview-mode) without a paired Suspense boundary; sibling `app/[locale]/listings/page.tsx` uses the Suspense idiom, so this route is the drifted case.

## Test plan
- [ ] New SSR unit test `frontend/apps/reality-web/src/components/listings/__tests__/ListingDetailContent.ssr.test.tsx` asserting the JSON-LD `<script type="application/ld+json">` block appears in the rendered SSR HTML for a stub listing.
- [ ] Snapshot check that the breadcrumb and agent-contact markup render server-side.
- [ ] Command: `cd frontend && pnpm -F reality-web test -- ListingDetailContent.ssr`

## Out of scope
- Redesigning the listing-detail layout, tab structure, or JSON-LD schema itself.
- Fixing other Suspense bail-out cases outside `components/listings/**` — those become their own backlog items.
- Any change to reality-server or its listing endpoints.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-listing-detail-suspense-bailout.md`
- Mark the matching `backlog.json` row as `status: "done"`
