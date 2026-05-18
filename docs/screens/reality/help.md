---
id: reality/help
name: Help Center
product: reality
implementations:
  reality-web:
    component: HelpPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: stub
  mobile-native:
    component: MHelp
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: stub
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/contact
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - search-bar
  - accordion
  - chip-group
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/help.html
    frame: help-center
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MHelp (KMP)
useCases:
  - UC-42
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Hero
- [ ] [w,m] H1 "Ako vám môžeme pomôcť?"
- [ ] [w,m] Large search input "Hľadať v centre pomoci…" with autosuggest

### Category tiles
- [ ] [w] 4-up grid (Pre kupujúcich · Pre nájomníkov · Pre predajcov · Pre realitky) — each with icon + title + N article count
- [ ] [m] Vertical list with same data

### FAQ accordion
- [ ] [w,m] H2 "Často kladené otázky" + grouped accordion list (per `forms/accordion`-like pattern); 8–12 items with concise answers

### Cannot find an answer
- [ ] [w,m] Card with "Nenašli ste odpoveď?" + link to `reality/contact` (button) + chat-bubble icon

### Footer
- [ ] [w] Standard portal footer; [m] System bottom-nav

## States

- **Default**: all categories + FAQ expanded-on-click
- **Search active**: filtered FAQ + categories matching query
- **Empty search**: "Žiadne výsledky pre <query> · Skúste iné kľúčové slová alebo nás kontaktujte"

## Notes

### Broader context

UC-42 self-service support. Reduces contact-form load. Categories segment audience (buyer vs. seller vs. agent) for faster discovery.

### Specific (recent)

- FAQ search must support diacritics-insensitive matching (sk: "kúpa" matches "kupa").
- Each FAQ item should have a slug for direct linking (`/help#kupa-nehnutelnosti`); enables sharing answers.
- "Pre realitky" category links to `reality/for-agents` for the full pricing/feature page.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/help.html + mobile-new-pages.html MHelp frame)
