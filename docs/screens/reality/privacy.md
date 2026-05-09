---
id: reality/privacy
name: Privacy Policy
product: reality
implementations:
  reality-web:
    component: PrivacyPage
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: n/a
  mobile-native:
    component: MLegal[kind=privacy]
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: n/a
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/terms
    rel: sibling
  - id: reality/cookies
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - legal-toc
  - section-numbering
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/privacy.html
    frame: privacy-policy
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MLegal[kind=privacy] (KMP)
useCases:
  - UC-23
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header
- [ ] [w,m] H1 "Zásady ochrany súkromia" + last-updated date + version pill

### Right rail · ToC (≥1024px)
- [ ] [w] Sticky ToC with 8 §; current section highlighted on scroll

### 8 paragraphs
- [ ] [w,m] § 1 Kto sme (data controller info)
- [ ] [w,m] § 2 Aké údaje zbierame (categories: account, listings, behavioral, technical)
- [ ] [w,m] § 3 Na čo ich používame (lawful basis per category)
- [ ] [w,m] § 4 Zdieľanie s tretími stranami (subprocessors list)
- [ ] [w,m] § 5 Cookies (links to reality/cookies)
- [ ] [w,m] § 6 Vaše práva podľa GDPR (8 rights with action links)
- [ ] [w,m] § 7 Bezpečnosť dát (encryption, retention, breach notification)
- [ ] [w,m] § 8 Doba uchovávania (retention table)

### Footer
- [ ] [w,m] "Otázky? Kontakt: gdpr@reality-portal.sk · DPO: Meno Priezvisko"

## States

- **Default**: only state — static content with print-friendly CSS

## Notes

### Broader context

UC-23 GDPR Art. 12–14 transparency. Required surface; must be linked from registration flows + footer + cookie banner. Subprocessors list (§ 4) must update with engineering changes (Stripe, AWS, Mapbox, etc.) — track as data, not hardcoded.

### Specific (recent)

- § 6 GDPR rights (access / rectify / erase / restrict / portability / object / withdraw consent / lodge complaint) each get an action link to the relevant `ppt/privacy-settings` flow OR contact endpoint.
- § 8 retention table is a real table with category × retention-period × lawful-basis columns.
- Versioning policy same as `reality/terms`: immutable per-version archive.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/privacy.html + mobile-new-pages.html MLegal[privacy] frame)
