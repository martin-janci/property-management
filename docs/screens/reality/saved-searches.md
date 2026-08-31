---
id: reality/saved-searches
name: Saved Searches
product: reality
sitemapRefs:
  reality-web: reality-saved-searches
implementations:
  reality-web:
    component: SavedSearchesPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: SavedSearchesScreen
    buildStatus: in-progress
    redesignStatus: applied
    apiStatus: stub
endpoints:
  - saved_searches_list
relatedScreens:
  - id: reality/listings
    rel: sibling
  - id: reality/favorites
    rel: sibling
sharedComponents:
  - modal-drawer
  - radio-cards
  - chip-group
  - accordion
  - empty-state
  - error-state
  - bell-toggle
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/saved-searches.html
    frame: saved-searches-loaded+loading+empty+error+create-modal
useCases:
  - UC-45
  - UC-31
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header + intro
- [ ] [w] Portal header + breadcrumbs (Profil / Uložené hľadania)
- [ ] [w] H1 "Uložené hľadania" + lede "Ukladajte hľadania, dostávajte upozornenia, keď sa objavia nové zhody."
- [ ] [w] Right-aligned primary button "Nové uložené hľadanie" (plus icon) → opens create modal

### Loaded list (UC-45)
- [ ] [w] Vertical list (gap 12px) of `.ss-row` cards (radius 12, border + shadow-card)
- [ ] [w] Each row main: H3 search name + alerts pill (success-on / muted-off with bell-dot indicator); query summary line `<b>Bratislava · Ružinov</b> • 2–3 izby • Predaj · Byt • <b>do €350 000</b>`; meta line with results count + "N nové" tag (warning) + last-match relative time
- [ ] [w] Per-row actions: bell toggle icon-btn (filled when on, with diagonal slash when off, `aria-pressed`); "Upraviť" pencil-secondary; "Spustiť" play-primary → navigates to `reality/listings` with applied filters; trash icon-btn-danger
- [ ] [w] Inline delete confirmation: row shifts to "confirming" mode showing "Zmazať toto hľadanie? · Zmazať / Zrušiť"; on confirm → fade + scale-down out (220ms)
- [ ] [w] Toggle bell: optimistic alerts on/off; freq persists from prior choice or defaults to `daily` when re-enabling

### Empty
- [ ] [w] Center card: muted clock-search tile (88px) + "Žiadne uložené hľadania" h2 + "Uložte si akékoľvek hľadanie zo stránky inzerátov a dostávajte upozornenia." body + primary "Prejsť na inzeráty" CTA → reality/listings

### Loading
- [ ] [w] 4 skeleton rows (`.skel-row` with bar elements: 30%/70%/50% widths + 4 action skeletons including taller "run" button skeleton); shimmer animation; respects `prefers-reduced-motion`

### Error
- [ ] [w] Center card: danger-tile + "Uložené hľadania sa nepodarilo načítať." + "Skontrolujte pripojenie a skúste to znova." + primary "Skúsiť znova"

### Create / edit modal
- [ ] [w] Scrim + centered modal (max-width ~640, radius 16, modal shadow)
- [ ] [w] Header: "Nové uložené hľadanie" / "Upraviť hľadanie" + close icon-btn
- [ ] [w] Required Name input with placeholder example
- [ ] [w] **Filters** accordion: 5 collapsible groups, each with summary text on closed state and chevron-rotate animation (open):
  - Lokalita (city + district selects + chip multi-select)
  - Typ ponuky (segmented Predaj / Prenájom / Novostavby + property-type chips)
  - Cena (range inputs)
  - Izby a plocha (room chip-group + area range)
  - Ďalšie (amenity chips: Balkón / Parking / Výťah / Záhrada / Pivnica / Novostavba)
- [ ] [w] **Frekvencia upozornení** — 3 radio cards (3-up grid):
  - Vypnuté (bell-off icon, "Žiadne notifikácie. Hľadanie zostane uložené.")
  - Denne (calendar icon, "Súhrn každý deň o 9:00. Tichý cez víkendy.") ← default
  - Okamžite (bolt icon, "Push aj e-mail do 5 minút od pridania.")
- [ ] [w] Footer left: helper "Upozornenia môžete kedykoľvek vypnúť." Footer right: Zrušiť (secondary) + Uložiť hľadanie (primary)

## States

