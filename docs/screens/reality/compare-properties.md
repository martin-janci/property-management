---
id: reality/compare-properties
name: Compare Properties
product: reality
sitemapRefs:
  reality-web: reality-compare
implementations:
  reality-web:
    component: ComparePage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: reality/favorites
    rel: parent
  - id: reality/listings
    rel: sibling
  - id: reality/listing-detail
    rel: sibling
sharedComponents:
  - status-pill
  - data-table
  - modal-drawer
  - search-bar
  - empty-state
  - error-state
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/compare.html
    frame: compare-loaded+empty+loading+error+add-modal
useCases:
  - UC-48
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header + toolbar
- [ ] [w] Portal header + breadcrumbs (Obľúbené / Porovnanie)
- [ ] [w] H1 "Porovnanie" + count pill ("Porovnávame 3 zo 4") — max 4 listings per session
- [ ] [w] Right-toolbar ghost actions: "Pridať inzerát" (plus) → opens add-modal · "Zdieľať porovnanie" (share-network) · "Exportovať PDF" (download)

### Comparison table (≥768px)
- [ ] [w] Table card (radius 12, surface, border) with `cm-table` grid
- [ ] [w] First column = labels with icons (Fotka, Cena ↑, Cena za m² ↑, Typ, Izby ↓, Úžitková plocha ↓, Poschodie, Rok výstavby ↓, Energetická trieda ↑, Vykurovanie, Parkovanie, Vzdialenosť do centra ↑, Inzeruje, Aktualizované)
- [ ] [w] Subsequent columns = listings; remove-column × icon top-right of each
- [ ] [w] Best-in-row highlight: lowest price, lowest €/m², highest rooms/area/year, best EPC, shortest distance — `--success-soft-bg` cell + check-mark badge
- [ ] [w] Photo row: 4:3 mini photo + status badge (Predaj/Prenájom)
- [ ] [w] "Pridať inzerát" placeholder column (dashed border, plus icon, "Pridať inzerát" CTA) when count <4

### Mobile swiper (<768px)
- [ ] [m,w] Horizontal swiper of full-listing cards (one per page) with paging dots; each card shows the same row labels + values stacked vertically; sticky header with current "n/N" indicator

### Empty
- [ ] [w] Center card with 4-square grid icon + "Pridajte až 4 inzeráty na porovnanie" h2 + "Pridávajte inzeráty z obľúbených alebo z výsledkov hľadania a porovnávajte vedľa seba." body + primary "Prehliadať inzeráty" CTA

### Loading
- [ ] [w] Skeleton table — column count = saved compare-set size; each row shows 1 label-skeleton + N value-skeletons; shimmer

### Error
- [ ] [w] Center card: danger-tile + "Porovnanie sa nepodarilo načítať." + "Skontrolujte pripojenie a skúste to znova." + primary "Skúsiť znova"

### Add-listing modal
- [ ] [w] Scrim + centered modal (max-width ~520, radius 16)
- [ ] [w] Header: "Pridať inzerát do porovnania" + close ×
- [ ] [w] Search input (autofocus): "Hľadať podľa adresy, ID alebo titulu inzerátu…"
- [ ] [w] Two grouped lists: "Z obľúbených" + "Naposledy prezerané"
- [ ] [w] Each row: 4:3 thumb + name (`Address — N rooms, M m²`) + meta (district · €/m²) + price right-aligned
- [ ] [w] Already-in-compare rows show `.in` checked state with disabled add button (cannot add twice)
- [ ] [w] Tap available row → adds + closes modal + table re-renders

## States

- **Empty**: 4-square icon tile + "Pridajte až 4 inzeráty…" + body copy + "Prehliadať inzeráty" primary CTA. No table.
- **Loading**: Table skeleton with column count matching saved compare-set; row skeleton bars; shimmer; reduced-motion → static.
- **Error**: danger-50 tile + circle-info icon + retry CTA.
- **Success**: 3 listings in 4-column table (3 listings + 1 add-placeholder); best-of-row highlights on price, area, year, energy, distance.

## Notes

### Broader context

UC-48 property comparison — the side-by-side decision surface. Capped at 4 listings to keep cognitive load manageable. Source-of-truth is favorites + recent-views; user can also paste an URL in modal search. Table is column-major: each row = a feature, each column = a listing. Best-of-row highlights make differences scannable in <2 seconds.

### Specific (recent)

- Comparison is anchored to UC-44 favorites — most "add" actions originate from the favorites/listings checkbox, not the modal. The modal exists for "I have one in mind, let me search by address" and for re-adding after a remove.
- Best-of-row computation runs client-side using a `sort: 'asc' | 'desc'` hint per row (in the design's `ROWS` array). Implementation should avoid hardcoding direction by feature; expose it as table metadata so locale-specific rules (e.g. some markets prefer higher floor, others lower for traffic noise) can override.
- Mobile swiper kicks in below 768px; design intent is full-card-per-page paging, not horizontal-scroll table. Implementation must hide the desktop table and render the swiper conditionally — don't try to make the table responsive horizontally.
- `cm-pill` ("Porovnávame 3 zo 4") plural-aware in sk/cs (1/2-4/5+ inflections); use `Intl.PluralRules`.
- "Aktualizované" relative time uses sk Slovak humanizer ("Pred 2 dňami") — match the inquiries page humanizer for consistency.
- Energetická trieda comparisons: A < B < C ... < G — sort `asc` puts A on top. Render with the same colored A–G badges as listing-detail building passport.
- Add-modal in-state rows have `.in` styling (check-mark + disabled add-button); attempting to re-add throws no error, the row just doesn't respond. UI must convey this disabled state clearly (don't make it look unclickable for no reason).
- Export-PDF generates a printable comparison sheet — server-side per locale; should reuse the same template as favorites export when feasible (consistent branding).
- Share-comparison creates a public token URL (UC-48 future) — read-only, recipient sees the same table but no checkmarks for add/remove.
- DE strings: "Vergleichen" vs "Porovnanie" works; "Anzeige hinzufügen" vs "Pridať inzerát" runs longer — toolbar buttons must be icon-only on narrow widths or wrap.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (pages/compare.html — 4 main states + add-listing modal + mobile swiper + 14-row table); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (7 sections), all 4 states, design-specific notes; linked UC-48; declared 6 sharedComponents; added 3 relatedScreens
- 2026-05-08 — init: created from scan (source: sitemap)
