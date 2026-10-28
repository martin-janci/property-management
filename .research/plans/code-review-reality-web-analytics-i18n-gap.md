# code-review-reality-web-analytics-i18n-gap

**Vector:** bug
**Score:** 3
**Source:** code-review Phase 1.5 reality-web 2026-10-28 (file `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx:14-91`)
**Confidence:** high

## Hypothesis
The realtor `[locale]/realtor/analytics` page is a locale-aware route whose entire body is hardcoded English (page title, PERIODS chip labels, four StatCard labels, loading + error fallback) and whose four big-number stat tiles call `.toLocaleString()` with no locale argument, so a non-sk visitor sees English copy and whatever Node SSR defaults to (effectively en-US grouping) regardless of the active locale. This is the same regression class we just finished sweeping on sibling pages via PR #3166 (agency-profile) and PR #3179 (sell / realtor-public / price-map / listing-edit). The smallest change is to wire `useTranslations('pages.realtorAnalytics')` + `useLocale()` through this one page, route the four numeric stats through `formatNumber(value, locale)` from `src/lib/format.ts`, and add a six-language `pages.realtorAnalytics.*` block to each `messages/<locale>.json`.

## Evidence
- `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx:14-91` — raw English page body: `title: 'Analytics'`, PERIODS array `[{label:'Last 7 days'},…]`, StatCard labels `'Total listings'`, `'Active listings'`, `'Total views'`, `'Inquiries'`, loading state `'Loading metrics…'`, error fallback `'Could not load analytics.'`.
- `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx:85-88` — four `*.toLocaleString()` calls with no locale argument (confirmed by `grep -n toLocaleString` on the file).
- Sibling fix landed yesterday: PR #3179 (`gh-issue-3175: fix(reality-web): format remaining hardcoded sk-SK price displays with active locale`) — identical shape, uses `useLocale()` + `formatPrice`/`formatNumber` from `@/lib/format`.
- Sibling fix landed previously: PR #3166 (`gh-issue-3162: fix(reality-web): format agency profile price with active locale`) — identical regression test shape (vi.mock next-intl, assert de-DE grouping).
- `frontend/apps/reality-web/src/lib/format.ts` — already exports `formatPrice(value, locale)` and `formatNumber(value, locale)` helpers; no new infra needed.

## Files
- `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx`
- `frontend/apps/reality-web/src/lib/format.ts`
- `frontend/apps/reality-web/messages/sk.json`
- `frontend/apps/reality-web/messages/cs.json`
- `frontend/apps/reality-web/messages/de.json`
- `frontend/apps/reality-web/messages/en.json`

## Dependencies

(none — stand-alone frontend fix)

## Required capabilities
- [x] C1 — Systematic debugging (bug vector)
- [ ] C2 — Seed data
- [ ] C3 — Dev instance running
- [ ] C4 — Browser · **local-only**
- [ ] C5 — ADB device · **local-only**
- [x] C6 — Verification before completion
- [ ] C7 — Code-review reception

Mode: cloud-ok

## Repro steps
1. In `frontend/apps/reality-web/`, open the page as a de-DE visitor: `pnpm -F reality-web dev` and visit `/de/realtor/analytics` (any realtor id).
2. Observe: page title shows literal `"Analytics"` (English) regardless of locale; stat cards show numbers with en-US grouping (e.g. `1,234` not `1.234`); period chip reads `"Last 7 days"`.
3. Expected after fix: page title and chip labels render in the active locale's translation; numeric stats group thousands by locale (`de`: `1.234`, `sk`: `1 234`, `en`: `1,234`).

