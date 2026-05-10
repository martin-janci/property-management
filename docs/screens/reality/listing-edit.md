---
id: reality/listing-edit
name: Listing Edit (5-step wizard)
product: reality
implementations:
  reality-web:
    component: ListingEditWizard
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: stub
  mobile-native:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: reality/profile
    rel: parent
  - id: reality/sell
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
  - address-combobox
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit.html
    frame: react-app-shell-loading-listing-edit-app.jsx
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit-app.jsx
    frame: full-react-wizard
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit-step-1.html
    frame: step-1-type+location
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit-step-2.html
    frame: step-2-details
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit-step-3.html
    frame: step-3-photos
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit-step-4.html
    frame: step-4-price
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit-step-5.html
    frame: step-5-summary
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit-stepper.html
    frame: stepper-state-only
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/listing-edit-mobile.html
    frame: mobile-variant
useCases:
  - UC-31
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Wizard chrome
- [ ] [w,m] Portal chrome + breadcrumb `Profil / Inzeráty / Upraviť`
- [ ] [w,m] H1 "Upraviť inzerát" + listing reference + "Verzia ako koncept" pill
- [ ] [w,m] Stepper 5-step: 1 Typ + lokalita → 2 Detaily → 3 Fotky → 4 Cena → 5 Súhrn
- [ ] [w,m] Auto-save indicator
- [ ] [w,m] Right toolbar: "Náhľad" ghost + "Zrušiť" link + (after step 5) "Publikovať zmeny" primary

### Step 1 · Typ + lokalita
- [ ] [w,m] Sale/Rent segmented · property-type radio-cards · address-combobox

### Step 2 · Detaily
- [ ] [w,m] Rooms · area · floor · year built · energy class · heating · parking · amenities · move-in date

### Step 3 · Fotky a video
- [ ] [w,m] Multi-file dropzone + main-photo selector + per-photo caption + drag-to-reorder + video URL

### Step 4 · Cena
- [ ] [w,m] Price + per-m² auto-calc + negotiable toggle + commission/fees breakdown

### Step 5 · Súhrn
- [ ] [w,m] Read-only recap + per-section "Upraviť →" + "Publikovať zmeny" primary

## States

- **Per-step** (5 distinct artboards available)
- **Mobile variant**: separate `listing-edit-mobile.html` for compact mobile flow
- **Validation error per step**: inline + top banner
- **Submitting**: fields disabled + spinner
- **Published success**: success card + back to profile

## Notes

### Broader context

UC-31 listing edit flow. Distinct from `reality/sell` (creation) — same field set but already-populated values + "Publikovať zmeny" instead of "Publikovať inzerát". Implementation: shared component with `mode: 'create' | 'edit'` prop.

### Specific (recent)

- Bundle ships **9 design files** for this surface: 5 individual step HTMLs + a stepper-only state HTML + a mobile-specific HTML + a React JSX app + an HTML shell loading the JSX. The JSX is the canonical interactive prototype; HTMLs are static state captures for review.
- Edit mode preserves the listing's draft/published status — partial edits go to a separate draft revision until "Publikovať zmeny" merges them into the live listing.
- Photo step in edit mode allows deleting existing photos; warning on delete shown if listing has active inquiries.
- Address change is sensitive — triggers re-verification flow if user changes city/district significantly.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (9 listing-edit files: 5 steps + stepper + mobile + JSX + shell); UC-31; parent reality/profile; sibling reality/sell
