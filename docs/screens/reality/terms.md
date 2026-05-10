---
id: reality/terms
name: Terms of Service
product: reality
implementations:
  reality-web:
    component: TermsPage
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: n/a
  mobile-native:
    component: MLegal[kind=terms]
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: n/a
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/privacy
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - legal-toc
  - section-numbering
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/terms.html
    frame: terms-of-service
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MLegal[kind=terms] (KMP)
useCases:
  - UC-23
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header
- [ ] [w,m] H1 "Podmienky používania" + last-updated date + version pill

### Right rail · ToC (≥1024px only)
- [ ] [w] Sticky table-of-contents with 8 § sections; current section highlighted brand-600 on scroll

### 8 paragraphs
- [ ] [w,m] § 1 Úvod a definície
- [ ] [w,m] § 2 Registrácia účtu
- [ ] [w,m] § 3 Pravidlá inzerátov (with sub-list of forbidden content)
- [ ] [w,m] § 4 Verifikácia identity
- [ ] [w,m] § 5 Poplatky a platby
- [ ] [w,m] § 6 Práva duševného vlastníctva
- [ ] [w,m] § 7 Zakázané správanie
- [ ] [w,m] § 8 Zrušenie účtu

### Footer
- [ ] [w,m] "Otázky? Kontakt: legal@reality-portal.sk" + acceptance footer

## States

- **Default**: only state — static content with print-friendly CSS
- **Print**: hide nav + ToC, expand all collapsed sections, ensure section breaks

## Notes

### Broader context

UC-23 GDPR + ToS compliance. Legal text should be reviewed by SK/CZ/AT counsel before publish; do not auto-translate via machine.

### Specific (recent)

- Paragraph numbering format `§ N` matches SK legal convention; preserve in CS but adapt to DE (`§ N`) or EN (`Section N`) per locale.
- Each § has stable `id` for direct linking + ToC scroll-spy.
- Last-updated date drives a banner on next-login if changed since user's last_accepted timestamp ("Aktualizovali sme podmienky · Skontrolujte zmeny").
- Versioning: keep an immutable copy per version (e.g. `/terms/v2026-05`); current version always at `/terms`.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/terms.html + mobile-new-pages.html MLegal[terms] frame)