## Suggested approach
1. In `page.tsx`, add `import { useLocale, useTranslations } from 'next-intl';` and `import { formatNumber } from '@/lib/format';`.
2. Inside the component, add `const t = useTranslations('pages.realtorAnalytics'); const locale = useLocale();`.
3. Replace the five English literals (`'Analytics'`, period labels, four StatCard labels, loading/error strings) with `t('<key>')` calls — keep the key tree flat for easy translation review.
4. Replace the four `.toLocaleString()` call sites (lines 85-88) with `formatNumber(analytics.totalListings, locale)` etc.
5. Add `pages.realtorAnalytics.title`, `.periods.{last7d,last30d,…}`, `.stats.{totalListings,activeListings,totalViews,inquiries}`, `.loading`, `.errorFallback` to all six `messages/<locale>.json` files (sk/cs/de/en/pl/hu). Mirror the key names already used in `pages.sell.*` + `pages.priceMap.*` for consistency.
6. Add a regression test at `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.test.tsx` using the same shape as `realtor/[id]/page.test.tsx` (added in PR #3179): `vi.mock('next-intl', () => ({ useLocale: () => 'de', useTranslations: () => (key: string) => key }))`, render, assert that a known numeric stat uses German dot-grouping (`720.000`) and NOT Slovak space-grouping (`720 000`).
7. `pnpm -F reality-web test` and `pnpm -F reality-web check` to confirm green before pushing.

## Alternatives considered
- **Leave the English copy and only fix the number formatting** — rejected because the four StatCard labels are the most visible chrome on this page; a mixed state (localized numbers under English labels) looks worse than the current full-English fallback and would still mis-serve non-sk users on first contact. Also contradicts the pattern established by PR #3166/#3179.
- **Promote the whole reality-web `[locale]/*` route tree to a batched i18n sweep in one PR** — rejected because the previous sweep PRs (#3166, #3179) scoped tightly to one or two pages each and landed cleanly; the backlog already carries a separate reality-web churn-watch action (`pm-frontend-reality-web-locale-pages-churn-watch-2026-10-28`) that owns the broader audit question.

## Root-cause trace
1. Symptom: `/de/realtor/analytics` renders English headings and en-US-grouped numbers to a German visitor.
2. ← Immediate cause at `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.tsx:14-91`: the page body is raw English literals and `.toLocaleString()` with no locale.
3. ← Upstream cause: the page was authored before the project's localized-formatting infra (`@/lib/format` helpers + `useLocale()`) landed on sibling pages. No ESLint rule flags `.toLocaleString()` call-site without a locale argument, so the next frontend reviewer has no tripwire.
4. Origin: the page was added as part of the realtor dashboard scaffold (pre-locale-sweep); the generic locale-string regression was first flagged by `code-review-reality-web-formatprice-hardcoded-locale` (done 2026-10-17) and swept through sibling pages by #3166 / #3179.

## Test plan
- [x] `frontend/apps/reality-web/src/app/[locale]/realtor/analytics/page.test.tsx` — new vitest file, mirrors `realtor/[id]/page.test.tsx` from PR #3179. Mocks `next-intl` to `useLocale: () => 'de'`, renders the page with a seeded `analytics` fixture, asserts that a known value (e.g. `720000`) renders as `720.000` and does NOT match `/720[\s ]000/`.
- [x] Regression scenario: swap the mocked locale to `sk` in a second test case and assert the Slovak space-grouping form `720 000` renders — guards against a regression that always forces de.
- [x] Local command: `pnpm -F reality-web test src/app/\\[locale\\]/realtor/analytics/page.test.tsx` and the lint/typecheck gate `pnpm -F reality-web check && pnpm -F reality-web typecheck`.

## Out of scope
- The broader `pages.realtorAnalytics` content model (adding new metrics, period selector logic, backend contract changes) — this plan only localizes what the page already renders.
- Any backend `/api/v1/realtor/analytics` work.
- The six-language `messages/pl.json` / `messages/hu.json` block fill-in may be a one-line English fallback where translations do not yet exist; a translator pass is not in scope.
- The sibling `code-review-reality-web-saved-searches-price-locale` and `code-review-reality-web-inquiries-date-type-i18n` findings — tracked separately in backlog.

## After-merge
- Move this file to `plans/_archive/code-review-reality-web-analytics-i18n-gap.md`
- Mark the matching `backlog.json` row as `status: "done"`
