# code-review-reality-web-en-us-price-formatting

**Vector:** bug
**Score:** 3
**Source:** dispatcher Tier-1d rotating-expert-review 2026-09-11 (reality-web ListingHeader + AgencyListings)
**Confidence:** medium

## Hypothesis
Two reality-web components (`ListingHeader.tsx` and `AgencyListings.tsx`) each define a module-local `formatPrice()` that calls `new Intl.NumberFormat('en-US', ...)` regardless of the current route locale. Both feed the most prominent money field on public, SEO-indexed pages, so every sk/cs/de/pl/hu visitor sees US-style number grouping (`,` thousands / `.` decimals / `$`-style symbol placement) instead of `1 234 €` / `1.234 €`. The shared `@/lib/format#formatPrice(value, locale, options)` already threads `bcp47(locale)` and defaults to EUR — the fix is to delete both inline formatters and call the shared helper with `useLocale()`, mirroring the canonical call at `ListingCard.tsx:106`.

## Evidence
- `frontend/apps/reality-web/src/components/listings/sections/ListingHeader.tsx:6-16` defines a local `formatPrice(price, currency)` that hardcodes `new Intl.NumberFormat('en-US', ...)` on both branches; consumed at `:47` for the headline price and `:53` for price-per-sqm on the listing-detail page (`useTranslations('listing')` is already in scope at `:19`, so `useLocale()` is trivial to add).
- `frontend/apps/reality-web/src/components/agency/AgencyListings.tsx:373-374` defines another local `formatPrice(price, currency)` that hardcodes `new Intl.NumberFormat('en-US', ...)`; consumed at `:397` for every row in the agency listings table.
- `frontend/apps/reality-web/src/lib/format.ts:5-16` header documents that this file exists precisely to replace scattered `Intl.NumberFormat('en-US', {...})` sites; `formatPrice(value, locale, options)` at `:48-59` calls `bcp47(locale)` and defaults to EUR + `maximumFractionDigits: 0`.
- Canonical call site to mirror: `frontend/apps/reality-web/src/components/listings/ListingCard.tsx:12-13,27,106` — `import { formatPrice } from '@/lib/format'`, `const locale = useLocale()`, `formatPrice(listing.price, locale, { currency: listing.currency })`.
- Cluster: consolidates signal ids `code-review-reality-web-listingheader-en-us-price` (score 2) and `code-review-reality-web-agencylistings-en-us-price` (score 1); both dispatcher Tier-1d 2026-09-11.

## Files
- `frontend/apps/reality-web/src/components/listings/sections/ListingHeader.tsx`
- `frontend/apps/reality-web/src/components/agency/AgencyListings.tsx`
- `frontend/apps/reality-web/src/lib/format.ts`
- `frontend/apps/reality-web/src/components/listings/ListingCard.tsx`

## Dependencies
<!-- No blocking dependencies — self-contained frontend change. -->

## Required capabilities
- [x] C1 — Systematic debugging
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser (Chrome MCP / Preview / playwright)
- [ ] C5 — ADB device
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

**Execution mode:** `Mode: cloud-ok`

## Repro steps
1. `pnpm --filter @ppt/reality-web dev` and open `/sk/listings/<any-id>` in a browser.
2. Observe the headline price on the listing-detail page: it renders as `€1,234,567.00` (en-US thousands `,`, decimal `.`, currency prefix) instead of the Slovak-locale expected `1 234 567 €` (space grouping, EUR symbol trailing, no decimals).
3. Repeat for `/sk/agency/<slug>` — the agency listings table price cell renders the same US-style formatting on every row.
4. Expected after fix: SK route renders `1 234 567 €`; DE route renders `1.234.567 €`; CS route matches CS conventions.

