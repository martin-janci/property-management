---
id: reality/agent-profile
name: Agent Profile (public)
product: reality
implementations:
  reality-web:
    component: AgentProfilePage
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: stub
  mobile-native:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: reality/listing-detail
    rel: parent
  - id: reality/agency-dashboard
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - listing-card
  - rating-stars
  - kv-list
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/agent-profile.html
    frame: default-with-listings-and-reviews / empty-no-listings / loading / error
useCases:
  - UC-49
  - UC-51
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Hero
- [ ] [w] Cover band + 88px circular avatar + H1 (agent name) + role + agency link + verified badge ("ID overený · 4.9 ★ 128")
- [ ] [w] Right side: "Kontaktovať" primary + "Sledovať" ghost (notify on new listings)

### Stats strip
- [ ] [w] 4-cell: Aktívne inzeráty (14) · Predané za 12m (32) · Avg odpoveď (1.2h) · Na portáli (6 rokov)

### Section · Inzeráty tohto makléra
- [ ] [w] H2 + "Všetkých 14 →" link
- [ ] [w] 3-up grid of listing-cards

### Section · Hodnotenia
- [ ] [w] H2 + "Všetkých 47 →" link + average rating display
- [ ] [w] Vertical list of review cards (avatar + reviewer name + 5-star rating + date + body text + verified-buyer pill when applicable)

### Footer
- [ ] [w] Standard portal footer

## States

- **Default**: 14 listings + 47 reviews + verified
- **Empty**: "Tento maklér zatiaľ nepublikoval inzerát" placeholder card
- **Loading**: hero + stats + section skeletons
- **Error**: danger tile + retry; hero retained

## Notes

### Broader context

UC-49 + UC-51 public-facing agent profile. Linked from every listing-detail (agent card click). Drives trust + conversion via verified badge + reviews. Read-only on public side; agents edit their profile in `reality/agency-dashboard` agent management.

### Specific (recent)

- Reviews are post-transaction only — buyer must have actually inquired and the agent must confirm meeting/transaction. Prevents review bombing.
- "Sledovať" creates a notification trigger (UC-45 saved-search analog for agent's new listings).
- Verified badge ties to UC-23 ID-verification; high-trust signal.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/agent-profile.html — 4 states: default / empty / loading / error); UC-49/51; parent listing-detail
