---
id: reality/favorites
name: Favorites
product: reality
sitemapRefs:
  reality-web: reality-favorites
implementations:
  reality-web:
    component: FavoritesPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: FavoritesScreen
    buildStatus: in-progress
    redesignStatus: applied
    apiStatus: partial
endpoints:
  - favorites_list
  - favorites_remove
relatedScreens:
  - id: reality/listings
    rel: sibling
  - id: reality/listing-detail
    rel: child
  - id: reality/saved-searches
    rel: sibling
sharedComponents:
  - listing-card
  - segmented-control
  - chip-group
  - empty-state
  - error-state
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/favorites.html
    frame: favorites-loaded+loading+empty+error
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile-native/screens.jsx
    frame: KmpFavoritesScreen
useCases:
  - UC-44
  - UC-45
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Currently shipped (reality-web baseline)

<!-- Reflects the live `frontend/apps/reality-web/src/app/[locale]/favorites/page.tsx`
     as of 2026-07-13. This is the pre-redesign baseline that ships today; the
     redesign sections below are the target and stay unchecked until
     `redesignStatus: applied`. -->

- [x] [w] Route is auth-gated via `ProtectedRoute` (unauth users bounced to login)
- [x] [w] Portal `Header` + `Footer` chrome; `h1` from `pages.favorites.h1` (i18n)
- [x] [w] Load favorites via `useFavorites(page, 12)` → `favorites_list`
- [x] [w] Responsive `ListingCard` grid (`auto-fill minmax(280px, 1fr)`)
- [x] [w] Each card renders `isFavorite: true` with a filled heart; tap toggles off via `useRemoveFavorite` → `favorites_remove`
- [x] [w] Loading state: 6 pulse skeleton cards
- [x] [w] Error state: "Failed to load favorites. Please try again." (danger colour)
- [x] [w] Empty state: heart icon + "No favorites yet" + "Browse listings" CTA → `/listings`
- [x] [w] Server-side pagination (Previous / Next + "Page X of Y") when `totalPages > 1`
- [x] [w] Price tracking / price-drop alerts on favorited listings (UC-45.8, story 84.3) — inbox surface shipped at `/[locale]/favorites/alerts` (`PriceAlerts` reads `favorites_alerts` feed + mark-read); per-favorite "watch price" toggle (AC-5) wired via `useUpdateFavorite` → `PATCH /api/v1/favorites/{listing_id}` (`price_alert_enabled`)
- [x] [w] Mutation failures surface inline: a `.action-error` banner (`role="alert"`, `aria-live="assertive"`) above the grid when `useRemoveFavorite`/`useUpdateFavorite` error (PR #2890) — reuses the shared, already-translated `error.description` key

### Header
- [ ] [w] Portal header with brand-600 logo + nav (matches profile.html pattern)
- [ ] [w] Header-side favorites button shows current count (e.g. "12")
- [ ] [w] Notification bell with red dot indicator on unread

### Page header
- [ ] [w] Breadcrumbs: Profil / Obľúbené (last current, muted)
- [ ] [w] H1 "Obľúbené" + count chip ("12 obľúbených") with plural-aware count for sk/cs/de/en/pl/hu
- [ ] [w] Header actions: "Zdieľať zoznam" (share-icon, secondary) + "Exportovať PDF" (download-icon, secondary)

### Filters
- [ ] [w] Type segmented control: Všetko / Predaj / Prenájom (single-select, brand-600 active)
- [ ] [w] Sort segmented control: Najnovšie / Cena ↑ / Cena ↓ — labelled with "Zoradiť"

### Loaded grid
- [ ] [w] 3-column listing-card grid at ≥1024px, 2 col at 768–1023, 1 col at <768
- [ ] [w] Each card: 4:3 photo + badges + filled-red heart (top-right, `aria-pressed`, optimistic toggle, fade on unfavorite)
- [ ] [w] Tap heart → unfavorite + count decrements immediately

### Empty
- [ ] [w] Centered empty card (radius 14, 64×32 padding): muted picture-icon tile (88×88 radius 18) + "Zatiaľ žiadne obľúbené" h2 + "Klepnite na srdce pri ktoromkoľvek inzeráte..." body + primary "Prehliadať inzeráty" CTA → reality/home

### Loading
- [ ] [w] 6 skeleton cards (4:3 photo + 3 lines), shimmer animation; respects `prefers-reduced-motion`

### Error
- [ ] [w] Centered error card: danger-50 tile with circle-info icon (64×64) + "Obľúbené sa nepodarilo načítať." h2 + "Skontrolujte pripojenie a skúste to znova." body + primary "Skúsiť znova" CTA

### Footer
- [ ] [w] Portal footer with 5 link columns: brand tagline, Predaj, Spoločnosť, Pomoc, Právne; locale + currency switch in fbot

## States

- **Empty**: muted picture-icon tile + "Zatiaľ žiadne obľúbené" + body copy + primary "Prehliadať inzeráty" CTA. No filters.
- **Loading**: 6 skeleton cards in same grid layout. Shimmer animation; reduced-motion → static fills. Filters remain interactive.
- **Error**: danger-50 tile + circle-info icon + blunt headline + reason + "Skúsiť znova" primary CTA.
- **Success**: 6+ saved listings in grid; each card shows price/address/title/meta + filled-red heart top-right; count chip in header reflects total.

## Notes

### Broader context

UC-44 favorites management — the personal shortlist surface. Public anonymous users get nothing (all favorites require auth); SSO via UC-47 portal accounts. List actions (share / export) are differentiators over standard portals; export uses portal-side PDF generation (likely backed by `reality-server`).

### Specific (recent)

- Count plural-aware across `sk/cs/de/en/pl/hu`: SK uses 1 "obľúbená" / 2-4 "obľúbené" / 5+ "obľúbených" cases — implementation must use `Intl.PluralRules` not string concat.
- Heart toggle is **optimistic**: the card fades to `opacity:.55` on unfavorite for ~200ms, then removes from DOM. If the server rejects, undo + toast banner. Count chip updates synchronously with the optimistic action.
- Empty-state CTA links to `/` (Reality Portal home) per the bundle's hard-coded `href="../Reality Portal Home - Standalone.html"` — production should route to `/` or `/listings`.
- Error tile uses `--danger-50` background in light, `rgba(239,68,68,.12)` in dark with `rgba(239,68,68,.25)` border — explicit dark-mode pairing per project token rules.
- Share-list copies a public token URL (UC-44.X — likely a future UC); export-PDF triggers a per-list PDF render. Both should pass the user's locale to the server for correctly-translated output.
- Sort/type filter state should persist across visits (localStorage or user preference) — not depicted in the design but implied by the segmented controls' "selected" defaults.
- DE strings run ~35% longer ("Favoriten teilen" vs "Zdieľať zoznam"); the actions row must wrap, never truncate.
- **Shipped-vs-redesign split (2026-07-13):** the live reality-web route (`app/[locale]/favorites/page.tsx`, Epic 44 / story 44.5) ships the baseline captured under "Currently shipped" — grid + optimistic remove + loading/error/empty/pagination. It hits only `favorites_list` and `favorites_remove`; the header count chip, share/export, breadcrumbs, and type/sort segmented controls are redesign-only and not wired yet (`redesignStatus: in-progress`, hence `apiStatus: partial`).
- **Mutation error surfacing (PR #2890, 2026-08-30):** remove-favorite (`useRemoveFavorite`) and watch-price (`useUpdateFavorite`) failures were previously fire-and-forget — a rejected request just vanished. They now raise an inline `.action-error` banner (`role="alert"`, `aria-live="assertive"`) above the favorites grid via the mutation `isError` flag, mirroring the `mutation.error &&` pattern already used in the import components (`CsvImport.tsx`, `FeedImport.tsx`). Copy reuses the shared `error.description` key (present in all six locales: en/sk/cs/de/pl/hu) — no catalog change, no new endpoint.
- **Price tracking (story 84.3 / UC-45.8 "Receive Price Drop Alert") — now partially shipped.** PR #2287 added the alert **inbox** at `/[locale]/favorites/alerts` (`PriceAlerts` component consuming the reality-server `FavoriteAlertWorker` feed: list + mark-read + mark-all-read). Follow-up #2326 added the missing AC-5 **subscription** half: a per-favorite "watch price" toggle on the favorites cards (`price_alert_enabled`) backed by the newly-wired `PATCH /api/v1/favorites/{listing_id}` route and `useUpdateFavorite` hook. Note the alerts feed delivers `rust_decimal` prices as JSON **strings** (serde-str) — the client normalizes them to numbers in `useFavoriteAlerts` so drop/increase direction is computed numerically, not lexicographically (the bug #2326 fixed). The redesign-target affordances (count chip, share/export) remain unbuilt, so `apiStatus` stays `partial`.

## Agent Log

<!-- newest entries on top -->

- 2026-08-31 — agent: screen-map drift reconcile for PR #2890 ("surface swallowed mutation errors"). reality-web `app/[locale]/favorites/page.tsx` now renders an inline `.action-error` banner (`role="alert"`, `aria-live="assertive"`) above the grid when `useRemoveFavorite`/`useUpdateFavorite` fail — previously fire-and-forget, silently swallowed. Copy reuses the already-translated `error.description` key (no catalog change, no new endpoints). Added a "Currently shipped" checklist item + a Specific note. Frontmatter unchanged: `buildStatus: shipped`, `apiStatus: partial` (behavioral hardening on the shipped baseline, no API surface change).
- 2026-07-15 — agent: follow-up #2326 (post-merge review of PR #2287). Fixed the Decimal-as-string wire bug: `useFavoriteAlerts` now normalizes `old_price`/`new_price`/`change_percentage` (delivered as JSON strings by `rust_decimal` serde-str) to numbers so `PriceAlerts.priceDirection()` classifies drops correctly. Completed AC-5: wired `PATCH /api/v1/favorites/{listing_id}` (`update_favorite`) on reality-server + `useUpdateFavorite` hook + a per-favorite "watch price" toggle on `app/[locale]/favorites/page.tsx` cards (6-locale `watchPrice` key). Switched the 3 new/touched favorites files to the locale-aware `Link` from `@/i18n/routing`. Per-row mark-read pending state. Marked the 84.3 checklist item shipped ([w]). No frontmatter status flip — `apiStatus` stays `partial` (redesign affordances still unbuilt).
- 2026-07-13 — agent: gap-84-3 reconciled the reality-web screen-map with the live route (`app/[locale]/favorites/page.tsx`). Confirmed `buildStatus: shipped` is correct; added a "Currently shipped (reality-web baseline)" checklist section (grid + optimistic remove + loading/error/empty/pagination), added `favorites_remove` to `endpoints:` (route calls `useRemoveFavorite`), added `UC-45` (price alerts). Documented that price tracking / price-drop alerts (story 84.3 / UC-45.8) are NOT implemented on the shipped screen. No code change — docs/screen-map only.
- 2026-05-13 — agent: implemented KMP FavoritesScreen redesign per ui_kits/mobile-native/screens.jsx KmpFavoritesScreen. New layout: large-title header + share/export actions, M3 TabRow (Properties · Searches), transaction-filter chip strip (All / Sale / Rent), pill segmented sort control (Newest / Price ↑ / Price ↓), 2-col grid of cards with 4:3 photo + white heart pill + price + 1-line title + meta. FAB "Create list" (primaryContainer). Saved-searches tab preserved. Added 10 new strings (sk/en). buildStatus → in-progress, redesignStatus → applied.
- 2026-05-09 — agent: design analyzed (pages/favorites.html, 4 states: loaded + loading + empty + error); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (8 sections), all 4 states, design-specific notes; linked UC-44; declared 5 sharedComponents; added 3 relatedScreens
- 2026-05-08 — init: created from scan (source: sitemap)
