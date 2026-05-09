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
- [ ] [w,m] Inline card "Nepremeškajte" + email input + GDPR checkbox + Subscribe primary

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
- Newsletter signup must integrate with email provider (Mailchimp / Resend / SendGrid) — confirm at handoff.
- Categories should match those used in `reality/article-detail` filter chips for round-trip consistency.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/journal.html + mobile-new-pages.html MJournal frame); UC-13/42; flagged need for `reality/article-detail` screen-map (not yet created)
