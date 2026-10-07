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
- [ ] [w,m] 5-step horizontal stepper: 1 Typ → 2 Detaily → 3 Fotky → 4 Cena → 5 Kontakt + zhrnutie
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

### Step 5 · Kontakt + zhrnutie
- [ ] [w,m] Contact preference radio-cards (Phone visible / Phone hidden / Form only) · live preview of how listing will look · GDPR + ToS confirm checkboxes · "Publikovať" primary

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
- **Endpoint path mismatch to verify (reality-web).** `createListing()` POSTs to `/api/v1/listings`, but the reality-server create route is `POST /api/v1/my/listings` (`create_listing`); `/api/v1/listings` only serves GET search/featured/categories/suggestions/{id} + POST {id}/view. `endpoints: []` left unchanged pending confirmation of the correct route/operationId; a future agent should reconcile the client path and then record the endpoint ID here.

## Agent Log

<!-- newest entries on top -->

- 2026-10-07 — agent: reconciled with PR #3013 (sell-wizard persist fix). reality-web Publish now calls `createListing()` with await+try/catch/finally; adds saving/submitError states, inline error banner, and a `page.publish.test.tsx` regression test; i18n keys `sell.form.publishing` + `sell.form.submitFailed` added across locales. Frontmatter: reality-web apiStatus stub → partial (mobile-native untouched). Noted client/server path mismatch (`/api/v1/listings` POST vs server `/api/v1/my/listings`). Docs-only; `/screens validate` clean.
- 2026-05-09 — agent: bootstrapped from bundle (pages/sell.html — 5-step wizard with stepper + progress aside + mobile-new-pages.html MSell frame); 8 sharedComponents; sibling listing-edit
