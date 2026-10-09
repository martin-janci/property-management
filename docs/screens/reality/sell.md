---
id: reality/sell
name: Sell · Add Listing (5-step wizard)
product: reality
implementations:
  reality-web:
    component: SellWizard
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: MSell
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: stub
relatedScreens:
  - id: reality/profile
    rel: parent
  - id: reality/listing-edit
    rel: sibling
sharedComponents:
  - wizard
  - stepper
  - radio-cards
  - chip-group
  - text-input
  - file-upload
  - phone-input
  - validation-patterns
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/sell.html
    frame: sell-5-step-wizard
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MSell (KMP)
useCases:
  - UC-31
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header + hero
- [ ] [w,m] Portal header + H1 "Predajte alebo prenajmite vašu nehnuteľnosť" + lede

### Stepper
- [ ] [w,m] 5-step horizontal stepper: 1 Typ → 2 Detaily → 3 Fotky → 4 Cena → 5 Zhrnutie
- [ ] [w,m] step-pill on each form-card heading shows "N / 5"

### Step 1 · Čo predávate (Type & location)
- [ ] [w,m] Sale/Rent segmented · property-type radio-cards (Byt / Dom / Pozemok / Komerčné / Garáž) · address combobox per `forms/address-combobox.html`

### Step 2 · Detaily nehnuteľnosti
- [ ] [w,m] Rooms · area · floor · year built · energy class · heating · parking · amenities (multi-checkbox grid) · move-in date

### Step 3 · Fotky a video
- [ ] [w,m] Multi-file dropzone per `forms/file-upload.html` + main-photo selector + per-photo caption + drag-to-reorder
- [ ] [w,m] Optional video URL (YouTube / Vimeo)

### Step 4 · Cena
- [ ] [w,m] Price input + price-per-m² auto-calc + negotiable toggle + commission-included toggle (sale only) + monthly fees breakdown (rent only)

### Step 5 · Zhrnutie + publikovať
- [ ] [w,m] Live preview of how listing will look · GDPR + ToS confirm checkboxes · "Publikovať" primary
- (reality-web) Seller contact is **not** collected here — the listing is associated with the authenticated realtor (`principal.user_id`) server-side, so contact derives from the realtor profile. The step-5 contact inputs (name/phone/email) were removed in #3022 (closes #3016); the original design's contact-preference radio-cards (Phone visible / Phone hidden / Form only) remain unimplemented.

### Right rail (≥1024px) — progress checklist
- [ ] [w] Aside card: 5-row checklist matching steps; each row checked when step validated

### Why list (visible when not in form)
- [ ] [w] 3-tile card after step 5: 1.2M monthly visitors · Verified buyers · Free until first inquiry

### Footer
- [ ] [w] Standard footer; [m] System bottom-nav

## States

- **Step 1–5** (5 distinct artboards via stepper navigation)
- **Validation error per step**: inline field errors + top banner if multiple
- **Submitting**: disabled fields + spinner on Publish
- **Published success**: success card "Inzerát publikovaný · D-2026-XXXX" + "Zobraziť inzerát" + "Pridať ďalší"

## Notes

### Broader context

UC-31 listing creation. Critical conversion funnel — every drop-off step costs revenue. Stepper provides reassurance + per-step save means users can leave and return.

### Specific (recent)

