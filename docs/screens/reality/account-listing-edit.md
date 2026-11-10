---
id: reality/account-listing-edit
name: Edit Listing (Account)
product: reality
sitemapRefs: {}
implementations:
  reality-web:
    route: "/account/listings/[id]/edit"
    component: ListingEditPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens:
  - id: reality/account
    rel: parent
  - id: reality/listing-edit
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics: []
designSources: []
owner: reality-frontend
---

# Edit Listing (Account)

Owner-facing 5-step listing-edit wizard (Type+location → Details → Photos → Price → Summary), pre-populated from the existing listing. Lives at `frontend/apps/reality-web/src/app/[locale]/account/listings/[id]/edit/page.tsx` (default export `ListingEditPage`). All copy is localized via `useTranslations('pages.listingEdit.*')` across sk/cs/de/en/pl/hu.

API is wired to reality-server through the thin hand-written `@/lib/realtor-api` helpers (`getMyListing` to load, `updateListing` to save, with `RealtorApiError` surfaced inline) — hence `apiStatus: partial`: real wiring, but the helpers are explicitly marked for replacement by the generated client, and publishing (`active`) is moderation-gated rather than a direct write.

Note: distinct from `reality/listing-edit` (the realtor variant). Disambiguation still needed when humans add useCases/epics.

## Notes

### Broader context

The backend carries a listing lifecycle enum (`active`/`paused`/`draft` plus the terminal `sold`/`rented`/`archived`) that is wider than the wizard's 3-state status control (`active`/`inactive`/`draft`). The two mapping directions are the page's historical churn hotspot (#1762, #3197, #3203): getting either wrong silently downgrades a live or terminal listing to `draft`.

### Specific (recent)
- 2026-11-04 — PR #3217 extracted the last inline, triplicated, untested bit of `handleSave`: the optional-numeric-field coercion for `sizeSqm`/`rooms`/`floor`, into a unit-tested `_number.ts` helper (`resolveOptionalNumber`), completing the `_price.ts`/`_status.ts` de-churn pattern. It also fixed a latent correctness bug of the same silent-zero family as the price fix (#3197): the old inline `Number(x) || undefined` preserved a numeric `0` but silently dropped the same value when it arrived as the string `'0'` (the Decimal-serialized shape the backend can send). The helper now coerces number and string identically and omits only empty/`null`/`undefined`/non-finite values; `_number.test.ts` pins the regression. No route, build-surface, or API-surface change — pure refactor + test hardening on the shipped baseline.
- 2026-11-02 — PR #3212 extracted the backend-lifecycle ↔ form status mapping out of `page.tsx` into a unit-tested `_status.ts` helper (sibling of the already-extracted `_price.ts`), behind two functions: `mapListingStatusToForm` (backend → form; terminal/unknown values display as `draft`) and `resolveStatusUpdate` (form → backend on save; omits `status` when unchanged to preserve the real terminal state, and never sends `active` directly since publishing is moderation-gated). `_status.test.ts` pins the #1762 silent-downgrade regression in both directions. No route, build-surface, or API-surface change — pure refactor + test hardening on the shipped baseline.
- 2026-05-18 — audit: stub created from `page.tsx`.

## Agent Log

<!-- newest entries on top -->
- 2026-11-04 — agent: screen-map drift reconcile for PR #3217 ("extract listing-edit numeric coercion into tested `_number` helper"). Pure refactor of `page.tsx` `handleSave` — extracted the triplicated `sizeSqm`/`rooms`/`floor` optional-numeric coercion into unit-tested `_number.ts` (`resolveOptionalNumber`) and fixed a silent-zero bug (string `'0'` → dropped). Documented under Notes > Specific. No frontmatter change: `buildStatus: shipped` and `apiStatus: partial` both unchanged (no route/build/API-surface change). Docs-only.
- 2026-11-02 — agent: screen-map drift reconcile for PR #3212 ("extract status mapping into tested `_status` helper"). Corrected `component` (`AccountListingEditPage` → actual default export `ListingEditPage`) and `apiStatus` (`stub` → `partial`: the route wires real reality-server calls via `@/lib/realtor-api` `getMyListing`/`updateListing`). Documented the `_status.ts`/`_price.ts` extraction and the #1762 downgrade regression now pinned by `_status.test.ts`. Docs-only change; `buildStatus: shipped` unchanged.
- 2026-05-18 — agent: created stub for unmapped route.
