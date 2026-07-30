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
    component: ListingDetailScreen
    buildStatus: in-progress
    redesignStatus: applied
    apiStatus: partial
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
epics: []
diagrams: []
owner: reality-frontend
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
- **Error**: not depicted; per voice → blunt `Unable to load listing details` banner inside content grid with `Try again` CTA, retain header + breadcrumbs. SSR note: `generateMetadata` must never throw on a malformed/partial 200 body — it degrades to the `Listing Not Found - Reality Portal` fallback metadata (see `metadata.ts` `buildListingMetadata` / `FALLBACK_METADATA`) rather than crashing the request.
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
- Layout ISR revalidation (write-side of the resolved-layout rendering above): reality-web exposes an internal `POST /api/layout-revalidate` webhook — `frontend/apps/reality-web/src/app/api/layout-revalidate/route.ts` (PR #2431, `feat(layout): publish webhook`; hardened by PR #2497). The api-server calls it after a layout is published. Auth/verification order (all failures short-circuit before `revalidateTag`): (1) 503 `disabled` when `LAYOUT_WEBHOOK_SECRET` is unset; (2) 413 when the raw body exceeds `MAX_BODY_BYTES` (16 KiB) — the size cap runs *before* any HMAC/JSON work so an oversized unauthenticated body can't force an unbounded pass; (3) **replay protection (issue #2485)** — the delivery must carry an `X-Webhook-Timestamp` (unix seconds) that is fresh within `TOLERANCE_SECS` (±300 s of the receiver clock), else 401; (4) the `X-Webhook-Signature: sha256=<base64 HMAC>` is verified over the **timestamped payload `"{timestamp}.{body}"`** (not the raw body — the timestamp is folded into the HMAC so a captured delivery can't be replayed or re-timestamped), 401 on mismatch (constant-time compare); (5) 422 on a malformed `{ "screen": "reality/<slug>" }` payload. On success it maps the screen id to a `layout:<slug>` cache tag via `layoutTagsFor` and calls `revalidateTag`, refreshing this page's resolved-layout section registry after a CMS publish. **Observability (issue #2532):** every return path fire-and-forgets a `layout_revalidate_received` analytics event (via shared `trackEvent`, never throws / never changes the HTTP outcome) carrying `revalidate_outcome` (`revalidated` | `disabled` | `invalid_signature` | `invalid_body` | `invalid_screen`), `http_status`, `target_tenant` (the request host), `screen`, `tags`, and an optional `delivery_id` echoed from the webhook body (gap D) to correlate with the api-server `layout_change_published` / `layout_webhook_dispatched` events. It is an internal infrastructure endpoint, not a reachable screen, so per `docs/screens/README.md` ("a screen-map describes a reachable screen") it intentionally has **no standalone screen-map** — same treatment as `src/app/api/health/route.ts`. It also stays out of the `endpoints:` frontmatter: that list holds `@ppt/sitemap` operationIds (backend OpenAPI ops), and this is a reality-web-internal Next.js route, not a sitemap op.
- SSR metadata is built defensively in a standalone, component-free module: `frontend/apps/reality-web/src/app/[locale]/listings/[slug]/metadata.ts` exposes `buildListingMetadata(listing: unknown)` (+ `FALLBACK_METADATA`), unit-tested in `metadata.test.ts`. `getListing` returns the raw JSON body of any 200 response, so a truthy-but-malformed body (e.g. `{}`, missing `title`/`address`/`description`) is treated as `unknown` and validated before nested access; the page's `generateMetadata` just delegates to the helper. When changing title/og-image/description shape, edit the helper (not `page.tsx`) so the regression test stays the contract.

## Agent Log

<!-- newest entries on top -->

- 2026-07-30 — agent: resolved screen-map drift for PR #2497 (layout-revalidate hardening). PR changed the internal `POST /api/layout-revalidate` webhook (`src/app/api/layout-revalidate/route.ts`): added replay protection (issue #2485 — `X-Webhook-Timestamp` freshness ±300 s + signature now over the timestamped payload `"{timestamp}.{body}"` instead of the raw body), a 16 KiB body-size cap (413) ahead of any HMAC/JSON work, and fire-and-forget `layout_revalidate_received` analytics on every return path (issue #2532, outcome/http_status/target_tenant/screen/tags/delivery_id). Updated the Notes > Specific write-side note to match the new verification order and observability; the prior note's "signature computed over the raw body" was stale. Decision unchanged from the PR #2431 reconciliation: internal infra endpoint → no standalone screen-map (README "a screen-map describes a reachable screen"; mirrors `api/health`); documented here as the write-side of this screen's resolved-layout rendering. Frontmatter unchanged (no reachable screen, no route, no `@ppt/sitemap` op; `endpoints:` stays `listings_get`). `/screens validate` green.
- 2026-07-23 — agent: resolved screen-map drift for PR #2431 (`feat(layout): publish webhook`). PR added reality-web internal API route `src/app/api/layout-revalidate/route.ts` (HMAC-signed ISR revalidation the api-server calls to bust the `layout:<slug>` cache tag on CMS publish). Conclusion: internal endpoint → no standalone screen-map warranted (README rule "a screen-map describes a reachable screen"; mirrors undocumented `api/health`). Documented it here — the layout webhook is the write-side of this screen's resolved-layout rendering — under Notes > Specific. Frontmatter unchanged (no reachable screen, no route, no `@ppt/sitemap` op; `endpoints:` stays `listings_get` only). `/screens validate` green.
- 2026-07-20 — agent: iOS listing detail renders via shared resolved layout dispatch (Swift compile pending macOS verification).
- 2026-07-20 — agent: Android listing detail renders via shared resolved layout dispatch (iOS follow-up pending).
- 2026-07-20 — agent: layout preview mode (postMessage bridge) added.
- 2026-07-19 — agent: page now renders via resolved-layout section registry (defensive rendering, spec 2026-07-19-layout-content-manager-design)
- 2026-06-07 — agent: reconciled screen-map with PR #1085 (reality-web listing-detail SSR metadata hardening). PR extracted a defensive `buildListingMetadata(listing: unknown)` helper + `FALLBACK_METADATA` into a new `metadata.ts` module (with `metadata.test.ts`, 8 cases) so a malformed/partial 200 body no longer throws in `generateMetadata` during SSR; `page.tsx` now delegates to it. Documented the SSR fallback in States > Error and Notes > Specific. Frontmatter unchanged: reality-web buildStatus stays `shipped` (no UI/route change) and apiStatus stays `partial` (no endpoint change — still only `listings_get`); this is a robustness fix, not a feature.
- 2026-05-13 — agent: implemented KMP ListingDetailScreen redesign per ui_kits/mobile-native/screens.jsx KmpListingDetailScreen. New layout: 280dp swipeable hero gallery with white-pill back/share/heart, page dots + counter; sticky agent bar (gradient avatar + Verified pill + Call pill); header section (Featured + transaction-type uppercase badges, large display price + €/m², title, address); 4-card quick stats strip (Area / Year / Energy / Floor); tab strip (Overview / Building / Nearby / Price history) with brand-600 underline; tab body — overview = description + features chips, building = 2-col K/V passport card, nearby = map placeholder, price-history = placeholder; bottom action bar with circle message tile + full-width primary "Žiadať obhliadku". Inquiry dialog + share sheet preserved. Added 11 new strings (sk/en). buildStatus → in-progress, redesignStatus → applied.
- 2026-05-09 — agent: design analyzed (ui_kits/reality-web/listing-detail.html); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (8 sections), states, design-specific notes; linked UC-31/44/46; declared 7 sharedComponents; added 3 relatedScreens (report-listing forward ref deferred — screen-map not yet created)
- 2026-05-08 — init: created from scan (source: sitemap)
