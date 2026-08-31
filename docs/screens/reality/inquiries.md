---
id: reality/inquiries
name: Inquiries
product: reality
sitemapRefs:
  reality-web: reality-inquiries
implementations:
  reality-web:
    component: InquiriesPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: InquiriesScreen
    buildStatus: in-progress
    redesignStatus: applied
    apiStatus: partial
endpoints:
  - inquiries_list
relatedScreens:
  - id: reality/listing-detail
    rel: parent
sharedComponents:
  - status-pill
  - search-bar
  - segmented-control
  - empty-state
  - error-state
  - message-bubble
  - day-strip
  - slot-grid
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/inquiries.html
    frame: inquiries-2col-list+thread (4 list states × 4 thread states)
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile-native/screens.jsx
    frame: KmpInquiriesScreen + KmpInquiryThreadScreen
useCases:
  - UC-46
epics: []
diagrams: []
owner: reality-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Layout
- [ ] [w] 2-column workspace: left list (≈360px) + right thread (flex 1); single-column collapse on mobile (list-only with back-nav into thread)
- [ ] [w] Page header: portal header + breadcrumbs (Profil / Dopyty) + H1 "Dopyty"

### List column (UC-46)
- [ ] [w] Header: "Konverzácie" + total summary "3 nečítané · 8 spolu"
- [ ] [w] Search input: "Hľadať podľa adresy alebo mena…"
- [ ] [w] Filter tab strip: Všetky / Čaká / Odpovedané / Naplánované / Uzavreté — each with count badge (`.ct`)
- [ ] [w] List item rows: avatar (initials, gradient bg per-thread), 4:3 mini-thumb of listing photo, name + relative time, listing address line, preview snippet (last message), bottom row with status pill (Čaká amber / Odpovedané blue / Naplánované violet / Uzavreté gray); unread indicator (left blue bar + bold name)
- [ ] [w] Active row has selected highlight; click switches the thread column
- [ ] [w] Empty (list): muted message tile + "Zatiaľ žiadne dopyty" + "Dopyty, ktoré odošlete, sa zobrazia tu."
- [ ] [w] Loading (list): N skeleton rows (avatar + 3 lines)
- [ ] [w] Error (list): danger tile + "Dopyty sa nepodarilo načítať" + retry CTA

### Thread column
- [ ] [w] Header: back-arrow icon-btn (mobile single-col), listing thumb + price + address inline meta, agent card (avatar + name + license badge with check icon), tools (Hovor + More dropdown)
- [ ] [w] Day-divider chips ("Včera · 14:20", "Dnes · 09:14") between message clusters
- [ ] [w] Message bubbles: `mine` aligned right with brand-soft bg + dark ink + read-receipt double-check; `them` aligned left with surface bg + secondary ink; avatar shown beside each (initials)
- [ ] [w] Composer row: paperclip attach button + text input + "Žiadať obhliadku" CTA + send (paper plane) primary
- [ ] [w] **Quick replies** above composer: 3 chip suggestions (e.g. "Aký je stav fondu opráv?", "Cena dohodou?", "Posielate energetický certifikát?") — tap inserts into input
- [ ] [w] **Inline schedule panel** (toggled by "Žiadať obhliadku"): h4 "Žiadať obhliadku" + sub copy + day-strip (7-day horizontal scroll, 1 active) + slot grid (8×2 30-min slots with 3 states: available / taken (gray) / on (selected)) + footer with picked-summary + Zrušiť/Žiadať slot actions
- [ ] [w] Empty (thread): "Vyberte konverzáciu" + "Detail dopytu sa zobrazí tu."
- [ ] [w] Loading (thread): bubble-shaped skeletons alternating left/right
- [ ] [w] Error (thread): danger tile + "Konverzáciu sa nepodarilo načítať" + retry

## States

- **Empty (list)**: muted message tile + "Zatiaľ žiadne dopyty" + "Dopyty, ktoré odošlete, sa zobrazia tu." Thread shows "Vyberte konverzáciu".
- **Loading (list)**: skeleton rows; thread can independently show its own loading bubbles.
- **Error (list)**: danger tile + retry CTA; thread independently shows empty.
- **Success**: 8 mock conversations across 4 statuses (3 unread); active thread shows 5 messages spanning 2 days, schedule panel open with Štvrtok 14. máj · 17:30 selected.

## Notes

### Broader context

UC-46 contact inquiries — primary conversion surface from listing detail. Threaded chat between portal user and agent (UC-49 agency-side). Schedule panel produces a calendar booking that flows to the agent's diary; "Maklér potvrdí do 2 hodín" SLA is published in the lede.

### Specific (recent)