- **Empty**: clock-search tile + "Žiadne uložené hľadania" + body copy + "Prejsť na inzeráty" primary CTA. No table.
- **Loading**: 4 skeleton rows with bar+action skeletons; shimmer. Header + create button remain interactive.
- **Error**: danger-50 tile + circle-info icon + blunt headline + retry primary CTA.
- **Success**: list of ss-rows with current alert state, results counts, "N nové" tags where applicable, last-match relative timestamps.

## Notes

### Broader context

UC-45 saved searches & alerts. Each saved search is a serialized filter state from `reality/listings` plus an alert frequency. Alert delivery is server-side (push + e-mail at the chosen cadence). "Spustiť" reapplies the filters to the listings page; "Upraviť" reopens the same modal pre-populated. Lives behind UC-47 portal account auth.

### Specific (recent)

- `lastMatch` strings ("pred 2 h", "pred 11 h", "včera") are produced by a humanizer — must use locale-aware relative-time (`Intl.RelativeTimeFormat` for sk/cs/de/en/pl/hu).
- Bell toggle has 3 visual states: filled (alerts on, brand-600 stroke + fill), outline-with-slash (alerts off, muted), and confirming-delete (red ring during confirmation overlay). Tooltip via `data-tip` attribute.
- Inline confirm-delete is a row-level overlay: `row.classList.add('confirming')` toggles a hidden `.ss-confirm` block visible. Cancel restores; confirm fades+scales the row out before DOM removal.
- "N nové" tag uses warning-bg (amber soft) + warning-700 ink — not the same as the alerts-on green pill. Differentiates "matches since last view" from "alerts active globally".
- Alert frequency cards use `radio-card` pattern (`.on` adds brand outline + ring); only one can be selected. Cards remain interactive even when the search itself has alerts off — the freq sticks if user re-enables alerts.
- Search name max length should be enforced (~80 chars) to prevent overflow in row H3; design uses ellipsis-via-line-clamp pattern via webkit-line-clamp:1.
- Modal must trap focus + close on Escape; scrim click-to-close optional but standard.
- DE strings ~35% longer ("Gespeicherte Suchen" vs "Uložené hľadania") — hero h1 must wrap; modal accordion summary text uses ellipsis only on closed state.
- Frequency `instant` ships push + email within 5 min of new match; daily uses 09:00 local with weekend silence — backend must respect both. Surface this to UC-23 quiet-hours user prefs.
- **Mutation error surfacing (PR #2890, 2026-08-30):** delete-search (`useDeleteSavedSearch`) and alert-toggle (`useToggleSearchAlert`) failures were previously swallowed silently; the shipped `SavedSearchCard` now renders an inline `.card-error` message (`role="alert"`, `aria-live="assertive"`) inside the affected card via the mutation `isError` flag. Reuses the shared `error.description` key (all six locales) — no new locale strings, no endpoint change.

## Agent Log

<!-- newest entries on top -->

- 2026-08-31 — agent: screen-map drift reconcile for PR #2890 ("surface swallowed mutation errors"). reality-web `app/[locale]/saved-searches/page.tsx` `SavedSearchCard` now surfaces `useDeleteSavedSearch`/`useToggleSearchAlert` failures inline (`.card-error`, `role="alert"`, `aria-live="assertive"`, rendered inside the affected card) instead of silently swallowing them. Reuses the shared `error.description` i18n key — no catalog change, no new endpoints. Added a Specific note. Frontmatter unchanged: `buildStatus: shipped`, `apiStatus: partial` (error-handling hardening, no API surface change).
- 2026-05-13 — agent: implemented KMP SavedSearchesScreen redesign per ui_kits/mobile-native/screens-extension.jsx KmpSavedSearchesScreen. Large-title header + back + primaryContainer "New search" pill, row cards with bookmark leading icon (filled when alerts on), clock + age + match-count meta, kebab menu (Enable/Disable alerts · Delete), "ALERTS ON/OFF" uppercase pill + "Run" primaryContainer pill action. Empty state with 72dp tinted circle. Added 6 strings (sk/en). buildStatus → in-progress, redesignStatus → applied.
- 2026-05-09 — agent: design analyzed (pages/saved-searches.html — 4 list states + create-edit modal); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (6 sections), all 4 states, design-specific notes; linked UC-45/31; declared 7 sharedComponents; added 2 relatedScreens
- 2026-05-08 — init: created from scan (source: sitemap)
