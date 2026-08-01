---
id: reality/home
name: Home
product: reality
sitemapRefs:
  reality-web: reality-home
implementations:
  reality-web:
    component: HomePage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: HomeScreen
    buildStatus: in-progress
    redesignStatus: applied
    apiStatus: partial
sharedComponents:
  - listing-card
  - hero-search
  - quickstat
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/reality-web/home.html
    frame: reality-portal-home
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile-native/screens.jsx
    frame: KmpHomeScreen
useCases:
  - UC-31
  - UC-44
endpoints: []
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header
- [ ] [w] Sticky header (64px) with brand-600 logo, nav (Buy / Rent / Sell / Journal / Help), Sign-in + List-your-property CTAs

### Hero
- [ ] [w] Gradient hero `linear-gradient(135deg, #1e40af, #3b82f6)` with h1 + subhead copy
- [ ] [w] Search-mode tabs floating above panel: For sale / For rent / New builds
- [ ] [w] Search panel (radius 14, white surface) with 4 inline fields (Where, Type, Rooms, Price) + primary Search button
- [ ] [w] Hero meta strip: active-listings count, verified-agents count, building-passport %

### Browse-by-type
- [ ] [w] 6-up category chip grid (Apartments, Houses, Commercial, Land, Recreation, Parking) with per-category listing counts
- [ ] [w] Hover lifts border to brand-600

### Featured listings
- [ ] [w] 4-column listing-card grid (`reused from listings.html`); each card: 4:3 photo, FEATURED/SALE/RENT badges (top-left), heart fav (top-right, optimistic toggle), price + €/m² suffix, address, 2-line truncated title, meta (rooms · m² · floor)
- [ ] [w] Section header with "See all 248 featured" link

### Popular neighbourhoods
- [ ] [w] 3-card grid with neighbourhood photo + label, residents count, avg €/m², active listings count, QoQ trend arrow (success-600 ↑ / danger-600 ↓)

### Value strip
- [ ] [w] 4-cell stat band: Verified agents · Building passport · Price history · Mortgage pre-approval — each with metric + uppercase label + supporting one-liner

### CTA band
- [ ] [w] "Selling your home?" panel on accent-soft-bg with Learn-more (secondary) + Start-listing (primary) actions

### Footer
- [ ] [w] Locale + currency switch (EN / SK / CZ · EUR), legal links

## States

- **Empty**: not depicted in design (home is content-rich; empty featured-list slot would degrade to 0-card grid with "No featured listings this week" message — propose at implementation time)
- **Loading**: not depicted; design implies hero is static SSR while featured/neighbourhood grids progressively enhance — use 4-card and 3-card skeletons matching the aspect ratios
- **Error**: not depicted; per SKILL.md error voice → blunt single-line `Unable to load featured listings` + `Try again` (retain hero + categories)
- **Success**: full content as designed (4 featured cards, 3 neighbourhood cards, populated stats)

## Notes

### Broader context

Reality Portal homepage with hero search, featured listings, neighbourhood explorer, value-prop strip, and selling-side CTA. Anchors the public funnel into UC-31 listings + UC-44 favorites + UC-49 agency entry points.

### Specific (recent)

- Hero gradient is the **only** gradient in the system per project README — `linear-gradient(135deg, #1e40af 0%, #3b82f6 100%)`. Do not introduce additional gradients elsewhere.
- Search-mode tabs sit on a glass-on-blue `rgba(255,255,255,.15)` strip with `backdrop-filter: blur(8px)` — one of the only two translucent surfaces in the system. Active tab swaps to white surface with brand-600 ink.
- Search panel shadow `0 20px 50px -20px rgba(0,0,0,.4), 0 8px 20px -8px rgba(0,0,0,.2)` — distinct from the standard 2-elevation card shadows; it's a hero-only treatment.
- Photo placeholders use `linear-gradient(135deg, var(--ph-a), var(--ph-b))` with two radial highlights — production should swap to real 4:3 property photos and fall back to the `#f3f4f6` muted picture-icon tile when missing.
- Trend arrows use Unicode glyphs `↑ ↓ →` (retained per iconography rules); colored success-600 / danger-600 inline.
- Featured/Sale/Rent badges are uppercase 10px / 700, letter-spacing 0.04em — only badges and chips are uppercase across the system.
- Hero meta numbers (`14,280` listings, `1,120` agents, `98%` passport) need locale-aware `Intl.NumberFormat` — diacritics + DE wrap considerations apply to surrounding labels.
- Dark mode pair via `[data-theme="dark"]` is implied (token-driven); v0 — needs contrast QA per project caveat.
- Cards use radii 12, hover lift `translateY(-2px)` + `var(--shadow-card-hover)`, motion 200ms `ease-standard`. Respect `prefers-reduced-motion`.
- Shared root layout inline scripts (`frontend/apps/reality-web/src/app/[locale]/layout.tsx`, wraps this and every reality-web screen): two payloads are inlined into `<script>` — the static `ORGANIZATION_LD` JSON-LD (schema.org `Organization`, SEO) and the per-request `window.__TENANT_CONFIG__` bootstrap (`tenant_id` + `feature_flags`, host-derived). Both MUST be serialized with `serializeForScript` from `src/lib/serialize-script.ts`, **not** bare `JSON.stringify` (PR #2600). The tenant bootstrap is the security-critical one: `tenant_id` / feature-flag keys are per-request attacker-influenceable, so an unescaped `</script>` (or U+2028/U+2029) would be an HTML-injection window on every request; the helper escapes `<`/`>`/`&`/U+2028/U+2029 to `\uXXXX`. The `/env.js` runtime-env route (`src/app/env.js/route.ts`) was hardened in the same PR (previously escaped only `<`). Any new inline-`<script>` payload added to the layout must route through the same primitive.

## Agent Log

<!-- newest entries on top -->

- 2026-08-01 — agent: reconciled screen-map with PR #2600 (reality-web inline-script XSS hardening). The shared `[locale]/layout.tsx` — which wraps this home screen and every reality-web route — now serializes its two inline `<script>` payloads (static `ORGANIZATION_LD` JSON-LD + per-request `window.__TENANT_CONFIG__` tenant bootstrap) via the new `serializeForScript` helper (`src/lib/serialize-script.ts`) instead of bare `JSON.stringify`, closing an HTML-injection window driven by host-derived `tenant_id` / feature-flag keys; `/env.js` was hardened the same way (was escaping only `<`). Documented under Notes > Specific here (home is the canonical screen for the shared layout). Frontmatter unchanged — no UI/route/endpoint change, pure security hardening. Listing-detail JSON-LD half of the same PR is logged on `reality/listing-detail`. `/screens validate` green.

- 2026-05-12 — agent: implemented KMP HomeScreen redesign per ui_kits/mobile-native/screens.jsx KmpHomeScreen. New layout: custom top bar (logo + bell with red dot + avatar circle), hero card with Brand800→Brand500 gradient + search trigger + transaction-type pills, featured carousel (260dp cards w/ FEATURED badge), 3×2 category grid (Apartments / Houses / Commercial / Land / Recreation / Parking), recent listings vertical list w/ 64dp thumbnails. Added Brand800 token + 14 new strings (sk/en). buildStatus → in-progress, redesignStatus → applied.
- 2026-05-09 — agent: design analyzed (ui_kits/reality-web/home.html); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (8 sections), states, and design-specific notes; linked UC-31 + UC-44; declared sharedComponents listing-card / hero-search / quickstat
- 2026-05-08 — init: created from scan (source: sitemap)
