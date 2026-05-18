---
id: reality/careers
name: Careers
product: reality
implementations:
  reality-web:
    component: CareersPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: n/a
  mobile-native:
    component: MCareers
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: n/a
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/about
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - chip-group
  - listing-card
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/careers.html
    frame: careers-page
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MCareers (KMP)
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
- [ ] [w,m] H1 "Stavajme realitný trh, ktorý dáva zmysel." + lede

### Otvorené pozície (Open positions)
- [ ] [w,m] H2 + filter chips (All / Engineering / Design / Sales / Operations / Customer Support)
- [ ] [w] Vertical list of position rows (title + dept + location + employment type + Apply CTA)
- [ ] [m] Same as compact cards

### Čo ponúkame (What we offer)
- [ ] [w,m] H2 + 3-column or 1-column benefit grid (icon + title + description per item: flexible hours, remote-first, equity, learning budget, ...)

### Apply flow
- [ ] [w,m] Tap on position → opens modal or new screen with apply form (Name, Email, CV upload, Cover letter optional, GDPR consent)

### Footer
- [ ] [w] Standard portal footer; [m] System bottom-nav

## States

- **Default**: positions list + benefits
- **Empty (no open roles)**: "Práve nemáme otvorené pozície. Zanechajte nám e-mail a ozveme sa, keď sa niečo otvorí." + email-capture input
- **Apply submitting / success / error**: standard form patterns

## Notes

### Broader context

UC-42 talent acquisition. Low-traffic but conversion-critical for hiring. Apply form integrates with HR/ATS (Greenhouse/Lever/Personio) — confirm provider before implementing.

### Specific (recent)

- Department chips are filter-as-search; URL should reflect filter (`/careers?dept=engineering`).
- Position cards link to detail (out of scope as separate screen-map); for now, modal is fine.
- "Apply" can be a generic mailto fallback if ATS isn't ready.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/careers.html + mobile-new-pages.html MCareers frame)