## Suggested approach
1. Open `frontend/apps/reality-web/src/components/listings/sections/ListingHeader.tsx`. Remove the local `formatPrice` at `:6-16`. Add `import { formatPrice } from '@/lib/format'` and `import { useLocale, useTranslations } from 'next-intl'` (extend the existing next-intl import). At the top of the component, add `const locale = useLocale();`.
2. Update the two call sites: replace `formatPrice(listing.price, listing.currency)` with `formatPrice(listing.price, locale, { currency: listing.currency })` at both `:47` (headline) and `:53` (price-per-sqm).
3. Open `frontend/apps/reality-web/src/components/agency/AgencyListings.tsx`. Remove the local `formatPrice` at `:373-374`. Add the same imports. If the component is server-rendered, propagate `locale` as a prop from the calling `[locale]/agency/...` page instead of `useLocale()`.
4. Update the call site at `:397` to pass `locale` (either from hook or prop) and `{ currency: listing.currency }`.
5. Grep the reality-web src tree for any other `Intl.NumberFormat('en-US'` occurrences and file follow-ups if found (out of scope for this PR — record as new signals if the pattern lingers elsewhere).
6. Run typecheck and unit tests: `pnpm --filter @ppt/reality-web typecheck && pnpm --filter @ppt/reality-web test`.
7. Verify visually in dev server across sk / cs / de / en routes on both listing-detail and agency pages.

## Alternatives considered
- **Add a `locale` argument to the local `formatPrice` and keep both copies** — rejected because it perpetuates the anti-pattern the shared helper was built to erase (the `lib/format.ts` header docs the same motivation); any future third component would duplicate the mistake again.
- **Ship a global codemod that rewrites every `Intl.NumberFormat` call across reality-web** — rejected because the scope and blast radius grow far beyond these two files (grep would need per-call-site review of currency vs. plain-number vs. percentage), and this cluster's two files are the highest-visibility offenders that have documented user impact today.

## Root-cause trace
1. Symptom: `€1,234,567.00` on `/sk/listings/<id>` and every `/sk/agency/<slug>` row for sk/cs/de/pl/hu visitors on production.
2. ← Immediate cause at `ListingHeader.tsx:9-11` and `AgencyListings.tsx:374` — hardcoded `new Intl.NumberFormat('en-US', ...)`.
3. ← Upstream cause: both components predate the introduction of the shared `lib/format.ts` locale-aware helper and were never migrated when `ListingCard.tsx:106` was.
4. Origin: pre-`lib/format.ts` scaffolding of the reality-web listing/agency pages (specific PR would need a `git blame` — not required for fix).

## Test plan
- [ ] Add / extend a unit test that renders `<ListingHeader listing={{price:1234567, currency:'EUR'}} />` with `next-intl`'s `NextIntlClientProvider` at `locale='sk'` and asserts the rendered price contains a non-breaking space thousands separator and trailing `€`, NOT `,` grouping. Suggested test file: `frontend/apps/reality-web/src/components/listings/sections/ListingHeader.test.tsx` (new).
- [ ] Add a similar unit test for `AgencyListings` at `locale='de'` asserting `.` thousands grouping and trailing `€`.
- [ ] Regression: keep the `en` case working (BCP47 map returns `en-GB` which uses `,` thousands + `£`/whatever the currency arg is → assert `€` is trailing there too).
- [ ] Run: `pnpm --filter @ppt/reality-web typecheck && pnpm --filter @ppt/reality-web test -- ListingHeader AgencyListings`.
- [ ] Full workspace check: `pnpm check` (biome) at repo root frontend/.

## Out of scope
- The `code-review-reality-web-price-map-i18n-hardcoded` signal (hardcoded Slovak strings on `/[locale]/price-map/page.tsx`) — separate defect, separate file, separate PR.
- Sweeping every `Intl.NumberFormat` call across the reality-web tree.
- Any changes to `@/lib/format` itself — the helper already does the right thing; only call sites need updating.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-en-us-price-formatting.md`
- Mark the matching `backlog.json` row (id `code-review-reality-web-en-us-price-formatting`) as `status: "done"`
