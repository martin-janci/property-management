---
id: reality/listings
name: Listings
product: reality
sitemapRefs:
  reality-web: reality-listings
implementations:
  reality-web:
    component: ListingsPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints:
  - listings_search
relatedScreens:
  - id: reality/listing-detail
    rel: child
  - id: reality/saved-searches
    rel: sibling
  - id: reality/favorites
    rel: sibling
  - id: reality/compare-properties
    rel: sibling
sharedComponents:
  - listing-card
  - filter-sidebar
  - chip-group
  - range-slider
  - segmented-control
  - map-pin
  - listing-popover
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/reality-web/listings.html
    frame: listings-main+empty+loading+error
useCases:
  - UC-31
  - UC-44
  - UC-45
  - UC-48
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Layout (≥1024px)
- [ ] [w] 3-column workspace: 280px filter sidebar / 50% map / 50% list
- [ ] [w] View-mode segmented control (Mapa / List / Mapa + List) — top-right of subhead chip row
- [ ] [w] Active-filter chip strip in subhead — each removable via × glyph; "+ ďalšie" overflow chip; brand-styled "saved-search" chip when applicable

### Filter sidebar (UC-31)
- [ ] [w] Header with "Vymazať všetko" link
- [ ] [w] Save-this-search primary button + alert toggle (e-mail · denne) under header — links to UC-45
- [ ] [w] Filter groups: Predaj/Prenájom segmented · Cena range · Izby chip-row (Garz./1/2/3/4/5+) · Plocha range · Typ chip-row (Byt/Dom/Pozemok/Komerčné)
- [ ] [w] Advanced expander (chevron rotates 180°): Poschodie range · Rok výstavby range · Energetická trieda multi-select chips (A–G) · Vybavenie checkbox cards (Výťah / Parkovanie / Balkón / Domáce zvieratá [rent only, disabled when sale] / Zariadený [rent only, disabled when sale]) · Vzdialenosť slider with pinned location
- [ ] [w] Active-count badge on advanced toggle ("5 aktívnych")

