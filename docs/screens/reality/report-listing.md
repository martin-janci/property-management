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

- reality-web `/report` submit now POSTs to `POST /api/v1/reports` (UC-23) via `useSubmitReport` (`@ppt/reality-api-client`, mirrors `useCreateInquiry`); pending (`Odosiela sa…`), success, and error states — distinct copy for 429 rate-limit, 404 listing-not-found, and generic — are live (PR #3014). Before #3014 the form only set local `submitted=true` and every report was silently dropped client-side.
- `listing_id` (UUID) is resolved from a `?listing=`/`?listingId=` deep-link param first, otherwise from a UUID embedded in the free-text reference field; if none is found the POST is blocked with a user-facing message instead of faking success. The raw reference text is preserved inside the description so moderators don't lose it.
- Attachments control is disabled with a "coming soon" hint and is **not** transmitted (follow-up #3017/#3019): the endpoint accepts pre-uploaded http(s) attachment URLs but no upload pipeline exists yet. This is why reality-web `apiStatus` stays `partial`, not `complete`.
- Success card does not surface a report ID — the server's 201 `{ report }` id is not read back, so the checklist's "ID R-2026-XXXX" is still aspirational, not implemented.
- Sitemap gap: `POST /api/v1/reports` (backend `reality-server/src/routes/reports.rs`) is live and now called by the frontend, but it is **not** registered in `@ppt/sitemap` `realityServerEndpoints`. Register its operationId there first; only then can it be added to this doc's `endpoints:` without failing `/screens validate`.
- Listing-reference auto-prefill happens when user clicks "Nahlásiť tento inzerát" from listing-detail; manual entry needed when accessed from footer.
- Anonymous reports allowed (Email optional) but processed slower.
- Attach moderation-queue admin UI is out of scope (separate `ppt/moderation-queue` future map).

## Agent Log

<!-- newest entries on top -->

- 2026-10-07 — agent: reconciled with PR #3014 (reality-web `/report` submit wired to `POST /api/v1/reports`) + follow-up #3017/#3019 (attachment control disabled). Frontmatter statuses unchanged after review: reality-web `apiStatus` stays `partial` (core submit + 429/404/generic error + pending states are now live, but attachment upload is unwired and the server report id is not surfaced — pre-PR the page had no API call at all, so this is an accuracy gain, not an enum change) and `buildStatus` stays `in-progress`. mobile-native (`MReport`) untouched by #3014. `endpoints: []` left empty on purpose: `POST /api/v1/reports` is not in `@ppt/sitemap` `realityServerEndpoints`, so adding its operationId would fail `/screens validate` — sitemap registration is the prerequisite (out of this docs-only scope).
- 2026-05-09 — agent: bootstrapped from bundle (pages/report.html + mobile-new-pages.html MReport frame); UC-23/31; parent reality/listing-detail
