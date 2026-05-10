---
id: reality/about
name: About
product: reality
implementations:
  reality-web:
    component: AboutPage
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: n/a
  mobile-native:
    component: MAbout
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: n/a
relatedScreens:
  - id: reality/home
    rel: parent
sharedComponents:
  - portal-header
  - portal-footer
  - team-grid
  - milestone-list
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/about.html
    frame: about-page
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MAbout (KMP Android frame 412×892)
useCases:
  - UC-42
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Hero
- [ ] [w,m] H1 "Realitný trh, ktorý je spravodlivý voči obom stranám." with subhead

### Náš príbeh (Our story)
- [ ] [w,m] 2–3 paragraph long-form essay about portal founding, mission, fairness principles

### Tím (Team)
- [ ] [w] 4-up grid of team-member cards (avatar + name + role + 1-line bio); "Všetkých 28 →" link
- [ ] [m] Vertical list of team-member rows (compact)

### Míľniky (Milestones)
- [ ] [w,m] Vertical timeline: 2019 founding → 2021 SK national → 2023 CZ + AT expansion → 2026 (current state)

### Footer
- [ ] [w] Standard portal footer with locale + currency switch
- [ ] [m] System bottom-nav with Profil active

## States

- **Default**: only state — static long-form content, no async data
- **Loading**: not applicable (SSR/SSG)
- **Error**: 404 fallback if route mis-configured (handled by portal shell)

## Notes

### Broader context

UC-42 onboarding & help — the marketing-and-trust surface that grounds the "verified listings + building passport" value prop. Linked from the portal footer on every page; rarely visited but important for trust signals.

### Specific (recent)

- Hero copy is the brand's mission statement — keep verbatim across locales when possible.
- Team grid pulls from a CMS or static config; "Všetkých 28 →" implies a separate `reality/team` page (out of scope, deferred).
- Milestones list should auto-update when a new milestone is added; consider managing as data, not hardcoded HTML.
- KMP mobile variant uses Compose M3 list patterns for team + milestones (vs. web grid).
- Slovak hyphenation: long titles (e.g. "Stavajme realitný trh, ktorý dáva zmysel.") should wrap, not ellipsize.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/about.html + mobile-new-pages.html MAbout frame); set redesignStatus → in-progress on web + mobile-native; declared 4 sharedComponents; linked UC-42; parent reality/home
