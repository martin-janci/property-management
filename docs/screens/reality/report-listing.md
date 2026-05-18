---
id: reality/report-listing
name: Report Listing
product: reality
implementations:
  reality-web:
    component: ReportListingPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: MReport
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
relatedScreens:
  - id: reality/listing-detail
    rel: parent
  - id: reality/security
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - radio-cards
  - text-input
  - file-upload
  - validation-patterns
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/report.html
    frame: report-listing-form
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MReport (KMP)
useCases:
  - UC-23
  - UC-31
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header
- [ ] [w,m] Portal chrome + breadcrumb + H1 "Nahláste podozrivý inzerát"

### Form
- [ ] [w,m] **Čo je problém?** — radio-cards: Falošný inzerát · Žiadosť o zálohu · Spoofovaný kontakt · Diskriminácia · Iné
- [ ] [w,m] Listing reference (auto-prefilled from URL or manual paste)
- [ ] [w,m] Description textarea (min 30 chars)
- [ ] [w,m] Optional attachments (screenshot proof)
- [ ] [w,m] Reporter contact (Email + Phone optional)
- [ ] [w,m] GDPR + ToS consent checkbox
- [ ] [w,m] Submit "Nahlásiť" primary

### Submit success
- [ ] [w,m] Success card: "Hlásenie prijaté · ID R-2026-XXXX" + "Spracujeme do 24 hodín" + back to listing CTA

### Footer
- [ ] [w] Standard footer; [m] System bottom-nav

## States

- **Default**: form empty, submit disabled until required fields validate
- **Submitting**: fields disabled, spinner
- **Success**: confirmation card with ID
- **Error (server)**: top banner + retry; fields preserved

## Notes

### Broader context

UC-23 anti-fraud action surface. Complement to `reality/security` (educational). Reports go to a moderation queue handled by the support team.

### Specific (recent)

- Listing-reference auto-prefill happens when user clicks "Nahlásiť tento inzerát" from listing-detail; manual entry needed when accessed from footer.
- Anonymous reports allowed (Email optional) but processed slower.
- Attach moderation-queue admin UI is out of scope (separate `ppt/moderation-queue` future map).

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/report.html + mobile-new-pages.html MReport frame); UC-23/31; parent reality/listing-detail
