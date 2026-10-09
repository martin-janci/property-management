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
endpoints:
  - reports_submit
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
- [ ] [w,m] Optional attachments (screenshot proof) — reality-web control **disabled** pending an upload pipeline: a non-interactive "Prílohy (čoskoro)" placeholder (`aria-disabled="true"`, no `<input type="file">`) that collects and transmits nothing (#3017/#3019). mobile (MReport) still targets an active control.
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
- Attachments control is disabled with a "coming soon" hint and is **not** transmitted (follow-up #3017/#3019): the endpoint accepts pre-uploaded http(s) attachment URLs but no upload pipeline exists yet. This is why reality-web `apiStatus` stays `partial`, not `complete`. As merged in #3019, reality-web removed the old `attachments` `useState` + hidden `<input type="file">` (which collected `File` objects `handleSubmit` never posted — a reporter saw a success screen while evidence was silently dropped) and replaced it with a non-interactive `<div aria-disabled="true">` placeholder labelled *"Prílohy (čoskoro)"* with hint *"Pridávanie príloh bude čoskoro dostupné — prosím, opíšte dôkazy v popise vyššie."*. Two vitest regression guards lock this in: no `input[type=file]` renders (placeholder is `aria-disabled`), and the POST body carries no `attachments` key after a full form fill. A `TODO(#3017)` breadcrumb in `page.tsx` tracks restoring an active control wired to a presigned upload.
- Success card surfaces the created report reference id (PR for #3042): the 201 `{ report }` id is read back from `useSubmitReport` and rendered on the success card (`data-testid="report-reference-id"`) with a "quote this in a follow-up" hint. The server returns a raw UUID — no `R-2026-XXXX` scheme exists server-side yet, so the id is shown verbatim; the checklist's "ID R-…" wording is still aspirational on the *format* only.
- Sitemap: `POST /api/v1/reports` (backend `reality-server/src/routes/reports.rs`) is now registered in `@ppt/sitemap` `realityServerEndpoints` as operationId `reports_submit` (#3042), so it is referenced from this doc's `endpoints:`.
- Listing-reference auto-prefill happens when user clicks "Nahlásiť tento inzerát" from listing-detail; manual entry needed when accessed from footer.
- Anonymous reports allowed (Email optional) but processed slower.
- Attach moderation-queue admin UI is out of scope (separate `ppt/moderation-queue` future map).

## Agent Log

<!-- newest entries on top -->

- 2026-10-09 — agent: reconciled report-listing with the **merged** diff of PR #3019 (gh-issue-3017, "disable never-transmitted /report attachment control"). #3019 touched only the `/report` route (`frontend/apps/reality-web/src/app/[locale]/report/page.tsx` + `page.test.tsx`); earlier entries referenced it before it merged, so this pass pins the doc to the shipped UI: removed `<input type="file">` + `attachments` state, replaced by a disabled `aria-disabled="true"` "Prílohy (čoskoro)" placeholder with exact copy, plus two regression guards (no file input; POST body attachment-free). Tightened the Functionality Checklist attachment line and the Notes > Specific attachment bullet accordingly. Frontmatter statuses unchanged — reality-web `apiStatus` stays `partial` / `buildStatus` stays `in-progress` (attachment upload is still the only gap); mobile `MReport` untouched by #3019. `/screens validate` clean (167 maps, 0 errors). Docs-only.
- 2026-10-08 — agent: closed follow-up #3042. Registered `POST /api/v1/reports` in `@ppt/sitemap` `realityServerEndpoints` (operationId `reports_submit`) and added it to `endpoints:` here. reality-web `/report` success card now reads the 201 `report.id` back from `useSubmitReport` and renders it as a reference (raw UUID — no server-side `R-…` scheme yet). Added a regression test (surfaces id on success). apiStatus/buildStatus unchanged: attachment upload (#3017) is still the only gap keeping reality-web at `partial`.
- 2026-10-07 — agent: reconciled with PR #3014 (reality-web `/report` submit wired to `POST /api/v1/reports`) + follow-up #3017/#3019 (attachment control disabled). Frontmatter statuses unchanged after review: reality-web `apiStatus` stays `partial` (core submit + 429/404/generic error + pending states are now live, but attachment upload is unwired and the server report id is not surfaced — pre-PR the page had no API call at all, so this is an accuracy gain, not an enum change) and `buildStatus` stays `in-progress`. mobile-native (`MReport`) untouched by #3014. `endpoints: []` left empty on purpose: `POST /api/v1/reports` is not in `@ppt/sitemap` `realityServerEndpoints`, so adding its operationId would fail `/screens validate` — sitemap registration is the prerequisite (out of this docs-only scope).
- 2026-05-09 — agent: bootstrapped from bundle (pages/report.html + mobile-new-pages.html MReport frame); UC-23/31; parent reality/listing-detail