- Two independent state machines: list-state (loaded/loading/empty/error) and thread-state (loaded/loading/empty/error) — design exposes both via separate preview switchers, implementation must track separately.
- Avatar gradients are per-thread (8 mock combos in JS data) — implementation should hash agent ID to a stable palette so colors don't shift between renders.
- Status pills carry a `.dot` indicator + label; colors per status: Čaká → amber, Odpovedané → blue (information state), Naplánované → violet (event state), Uzavreté → gray. Tokens come from the 8-state-machine system in colors_and_type.css.
- Read-receipt SVG is a double-check polyline drawn manually (`points="2 12 8 18 22 4"` is the second tick) — when ported, use Lucide's `check-check`.
- Day-divider format relative-aware: "Včera · HH:MM", "Dnes · HH:MM", or "DD. MMM · HH:MM". Use locale `Intl.DateTimeFormat` with `dayPeriod: 'narrow'` for sk grammar.
- Slot grid 30-min granularity, 16 slots (8×2). `taken` slots have muted bg + strikethrough text; `on` (selected) gets brand-600 fill + white ink. Disabled state for past times not shown but should match `taken` styling without aria-pressed.
- Quick-reply chips are agent-curated (presumably stored per-agent or per-listing-type) — surface as a tunable list, not hardcoded.
- License badge pattern (`Lic. RM-08421`) is a Slovak agent licence number — render verbatim, do not translate the prefix; the surrounding role string (Realitný maklér) does translate.
- Single-column mobile layout: list visible on /inquiries; tap → push thread route with back-nav header. Don't try to fit both in one viewport on phones.
- Composer "Žiadať obhliadku" button collapses the schedule panel inline above the composer — does not open a modal. This keeps the convo + scheduling unified.
- DE strings: "Anfragen" vs "Dopyty" works; "Besichtigung anfragen" vs "Žiadať obhliadku" runs longer — composer button must wrap or icon-only on narrow widths.
- **i18n extraction of the shipped page (PR #2895, 2026-08-30):** the live reality-web `app/[locale]/inquiries/page.tsx` previously rendered hardcoded English chrome (page title/subtitle, the "All" filter, the five status labels, "Viewing scheduled: …", "Agent response:", the cancel confirm/yes/no/button labels, and Previous/pageInfo/Next pagination) in a 4-language app. These now resolve through `useTranslations('pages.inquiries')`, and the status labels moved out of the module-level `statusConfig` (now colors-only) to per-render ``t(`status.${status}`)``. Keys were added under `pages.inquiries.*` in all **six** message catalogs (en/sk/cs/de/pl/hu — reality-web's `src/i18n/config.ts` declares six locales, not the four in CLAUDE.md; that note is stale). A locale-parity regression guard (`src/i18n/inquiries-messages.test.ts`) asserts every locale defines the chrome + status keys and preserves the `{page}/{total}` and `{date}/{time}` placeholders. Pure i18n — no behavioral change, no endpoint change.

## Agent Log

<!-- newest entries on top -->

- 2026-08-31 — agent: screen-map drift reconcile for PR #2895 ("i18n the reality-web Inquiries page"). The shipped reality-web `app/[locale]/inquiries/page.tsx` had hardcoded English chrome + status labels; they now resolve via `useTranslations('pages.inquiries')` with keys added to all six locale catalogs (en/sk/cs/de/pl/hu), plus a `inquiries-messages.test.ts` locale-parity guard. Status labels moved out of module-level `statusConfig` (colors-only) to per-render `t(\`status.${status}\`)`. Added a Specific note. Frontmatter unchanged: reality-web `buildStatus: shipped`, `apiStatus: partial` (i18n extraction on the shipped baseline — no build/api surface change, no route change). Note: the live reality-web page is the flat single-column shipped baseline; the 2-col list+thread redesign in the checklist remains `redesignStatus: in-progress` and unbuilt.
- 2026-05-13 — agent: implemented KMP InquiriesScreen redesign per ui_kits/mobile-native/screens.jsx KmpInquiriesScreen. New layout: large-title "Dopyty" header, status filter chip strip (All · count / Pending / Responded / Closed), restyled M3 TabRow (Messages · Viewings), flat (no card chrome) message rows with 3dp left unread bar + 48dp gradient avatar + initials + 22dp listing thumb overlay + relative time + 2-line preview + uppercase status pill. Viewings list restyled as 16dp rounded cards with status pill + cancel dialog. Inline thread view (KmpInquiryThreadScreen — calendar grid + slot picker + bubble composer) deferred until inquiry-reply API + scheduling backend land. buildStatus → in-progress, redesignStatus → applied.
- 2026-05-09 — agent: design analyzed (pages/inquiries.html — 2-col list+thread, 4 list states × 4 thread states, schedule panel + quick replies + day-strip + slot-grid); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (3 sections), all 4 states (list-side, thread-side documented in checklist), design-specific notes; linked UC-46; declared 8 sharedComponents; added 1 relatedScreen
- 2026-05-08 — init: created from scan (source: sitemap)
