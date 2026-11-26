---
id: reality/journal
name: Journal · Magazine
product: reality
implementations:
  reality-web:
    component: JournalPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: MJournal
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
relatedScreens:
  - id: reality/home
    rel: parent
sharedComponents:
  - portal-header
  - portal-footer
  - article-card
  - newsletter-form
  - chip-group
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/journal.html
    frame: magazine-index
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MJournal (KMP)
useCases:
  - UC-13
  - UC-42
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Magazine head (hero band)
- [ ] [w,m] H1 "Reality, ktoré sa oplatí čítať" + lede + category chips (Trh · Investovanie · Bývanie · Mesto · Návody)

### Featured article
- [ ] [w] Large 2-col card with cover image + H2 title + 2-line excerpt + author + read-time
- [ ] [m] Single full-width hero card

### Najnovšie (Latest)
- [ ] [w] 3-up grid of article-cards (cover + category eyebrow + H3 + excerpt + author + date)
- [ ] [m] Vertical card list

### Newsletter signup
- [x] [w] Inline card rendered in an explicit **disabled** "not live yet" state — email input + submit button both `disabled`/`aria-disabled`, with an honest status note (`newsletterUnavailable`, wired via `aria-describedby`). No fake success and no silent email drop (PR #3764 removed the former no-op handler that flipped local `subscribed` state while dropping the address). No subscription endpoint exists (`endpoints: []`); drop `disabled` + the note and point the inputs at reality-server once one lands.
- [ ] [m] (design intent, MJournal unbuilt) Inline card "Nepremeškajte" + email input + GDPR checkbox + Subscribe primary

### Editor's picks + Most read (right rail)
- [ ] [w] 2-card stack: "Výber redakcie" (3 article rows) + "Najčítanejšie" (top-5 with rank numbers)
- [ ] [m] Below "Najnovšie" as separate sections

### Footer
- [ ] [w] Standard footer; [m] System bottom-nav

## States

- **Default**: as designed (1 featured + 6+ latest + 3 picks + 5 most-read)
- **Empty**: "Pripravujeme prvé články" + email-capture for launch notification
- **Loading**: 6 skeleton cards
- **Error**: danger tile + retry

## Notes

### Broader context

UC-13 + UC-42 content marketing. Drives organic traffic + builds editorial authority. Linked from header nav (Magazín) and footer. Articles from `reality/article-detail` (out of scope for reality side; design pattern from `ppt/article-detail`).

### Specific (recent)

- Article-detail screen-map for reality side is **not yet created** — `ppt/article-detail` provides a near-identical pattern (long-form + ToC + comments) that should port. Flag for follow-up.
- Newsletter signup: the reality-web form is **intentionally disabled** (no-op removed in PR #3764) because there is genuinely no subscription endpoint — reality-server has no newsletter route and this screen's `endpoints:` is empty. The honest "not live yet" copy is the `newsletterUnavailable` i18n key (replaced the misleading `newsletterSuccess`; `sk`/`en` real copy, other four carry the English placeholder per the journal block's convention). When wiring an endpoint, integrate with an email provider (Mailchimp / Resend / SendGrid) and drop both `disabled` attributes + the status note.
- Categories should match those used in `reality/article-detail` filter chips for round-trip consistency.

## Agent Log

<!-- newest entries on top -->

- 2026-11-26 — agent: screen-map drift reconcile for PR #3764 ("stop faking journal newsletter signup"; merged 2026-11-24). #3764 touched `app/[locale]/journal/page.tsx` + `page.test.tsx` + all six locale files without touching `docs/screens/reality/`. The reality-web newsletter form no longer fakes success: the old `handleSubscribe` (which only flipped local `subscribed` state and silently dropped the email) and the `email`/`subscribed` `useState` were removed; the email input + submit button now render `disabled`/`aria-disabled` with an honest `newsletterUnavailable` status note (`aria-describedby`). i18n `newsletterSuccess` → `newsletterUnavailable` across sk/cs/de/en/pl/hu. Updated the Functionality Checklist newsletter line + added a Notes > Specific bullet. Frontmatter unchanged: reality-web `buildStatus: in-progress` / `apiStatus: partial` / `redesignStatus: in-progress` (the journal has no endpoint and never did — making the no-op explicit removed a fake interaction, it did not change API surface or route); mobile `MJournal` untouched by #3764 (stays `in-progress`/`partial`). `endpoints:` kept empty (no reality-server newsletter route in `@ppt/sitemap`). `/screens validate` clean. Docs-only.
- 2026-05-09 — agent: bootstrapped from bundle (pages/journal.html + mobile-new-pages.html MJournal frame); UC-13/42; flagged need for `reality/article-detail` screen-map (not yet created)
