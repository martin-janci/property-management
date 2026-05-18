---
id: reality/cookies
name: Cookies Policy
product: reality
implementations:
  reality-web:
    component: CookiesPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: n/a
  mobile-native:
    component: MLegal[kind=cookies]
    buildStatus: in-progress
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
  - cookie-table
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/cookies.html
    frame: cookies-policy
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MLegal[kind=cookies] (KMP)
useCases:
  - UC-23
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header
- [ ] [w,m] H1 "Cookies a sledovanie" + last-updated date

### 5 paragraphs
- [ ] [w,m] § 1 Čo sú cookies
- [ ] [w,m] § 2 Naše kategórie (Strictly necessary · Analytics · Marketing · Preferences) — table with cookie name, purpose, duration, third party
- [ ] [w,m] § 3 Tretie strany (Google Analytics · Meta Pixel · Mapbox · Stripe)
- [ ] [w,m] § 4 Vaše voľby (link to consent banner re-open + browser settings + ppt/privacy-settings)
- [ ] [w,m] § 5 Doba uchovávania

### Re-open consent banner CTA
- [ ] [w,m] Sticky inline button "Otvoriť nastavenia cookies" — re-opens consent UI

### Footer
- [ ] [w,m] Standard footer

## States

- **Default**: only state — static; consent banner integration is the primary interactive surface

## Notes

### Broader context

UC-23 ePrivacy compliance (cookie consent). Must be linkable from cookie banner + footer.

### Specific (recent)

- § 2 cookie table is real data; should be auto-generated from a `cookie-registry.ts` config so listing stays in sync with implementation.
- "Otvoriť nastavenia cookies" re-opens the cookie banner consent panel — works via global helper `window.openCookieConsent()`.
- Third-party list (§ 3) drives subprocessors entry in `reality/privacy` § 4 — keep in sync.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/cookies.html + mobile-new-pages.html MLegal[cookies] frame)
