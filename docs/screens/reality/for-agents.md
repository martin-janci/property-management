---
id: reality/for-agents
name: For Agents
product: reality
implementations:
  reality-web:
    component: ForAgentsPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: n/a
  mobile-native:
    component: MAgents
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: n/a
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/agency-dashboard
    rel: child
sharedComponents:
  - portal-header
  - portal-footer
  - feature-grid
  - pricing-table
  - cta-band
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/for-agents.html
    frame: for-agents-marketing
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MAgents (KMP)
useCases:
  - UC-49
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Hero
- [ ] [w,m] H1 "Profesionálne nástroje, jeden dashboard, žiadne kompromisy s overením." + lede + "Začať" primary CTA → registration / `reality/agency-dashboard` (onboarding empty state)

### Čo dostanete (What you get)
- [ ] [w,m] H2 + 6-cell feature grid (icon + title + 1-line description): Multi-agent dashboard · Listings management · Branding · CSV/XML import · Analytics · Verified-agency badge

### Cenník (Pricing)
- [ ] [w,m] H2 + 3-column pricing table: Solo (€29/mes · 5 inzerátov) · Tím (€89/mes · 25 inzerátov) · Enterprise (custom)
- [ ] [w,m] Compare-features rows below cards

### Customer logos / case studies
- [ ] [w,m] Optional band: 6 agency logos that use the platform + 1 testimonial quote

### CTA band
- [ ] [w,m] Bottom card: "Pripravený začať?" + Try-free + Contact-sales CTAs

### Footer
- [ ] [w] Standard footer; [m] System bottom-nav

## States

- **Default**: marketing page only — static content
- **Loading**: not applicable (SSR/SSG)
- **Error**: 404 fallback only

## Notes

### Broader context

UC-49 agency acquisition funnel. This is the public-facing marketing page; the actual agency tools live behind auth at `reality/agency-dashboard`. CTA conversion drives B2B revenue.

### Specific (recent)

- Pricing in EUR; show local currency for CZ (CZK) and AT (EUR same) based on locale.
- "Verified-agency badge" feature links to `reality/security` (the trust angle).
- Customer logos must have written permission per logo (legal); if not, omit or use category placeholders.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/for-agents.html + mobile-new-pages.html MAgents frame); UC-49; child agency-dashboard
