---
id: reality/security
name: Security & Anti-fraud
product: reality
implementations:
  reality-web:
    component: SecurityPage
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: n/a
  mobile-native:
    component: MLegal[kind=security]
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: n/a
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/report-listing
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - tip-card
  - guarantee-grid
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/security.html
    frame: security-page
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MLegal[kind=security] (KMP)
useCases:
  - UC-23
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Hero
- [ ] [w,m] H1 "Ako sa chrániť pri kúpe a prenájme"

### 3 najčastejšie podvody na slovenskom trhu
- [ ] [w,m] H2 + 3 illustrated tip-cards: Falošný inzerát · Žiadosť o zálohu pred obhliadkou · Spoofovaný kontakt makléra
- [ ] [w,m] Each card: red-soft icon tile + scenario description + "Čo robiť" actionable bullet list

### Naše záruky (Our guarantees)
- [ ] [w,m] H2 + 4-cell grid: Verified agents (ID-checked) · Building passport · Price history · Mortgage pre-approval (matches reality/home value strip)

### Nahlásiť podozrivý inzerát CTA
- [ ] [w,m] Banner card linking to `reality/report-listing` with phone + email fallback

### Footer
- [ ] [w,m] Standard footer

## States

- **Default**: only state — static educational content

## Notes

### Broader context

UC-23 anti-fraud + trust signal. Linked from "Bezpečnosť" footer column on every page; complements `reality/report-listing` (the action) with education.

### Specific (recent)

- 3-scam list is SK-specific (deposit before viewing, fake-link emails impersonating agents). DE/CZ/AT may have different scam patterns — localize copy.
- "Nahlásiť" CTA must surface on every long page as sticky FAB or inline banner.
- Guarantees grid mirrors `reality/home` value strip — keep numbers in sync (1,120 verified agents, 98% passport, 5y history, 48h pre-approval).

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/security.html + mobile-new-pages.html MLegal[security] frame)