### Map column (UC-31, partial — provider TBD)
- [ ] [w] Mapbox-style muted tile (parks via radial gradients, roads via SVG, river path) — placeholder until real provider wired
- [ ] [w] Numeric price-cluster pins (rounded-rect, brand-600 fill, white text, white 2px border, drop shadow); selected pin → dark surface fill (#0f172a / inverted in dark), `scale(1.08)`; viewed pin → white surface
- [ ] [w] Cluster pins (pill radius, "12 listings" suffix muted)
- [ ] [w] Hover pin → highlight matching list card (scroll-sync)
- [ ] [w] Click pin → mini listing-card popover anchored above (radius 12, `0 16px 40px -12px` shadow, downward arrow tail, photo + badges + fav, price/title/meta, Detail (primary) + Kontakt (secondary) actions)
- [ ] [w] "Hľadať v tejto oblasti" pill button at top-center after pan
- [ ] [w] Map controls (zoom +/− and re-center) top-right; attribution `© Mapbox · OpenStreetMap` bottom-right

### List column
- [ ] [w] List header: result count + meta ("142 inzerátov v zobrazenej oblasti"); sort dropdown (Najnovšie / Cena ↑ / Cena ↓ / Najmenej zhliadnuté)
- [ ] [w] 2-column listing-card grid (collapses to 1 col below 1320px)
- [ ] [w] Card: 4:3 photo, badges top-left (FEATURED amber-on-dark + status), heart top-right (filled red when fav, optimistic toggle, UC-44)
- [ ] [w] Compare checkbox in card meta row (UC-48); when checked, card gets brand-600 outline `0 0 0 2px var(--accent)`
- [ ] [w] Hover/selected card → border-accent + lift `translateY(-1px)` + `shadow-card-hover`
- [ ] [w] Compare bar (UC-48) — sticky `#0f172a` footer over list-body when ≥2 selected, shows count + thumbnail strip with × per item + "Porovnať →" CTA

### Empty
- [ ] [w] State pane (replaces map+list area): magnifier-with-minus icon · "No listings match your filters" headline · contextual subhead (e.g. "Skúste rozšíriť cenový rozsah alebo odstráňte 'A' triedu") · primary "Vymazať filtre" + secondary "Uložiť toto vyhľadávanie aj tak" (UC-45 anyway-save)

### Loading
- [ ] [w] 6 skeleton cards (4:3 photo + 3 lines), shimmer; map gets shimmer overlay; filter panel stays interactive; sort/header titles skeleton-replaced

### Error · graceful degrade
- [ ] [w] Map column shows banner state (alert-triangle icon · "Map unavailable" · "Mapový provider neodpovedá (HTTP 503)" · primary "Skúsiť znova" + secondary "Pokračovať bez mapy"); list column remains fully functional with "list-only fallback" meta on header

## States

- **Empty**: Map+list replaced with single state pane in 2-col workspace (no-map): icon + "No listings match your filters" + contextual hint + Clear-filters / Save-anyway actions. Filter sidebar retained.
- **Loading**: Skeleton 6-card grid + map shimmer overlay (preserves attribution + zoom controls); filter rows reduce to skeleton bars. Reduced-motion → static gray fills, no shimmer animation.
- **Error**: Map column → alert-triangle state pane "Map unavailable — listings still searchable"; list column degrades to single-column fallback with "list-only fallback" meta. Filters remain active.
- **Success**: 142-result example with 8 single pins + 2 cluster pins, selected pin (€380k) showing popover above; 4 cards in 2×2 grid, 3 of them in compare with sticky compare bar.

## Notes

### Broader context

Search results page — the workhorse of UC-31. Bridges discovery (map browsing, filters), persistence (UC-45 save-search + alerts), shortlisting (UC-44 favorites), and side-by-side evaluation (UC-48 compare). Public anonymous-allowed; saving + alerts gates behind auth.

### Specific (recent)

- Map provider is TBD per project caveat — bundle uses a hand-drawn SVG mock (parks as radial gradients, roads as SVG paths, river as cyan stroke). Production needs Mapbox or Leaflet; the price-cluster pin pattern is provider-agnostic.
- Pin numeric format `€380k` truncates k/m suffix locale-aware. Cluster pin uses `Intl.NumberFormat`-friendly count + " listings" suffix (translatable per `sk/cs/de/en/pl/hu`).
- Selected pin swaps to inverted surface (`#0f172a` bg + white ink in light, `#fff` bg + `#0f172a` ink in dark) — opposite of normal accent pattern, intentional for selected emphasis.
- Popover arrow uses `transform: rotate(45deg)` square with selective borders to render the down-pointing notch — not an SVG triangle.
- Card-pin scroll-sync: hover map pin highlights matching `.lcard.hl`; hover card highlights pin via `pin.on`. Implementation must keep ID parity between pins and cards.
- Compare bar background `#0f172a` is one of the only "non-token" surfaces; in dark mode it shifts to `var(--bg-elevated)` with a top border. Bar count is plural-aware (sk/cs grammatical cases).
- Slider for distance uses 14px circular knob with brand-600 ring on white fill; `linear-gradient` track with `--accent` fill at 50% width. Pinned-location chip below shows the anchor (e.g. "FIIT STU · Mlynská dolina") with × to clear.
- Energetická trieda chips A–G are multi-select (chip-row with `.on` brand fill); rows D–G are off in the "main" example to imply realistic A–C-only filtering.
- Vybavenie rent-only checkboxes (Domáce zvieratá, Zariadený) carry `disabled` + `opacity:.55` and an inline " (len prenájom)" muted suffix when in sale-mode. Implementation must enable/disable based on segmented Predaj/Prenájom state.
- "Hľadať v tejto oblasti" pill is shown only after a manual pan (debounce 400ms) — not on initial render.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (ui_kits/reality-web/listings.html, 4 artboards: main + empty + loading + error); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (8 sections), all 4 states, design-specific notes; linked UC-31/44/45/48; declared 7 sharedComponents; added 3 relatedScreens
- 2026-05-08 — init: created from scan (source: sitemap)
