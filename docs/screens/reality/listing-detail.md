---
id: reality/listing-detail
name: Listing Detail
product: reality
sitemapRefs:
  reality-web: reality-listing-detail
implementations:
  reality-web:
    component: ListingDetailPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: KmpListingDetailScreen
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: stub
endpoints:
  - listings_get
relatedScreens:
  - id: reality/listings
    rel: parent
  - id: reality/favorites
    rel: sibling
  - id: reality/inquiries
    rel: sibling
sharedComponents:
  - listing-card
  - status-pill
  - tabs
  - timeline
  - map-pin
  - agent-card
  - mortgage-card
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/reality-web/listing-detail.html
    frame: listing-detail
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile-native/screens.jsx
    frame: KmpListingDetailScreen
useCases:
  - UC-31
  - UC-44
  - UC-46
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header + breadcrumbs
- [ ] [w] Sticky portal header (same as home / listings)
- [ ] [w] Breadcrumb path: Home › City › District › Address (current bold; brand-600 hover on links)

### Gallery
- [ ] [w] 3-column 2-row mosaic (`2fr 1fr 1fr`, two 240px rows): main hero photo (spans 2 rows) + 4 thumbnails — radius 14, 4px gap
- [ ] [w] Badges top-left of hero: FEATURED + status (For sale / For rent)
- [ ] [w] Save + Share floating actions top-right (white-95% surface, 8px radius)
- [ ] [w] "+ N photos · view tour" pill (rgba(0,0,0,0.65) + 8px backdrop blur) bottom-right of hero

### Title + price row
- [ ] [w] H1 title (26px / 700, -0.02em tracking) + address line with map-pin icon
- [ ] [w] Right-aligned price box: `€380,000` 26px/800 + per-m² + listed-N-days-ago meta line

### Pill row
- [ ] [w] Status pill (Published · active — uses status-anno-published tokens), feature pills (3 rooms · 78 m² · 4th of 8), info pills on accent-soft-bg (Verified agent · Building passport A)

### Quick stats strip
- [ ] [w] 4-cell card: Floor area / Built / Energy / Fees — each with uppercase label, 18/700 value, muted hint sub-line

### Tabs
- [ ] [w] In-page tab bar: Overview · Floor plan · Building passport · Neighbourhood · Price history (active = brand-600 + 2px underline)

### Sections (Overview tab)
- [ ] [w] **About this property** — multi-paragraph description with "Show full description →" expand link
- [ ] [w] **Specifications** — 2-column dl/dt/dd table (Property type, Tenure, Rooms, Bathrooms, Floor, Orientation, Heating, Parking, Storage, Move-in ready)
- [ ] [w] **Amenities** — 3-column checkmark grid + "All N →" expand link
- [ ] [w] **Building passport** — 2×2 grid of (Energy rating with A–G badge / HOA fund balance / Open faults / Planned assessments) + "Full report →" link
- [ ] [w] **Location** — 16:8 grid-pattern map preview with brand pin + walkable POI strip (tram / school / shopping / park with minute counts)
- [ ] [w] **Price & listing history** — left-rail timeline (current event with brand-600 dot + history with bordered dots): listing date, prior sale, original registration

### Right-rail aside (sticky)
- [ ] [w] **Agent card**: avatar + name + role + verified-badge (success-50 + success-600); 3-cell agent-meta (Active / Avg reply / On portal); contact row (Call primary + Message icon-btn + Save icon-btn); reply-time note
- [ ] [w] **Mortgage estimate** — accent-soft-bg card with assumptions line + monthly figure (22/800 brand-600) + "Start pre-approval →" CTA
- [ ] [w] **Verification + report** — bordered card stating verification date + "Something wrong? Report listing" danger-600 underlined link (target: a future report-listing screen-map; design exists in pages/report.html, screen-map TBD)

## States

- **Empty**: not depicted in design; for a missing listing (404), recommend portal-shell layout with `Listing not found` headline + "Browse listings" primary CTA + return-link to district. (Implementation note — match the header but skip gallery/aside.)
- **Loading**: not depicted; recommend gallery skeleton (mosaic shape) + title/price skeleton + 3 section skeletons; right-rail collapsed to single agent-card skeleton. SSR-first means most content is static; loading mainly affects history/passport async sections.
- **Error**: not depicted; per voice → blunt `Unable to load listing details` banner inside content grid with `Try again` CTA, retain header + breadcrumbs.
- **Success**: full populated detail as designed (5-image mosaic, 6 pills, 4-cell stats, all 7 sections, agent + mortgage + report aside).

## Notes

### Broader context

Single-property detail view — converts portal traffic into UC-46 inquiries (Call / Message via agent card) and UC-44 saves (heart). Building passport block is the differentiated asset; data flows from PPT (`reality-server` joins on the unit/building). Sticky agent aside is the primary conversion surface.

### Specific (recent)

- Gallery is `grid-template-columns: 2fr 1fr 1fr; grid-template-rows: repeat(2, 240px); gap: 4px; border-radius: 14px overflow:hidden`. Mobile (≤768px) collapses to single hero + horizontal-scroll thumb strip — design implies but does not depict; flag at implementation.
- Breadcrumb separator is `›` (Unicode `203A`), color `var(--border-strong)`. Last crumb gets weight 500 + `--fg-primary`.
- Building-passport energy badge: 24×24 inline-flex square with `--success-500` fill + bold A inside, then label trails right. Grade letter is the actual rating (A/B/C/D/E/F/G); ramp colors should follow Slovakia EPC convention (greens for A–B, yellow C, oranges D–E, reds F–G).
- Specs table uses `display: contents` on the inner `dl` so the dt/dd children participate in the parent grid — preserves accessible HTML semantics while getting a clean 2-column layout.
- Amenities checkmark uses an inline SVG data-URL with brand-600 stroke for the tick — when porting, convert to a small svg/component to retain dark-mode tinting.
- Map preview is a placeholder: 16:8 panel with a `60×60` grid pattern overlay + central pin. Production needs the same Mapbox/Leaflet provider as `reality/listings`. The pin tail is rendered with a `::after` triangle (border trick), not SVG.
- Right rail uses `position: sticky; top: 84px` (header 64 + breadcrumb 20) — must match the actual sticky header height in production to avoid jump.
- Price-history timeline uses `cur` event in brand-600 fill; previous events as bordered dots on `--bg-surface`. Newest entry is at top.
- POI strip currently uses emoji (🚋 🏫 🛒 🌳) — per SKILL.md these must be migrated to Lucide icons before shipping.
- Mortgage card border `#bfdbfe` is a hard-coded dawn shade — should derive from `--accent-soft-border` token or equivalent in dark mode.
- Verified badge `success-50` background only exists in light; in dark, drop to `rgba(16,185,129,.15)` per the dark-mode pairing rule.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (ui_kits/reality-web/listing-detail.html); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (8 sections), states, design-specific notes; linked UC-31/44/46; declared 7 sharedComponents; added 3 relatedScreens (report-listing forward ref deferred — screen-map not yet created)
- 2026-05-08 — init: created from scan (source: sitemap)
