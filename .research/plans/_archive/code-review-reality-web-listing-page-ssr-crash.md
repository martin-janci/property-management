# code-review-reality-web-listing-page-ssr-crash

**Vector:** bug
**Score:** 3
**Source:** rotating-expert-review reality-web 2026-06-14
**Confidence:** high

## Hypothesis

The public listing detail page (`app/[locale]/listings/[slug]/page.tsx`) builds JSON-LD structured data by dereferencing `listing.photos.map(...)`, `listing.address.street`, `listing.address.city`, `listing.price`, `listing.rooms`, `listing.area` etc. without runtime validation. `getListing` returns the raw `response.json()` body cast to `ListingDetail | null` — TypeScript types are not enforced at runtime, so any partial 200 body (`{}`, missing `address`, missing `photos`) crashes SSR on the core conversion route. The sibling module `metadata.ts:17-36` *already documents* this exact failure mode and defends `generateMetadata` against it; replicate that pattern in the page body before constructing `jsonLd`.

## Evidence

- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx:71-79` — `getListing` returns `response.json()` cast via TypeScript `as ListingDetail | null` with no runtime check
- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx:100-127` — JSON-LD build derefs `listing.photos.map((p)=>p.url)`, `listing.address.street/city/district/postalCode/country`, `listing.price`, `listing.currency`, `listing.rooms`, `listing.area`
- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/metadata.ts:17-36` — sibling defends `buildListingMetadata(listing: unknown)` with shape check (`!listing || typeof listing !== 'object'`, `typeof l.title !== 'string'`, etc.)
- Public Reality Portal listing detail is the core conversion route; an SSR 500 on a partial body would surface as Next.js error.tsx + lost SEO + lost lead

## Files

- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.tsx`
- `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/metadata.ts`

## Dependencies

## Required capabilities

- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode (auto-derived from the ticks):**
Mode: cloud-ok

## Repro steps

1. Make reality-server respond to `GET /api/v1/listings/<slug>` with a partial 200 body: `{ "title": "X", "slug": "x" }` (no `address`, no `photos`).
2. Hit the Next.js page `GET /<locale>/listings/x` (SSR).
3. Expected: page renders with a "Listing Not Found" or fallback experience; SSR does not crash.
4. Actual: page throws `TypeError: Cannot read properties of undefined (reading 'map')` at `page.tsx:106` (`listing.photos.map`) and the Next.js error boundary serves 500.

## Suggested approach

1. Extract a `buildListingJsonLd(listing: unknown): object | null` helper into a new module `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/jsonLd.ts`, mirroring the structure of `metadata.ts` (returns `null` when listing is malformed).
2. Inside the helper: narrow `listing` to `object`; pull `l = listing as Partial<ListingDetail>`; require `l.title`, `l.slug`, and a non-null `l.address` with `street/city/country` strings before constructing JSON-LD. Treat `l.photos`, `l.price`, `l.rooms`, `l.area` as optional and emit them only when present.
3. In `page.tsx`, replace the inline `jsonLd` literal with `const jsonLd = buildListingJsonLd(listing);` and skip the `jsonLd` prop in `<ListingDetailContent listing={listing} jsonLd={jsonLd} />` when it's `null`.
4. Either (a) update `ListingDetailContent` to accept `jsonLd: object | null` and skip the `<script type="application/ld+json">` tag when null, or (b) emit nothing in the page when null (whichever needs the smaller diff — inspect `ListingDetailContent.tsx` to choose).
5. Add Vitest coverage in a new sibling `jsonLd.test.ts` mirroring the metadata test pattern: fully-shaped body builds the object, `{}` returns null, missing `address` returns null, missing `photos` builds JSON-LD without the `image` field.

## Alternatives considered

- **Move the entire page body behind a zod-validated `ListingDetail` schema** — rejected because the rest of the page (the visual rendering inside `ListingDetailContent`) already tolerates partial bodies via React's optional-chaining; introducing zod just for JSON-LD adds a dep and a runtime parse on the hot path for no additional safety.
- **Wrap the JSON-LD block in a try/catch and silently swallow** — rejected because that hides the real bug from observability and still emits the empty `<script>` tag (or no tag at all) without ever logging the malformed body upstream; the explicit shape check is both safer and surfaces the upstream issue in logs.

## Root-cause trace

1. Symptom: SSR 500 on `GET /<locale>/listings/<slug>` for any malformed 200 body from reality-server.
2. ← `page.tsx:106` — `listing.photos.map((p) => p.url)` throws `Cannot read properties of undefined (reading 'map')`.
3. ← `page.tsx:93` — `const listing = await getListing(slug, host);` returns the raw fetch JSON body. Type cast `ListingDetail | null` provides no runtime check.
4. ← `page.tsx:76` — `return response.json();` casts via TS only; the only guard is `!response.ok`, which doesn't catch a 200 with a body that doesn't match the schema.
5. Origin: introduced when JSON-LD was added to the listing page (commit history shows `app/[locale]/listings/[slug]/page.tsx` has had the literal JSON-LD block since `metadata.ts` was extracted — the page was never updated to mirror the metadata-side defense).

## Test plan

- [ ] `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/jsonLd.test.ts` — fully-shaped body → object with all fields; `{}` → null; missing `address` → null; missing `photos` → object without `image`.
- [ ] Regression in `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/page.test.tsx` (or add one) — render the default export against a mocked `getListing` that returns `{ title: 'X', slug: 'x' }`; assert no throw and that `<ListingNotFound />` is rendered.
- [ ] Local: `pnpm --filter @ppt/reality-web test -- jsonLd page` to run just the affected suites.

## Out of scope

- Adding zod / runtime schema validation across the whole reality-web fetch layer (separate refactor).
- Touching `ListingDetailContent.tsx` beyond the minimal `jsonLd` prop adjustment.
- The reality-server API contract — fixing the upstream to never return partial bodies is a separate backend story.

## After-merge

- Move this file to `plans/_archive/code-review-reality-web-listing-page-ssr-crash.md`
- Mark the matching `backlog.json` row as `status: "done"`
