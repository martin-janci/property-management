---
id: reality/price-map
name: Price Map
product: reality
implementations:
  reality-web:
    component: PriceMapPage
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: MPriceMap
    buildStatus: in-progress
    redesignStatus: in-progress
    apiStatus: partial
relatedScreens:
  - id: reality/home
    rel: parent
  - id: reality/listings
    rel: sibling
sharedComponents:
  - portal-header
  - portal-footer
  - choropleth-map
  - chip-group
  - kv-list
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/price-map.html
    frame: price-map-bratislava
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/mobile-new-pages.html
    frame: MPriceMap (KMP)
useCases:
  - UC-31
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Hero / page header
- [ ] [w,m] H2 "Cenová mapa" + lede explaining choropleth: average €/m² per district

### Filter strip
- [ ] [w,m] City segmented (Bratislava · Praha · Viedeň) · Property-type chips (Byt / Dom / Pozemok / Komerčné) · Predaj/Prenájom segmented · Time-window (1m / 3m / 12m / 5y)

### Map (full-width on web, 4:3 on mobile)
- [ ] [w,m] Choropleth: Bratislava districts as polygons color-coded by avg €/m² (legend below); Danube river path; major roads thin
- [ ] [w,m] Hover/tap district → tooltip with avg price · listing count · trend arrow ↑↓
- [ ] [w,m] Active pin marker for selected district

### Selected district card (right rail or below map on mobile)
- [ ] [w,m] District name · avg €/m² · 12mo trend · listing count · "Zobraziť X inzerátov →" link to filtered `reality/listings`

### Insights band
- [ ] [w,m] 3-tile band: "Najlacnejší okres · Vrakuňa €3,210" / "Najdrahší · Staré Mesto €6,180" / "Najrýchlejšie rastie · Ružinov +3.4%"

### Footer
- [ ] [w] Standard footer; [m] System bottom-nav

## States

- **Default**: Bratislava + Byt + Predaj + 12m
- **Loading**: choropleth shimmer + filter strip interactive
- **Error**: "Cenová mapa nedostupná" + retry; filter strip preserved
- **Empty (no data for filter)**: "Pre vybraný filter nemáme dosť údajov · Skúste iný typ alebo väčšie okno"

## Notes

### Broader context

UC-31 market-data discovery. Powerful SEO surface + funnel into `reality/listings` (each district click = filtered search). Refreshes monthly from listing aggregations.

### Specific (recent)

- Choropleth provider TBD — Mapbox supports vector-tile choropleth natively; Leaflet via L.geoJSON works too. Same provider as `reality/listings` map preferred.
- District polygon GeoJSON from OSM administrative-boundaries dataset (level 9 for SK). Pre-bake at build time, not runtime.
- Trend arrows follow same convention as `reality/home` neighbourhoods strip (success-600 ↑ / danger-600 ↓ / muted →).
- Mobile map is read-only (no edit / save); add a "Zdieľať mapu" share button.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: bootstrapped from bundle (pages/price-map.html + mobile-new-pages.html MPriceMap frame); UC-31
