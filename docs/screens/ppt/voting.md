---
id: ppt/voting
name: Voting
product: ppt
sitemapRefs:
  mobile: mobile-voting
implementations:
  ppt-web:
    route: /voting
    component: VotingPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: complete
  mobile:
    component: VotingScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
endpoints:
  - voting_list
relatedScreens:
  - id: ppt/vote-detail
    rel: child
  - id: ppt/vote-create
    rel: child
  - id: ppt/home
    rel: parent
sharedComponents:
  - status-pill
  - quorum-tile
  - filter-sidebar
  - search-bar
  - segmented-control
  - bulk-action-bar
  - pagination
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/ppt-voting.html
    frame: loaded-1-selected-bulk-bar / empty / loading-6-skel / error-503
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: MobVotingScreen
useCases:
  - UC-04
epics:
  - Epic-5
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Manager chrome
- [ ] [w] PPT manager header with `Hlasovania` tab active
- [ ] [w] Breadcrumb `Hlasovania / Všetko`

### Page header
- [ ] [w] H1 "Hlasovania" + count chip "2 otvorené · 1 čaká kvórum · 4 uzavreté"
- [ ] [w] Right toolbar: "Exportovať výsledky CSV" secondary + "+ Nové hlasovanie" primary → `ppt/vote-create`

### Filter sidebar (220px)
- [ ] [w] **Stav** 6-state machine: Koncept · Plánované · Otvorené · Kvórum dosiahnuté · Uzavreté · Zrušené
- [ ] [w] **Typ**: Jediná voľba · Viacero možností · Áno-Nie · Poradie preferencií
- [ ] [w] **Kategória**: Plán opráv · Domový poriadok · Dodávatelia · Financie · Iné
- [ ] [w] **Účasť** range-slider 0–100%

### Toolbar
- [ ] [w] Search "Hľadať podľa názvu, kategórie alebo ID…"
- [ ] [w] Segmented "Všetky · Otvorené · Vyžaduje pozornosť · Mine"
- [ ] [w] Sort dropdown: Najbližší koniec ↓ default

### Card list (NOT table — denser, status-rich)
- [ ] [w] **Left side**: status pill + H3 title (single-line clamp 60ch) + 2-line excerpt + meta row (category chip + author + open date + countdown chip — amber <24h, danger <2h)
- [ ] [w] **Right side**: quorum + result tile (180×100) — large fraction "13/24" + label + split progress bar (success/danger for Yes-No, brand for single-choice) + "13 hlasovalo · 8 áno / 5 nie"
- [ ] [w] **Closed/Quorum-reached cards**: outcome chip ("Schválené 67% za" success-soft / "Zamietnuté" danger-soft / "Nedosiahlo kvórum" neutral)

### Bulk-action bar
- [ ] [w] Visible when ≥1 selected: Predĺžiť deadline · Pripomenúť hlasujúcim push · Archivovať · Exportovať výsledky

### Pagination
- [ ] [w] Per `forms/pagination.html` — "Zobrazujem 1–20 z 21"

### Mobile (RN)
- [ ] [m] `MobVotingScreen` — top bar with H1 + count chip, filter chips strip, card list with quorum mini tile per row, voted-status icon (check brand-soft when voted)

## States

- **Loaded**: 6 cards covering all 6 states; 1 selected with bulk bar; 1 closed approved + 1 closed rejected (outcome chips)
- **Empty**: ballot-box icon + "Zatiaľ žiadne hlasovania" + body + primary "+ Vytvoriť prvé"
- **Loading**: 6 skeleton cards (status-pill + 2 line + meta + quorum-tile skel)
- **Error 503**: danger tile + retry; toolbar + sidebar interactive

## Notes

### Broader context

UC-04 voting list. Quorum tile pattern matches `ui_kits/ppt-web/manager-dashboard.html` right-rail "Active votes · 2" tile — must round-trip with consistent thresholds + colors.

### Specific (recent)

- Quorum tile uses split bar logic: brand-only when single-choice, success/danger split when Yes-No, neutral fill below threshold + brand fill above when not yet quorum.
- Close countdown chip styling: default neutral, amber <24h, danger <2h (synchronized with vote-detail header).
- 6-state pillset uses `--status-vote-{state}-{bg|ink}` token pairs.
- Mobile voting screen reuses card pattern but compresses quorum tile to inline mini-bar.
- 2026-08-31 — realtime sync (PR #2889): the `votes` query root now auto-refetches on a `notification.created` frame with `category: votes`. Previously `WebSocketContext.eventToQueryKeys` keyed on dead `entity:*` names the api-server never emits (100% dead sync); PR #2889 added `categoryToQueryKeys.votes → ['votes']` and wired `App.tsx`'s `onEntityEvent`. REST wiring unchanged.

## Agent Log

<!-- newest entries on top -->

- 2026-08-31 — agent: screen-map-drift-pr-2889-ppt — reconcile drift from PR #2889 (realtime ws→query-invalidation fix). ppt-web `WebSocketContext` re-keyed cache invalidation to canonical `domain.action` events and its `notification.created` subscriber routes by `payload.category`, so `category=votes` now invalidates the `votes` root; `App.tsx` wires `onEntityEvent → queryClient.invalidateQueries`. No route/component/endpoint/status change — frontmatter unchanged; docs-only.

- 2026-06-08 — agent (CTO/PAP-19): built ppt-web `features/voting/VotingPage` (status-filtered vote list, quorum tile, create CTA) wired to `@ppt/api-client` `useVotes`/`useBuildings`; mounted `votingRoutes()` (`/voting`) in `AppRoutes.tsx` + lazyRoutes; flipped ppt-web buildStatus planned→shipped, apiStatus stub→complete, added route. Functional MVP (English labels); Slovak-localized design-system polish remains a follow-up.
- 2026-05-09 — agent: integrated Batch E (pages/ppt-voting.html list — 4 artboards: loaded-1-selected/empty/loading/error) + Batch F1 (MobVotingScreen); flipped ppt-web from n/a → planned + redesignStatus → in-progress (drift: route not in sitemap); mobile redesignStatus → in-progress; attached 2 designSources; populated 8 sections + 4 states + 4 notes; declared 7 sharedComponents; added 3 relatedScreens (vote-detail + vote-create children, ppt/home parent)
- 2026-05-08 — init: created from scan (source: sitemap)