- This is a **separate wizard** from `reality/listing-edit` (which is for editing existing listings). Sell-flow is opinionated for first-time creation; edit-flow is direct field access.
- Photo step is the biggest drop-off historically — make it as frictionless as possible. Allow up to 25 photos but require min 3.
- Address combobox should geocode + reverse-geocode to enable map display on the listing.
- Energy class chip-group A–G uses same color treatment as listing-detail building passport.
- Mobile (KMP) uses a single-column step-by-step flow with `MSell` component.
- **(reality-web) Publish is now wired to the API (PR #3013).** The step-5 "Publikovať" button calls `createListing()` from `@/lib/realtor-api` and only shows the success screen on a resolved 2xx; failures render an inline `role="alert"` error banner and keep the wizard open, and the button is disabled + shows a "publishing…" label while in flight. apiStatus raised `stub → partial` for reality-web: listing creation persists, but photo/video upload (step 3) and several step-2 detail fields are still mock-only (not in the POST payload), so it is not yet `complete`. mobile-native `MSell` remains `stub`.
- **Endpoint path mismatch RESOLVED (reality-web, #3039).** `createListing()` now POSTs to `POST /api/v1/my/listings` (`portal_listings::create_listing`). Previously it POSTed to `/api/v1/listings`, a GET-only route (search/featured/categories/suggestions/{id} + POST {id}/view) → 405 Method Not Allowed, so Publish always failed. Fetch-level regression test at `src/lib/realtor-api.test.ts` pins the path. `endpoints:` stays `[]` because the create route is not yet registered in `@ppt/sitemap` (no `create_listing` operationId under reality-server); a future agent should register it in `frontend/packages/sitemap/src/data/reality-server/` and then record that operationId here.
- **Remaining payload-shape gaps (reality-web) — follow-up, NOT fixed in #3039.** The `ListingDraft` → `CreatePortalListingRequest` body still does not fully match: (a) the wizard sends `area` but the server reads `sizeSqm` (`size_sqm`), so the floor area is silently dropped; (b) `postalCode` is a required `String` on `CreatePortalListingRequest` but the sell wizard collects no postal-code field and omits it, so the POST will now 422 (missing field) instead of 405. These affect the broader `ListingDraft` contract (also used by `ListingForm` / `realtor/listings/new`) and the wizard form UI, so they are out of scope for the path fix and need their own follow-up.
- **(reality-web) Step-5 seller contact inputs removed (#3022, closes #3016).** Step 5 previously collected `contactName`/`contactPhone`/`contactEmail`, but `handlePublish` dropped them before POST (they were never in the `ListingDraft` body). Rather than add per-listing contact storage (`CreatePortalListingRequest` and the `Listing` model have no contact columns — would need a migration + contract change), #3022 removed the inputs, their step-5 validation, and the now-unused `EMAIL_RE`/`PHONE_RE`: the listing is associated with the authenticated realtor (`principal.user_id`) server-side, so contact comes from the realtor profile (`GET /api/v1/realtors/me`). Step 5 is now **summary + terms only**. Regression test `page.publish.test.tsx` pins that the inputs are gone and the POST payload carries no `contact*` keys. apiStatus stays `partial` — the removed fields were not part of the persisted payload, and the real gaps (step-3 photo/video upload, several step-2 detail fields mock-only) are untouched. The orphaned i18n keys (`pages.sell.fields.contact*` + `validation.name/phone/email*`) were intentionally left in all 6 locale files as harmless dead keys; cleaning them up is a future candidate.

## Agent Log

<!-- newest entries on top -->

- 2026-10-09 — agent: reconciled with PR #3022 (drop dead seller contact fields from /sell, closes #3016; merged 2026-10-05). reality-web step 5 no longer collects contactName/phone/email — they were dropped before POST anyway, and contact now derives from the authenticated realtor profile. Renamed Step 5 (Kontakt + zhrnutie → Zhrnutie + publikovať), annotated the checklist, and added a Notes bullet. Frontmatter unchanged (apiStatus stays `partial`: removed fields weren't part of the persisted payload; step-3 upload + step-2 detail gaps untouched). Flagged orphaned contact i18n keys across 6 locales as a cleanup follow-up. Docs-only; `/screens validate` clean.
- 2026-10-08 — agent: screen-map gate fix (#3039, follow-up). Reverted `endpoints:` back to `[]` — `POST /api/v1/my/listings` is a raw method+path, not a valid `@ppt/sitemap` operationId, so it failed `/screens validate` (screen-map.yml RED). The create route has no registered operationId under reality-server yet; documented the reconcile step in Notes. realtor-api.ts path fix + its regression test are unchanged. `/screens validate` now clean.
- 2026-10-08 — agent: fixed createListing() path mismatch (#3039). Client now POSTs to `/api/v1/my/listings` (was `/api/v1/listings`, GET-only → 405). Added fetch-level regression test `src/lib/realtor-api.test.ts` (fails on the old path). Flagged two remaining payload gaps (`area` vs `sizeSqm`; required `postalCode` not collected) as follow-up — out of scope for the path fix.
- 2026-10-07 — agent: reconciled with PR #3013 (sell-wizard persist fix). reality-web Publish now calls `createListing()` with await+try/catch/finally; adds saving/submitError states, inline error banner, and a `page.publish.test.tsx` regression test; i18n keys `sell.form.publishing` + `sell.form.submitFailed` added across locales. Frontmatter: reality-web apiStatus stub → partial (mobile-native untouched). Noted client/server path mismatch (`/api/v1/listings` POST vs server `/api/v1/my/listings`). Docs-only; `/screens validate` clean.
- 2026-05-09 — agent: bootstrapped from bundle (pages/sell.html — 5-step wizard with stepper + progress aside + mobile-new-pages.html MSell frame); 8 sharedComponents; sibling listing-edit
