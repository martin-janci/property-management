---
id: reality/agency-dashboard
name: Agency Dashboard
product: reality
sitemapRefs:
  reality-web: reality-agency
implementations:
  reality-web:
    component: AgencyPage
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile-native:
    component: AgencyHubScreen+AgencyInquiriesScreen
    buildStatus: in-progress
    redesignStatus: applied
    apiStatus: stub
endpoints:
  - agencies_get
epics:
  - Epic-32
relatedScreens:
  - id: reality/listings
    rel: sibling
sharedComponents:
  - quickstat
  - tabs
  - activity-feed
  - empty-state
  - error-state
  - status-pill
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/pages/agency-dashboard.html
    frame: agency-dashboard-default+empty+loading+error
useCases:
  - UC-49
  - UC-50
  - UC-51
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header
- [ ] [w] Portal header + notifications bell + user menu
- [ ] [w] Agency identity bar: agency mark (initials avatar), agency name (H1), meta line (Verified pill with check + city · website · "Active since YYYY"), "Upraviť profil" pencil action

### Tab nav
- [ ] [w] Tab strip: Prehľad (active) · Inzeráty (badge count 42) · Makléri (badge 8) · Dopyty (badge 7) · Branding · Import — last 3 tabs route to separate sub-pages (agency-branding, agency-import, see future screen-maps)

### Overview / default
- [ ] [w] **QuickStats** — 4-up grid:
  - Aktívne inzeráty 42 (brand variant) ↑3 za týždeň
  - Nové dopyty (7d) 23 (warning variant) ↑8 vs. min. týždeň
  - Priem. čas odpovede 2.4h ↓18 min rýchlejšie
  - Konverzia 34% (success variant) ↑4 b. dopyt → obhliadka
- [ ] [w] **Recent activity feed** — vertical list with category icon + body text (with bold actor names + linked listings) + relative timestamp ("pred 14 min", "pred 38 min", "pred 2 hod", "včera", "2 d", "3 d"); 8 mock entries; "Zobraziť všetko →" header link
- [ ] [w] Activity types: publish (CheckCircle), inquiry (MessageSquare), import (Download), realtor (UserPlus) — colored per category
- [ ] [w] **Najúspešnejšie inzeráty** — 3-card grid; each card: photo placeholder + title + address + price + tri-stat strip (zobrazení / dopytov / obhliadky); "Všetky výkony →" link

### Empty (new agency onboarding)
- [ ] [w] Welcome hero: "Vitajte! Poďme nastaviť vašu agentúru" + body + progress chip ("0 z 3 dokončených") + estimated time ("~ 15 min spolu")
- [ ] [w] 3-card numbered onboarding grid:
  - 1. Nastavte branding → agency-branding
  - 2. Pozvite maklérov → invite flow
  - 3. Importujte inzeráty → agency-import (CSV / TopReality / Nehnuteľnosti.sk / vlastný XML)
- [ ] [w] Each card has muted "Začať / Pozvať tím / Importovať" right-arrow CTA

### Loading
- [ ] [w] QuickStats skeleton (4×) + activity feed skeleton (4 items, 36px circular avatar skel + 2 line skels + tiny timestamp skel) + top-listings skeleton (3 cards, 80px square + 3 line skels)

### Error
- [ ] [w] Centered error state: circle-info icon + "Nepodarilo sa načítať dashboard" + body ("Skontrolujte pripojenie a skúste to znova. Ak problém pretrváva, naše služby sú pravdepodobne nedostupné — pracujeme na tom.") + "Skúsiť znova" button

## States

- **Empty**: new-agency onboarding. 3-card numbered checklist + progress meter. Replaces overview content entirely; tabs stay visible but disabled until step 1 done.
- **Loading**: quickstats + activity + top-listings skeleton at the same dimensions as default state. Preserves agency identity bar.
- **Error**: full-pane error state replacing overview content. Tabs and identity bar preserved.
- **Success / default**: 4 quickstats + 8-row activity feed + 3-card top listings grid; verified badge in identity bar.

## Notes

### Broader context

UC-49 agency management hub. Overview is the daily-use surface (KPIs + activity); other tabs are management/admin spaces. New agencies land in the empty (onboarding) state until the 3 onboarding actions are complete. Verified-agency badge unlocks elevated discovery on the public portal (boosting `featured` slot rotation per the spec).

### Specific (recent)

- QuickStat variants are tokenized (`brand` / `warn` / default / `success`) — colors come from the 8-state-machine token set, not hard-coded. Trend `.tr.up` uses success-600 ink, `.dn` uses success-600 too here intentionally because faster response time is a positive (semantic flip) — flag at implementation; the data layer must indicate direction-good/bad explicitly per metric.
- Activity feed `relative-time` strings ("pred 14 min" / "pred 38 min" / "pred 2 hod" / "včera" / "2 d" / "3 d") need `Intl.RelativeTimeFormat` for sk/cs/de/en/pl/hu — match other reality pages for consistency.
- Activity-icon colors: publish → success-soft, inquiry → brand-soft, import → blue-soft, realtor → violet-soft. Tokens come from category soft-bg variants.
- Top-listings cards have a left photo block (currently text "photo" placeholder); production renders real listing photos with the same 4:3 listing-card photo treatment. Card variants `b` and `c` may carry different left-edge tints — preserved as visual hierarchy ("most-viewed" gets brand tint?). Verify intent with design owner.
- Onboarding progress bar fills proportionally as steps complete; design shows 0% — animate to target value on data load.
- Onboarding card numbered badges (1/2/3) — use `tabular-nums` so they don't shift in dark mode.
- Tabs `Inzeráty / Makléri / Dopyty` carry `tbadge` count chips — must update when underlying data changes (websocket or polling).
- `Dopyty` tab in this design has badge "7" but no `.muted` class — design intent is that this surface is the destination for inquiry escalation routing from individual realtors. Implementation: badge color shifts based on whether unread inquiries exist (warn vs neutral).
- Identity-bar dots between meta items are `var(--fg-muted)` 4px circles — not Unicode bullets. Implementation should use a styled `<span>` not `·`.
- Verify-pill (Overená agentúra) uses success-soft-bg — must remain legible in dark mode (paired token, never inverted per project rules).

## Agent Log

<!-- newest entries on top -->

- 2026-05-13 — agent: implemented KMP AgencyHubScreen + AgencyInquiriesScreen redesigns per ui_kits/mobile-native/screens-extension.jsx KmpAgencyHubScreen / KmpAgencyInquiriesScreen. Hub: gradient RP identity row, 2×2 stat grid (Active / New inquiries [accent] / Avg response / Conversion), Workspace section card with 4 action rows, recent activity feed (5 events with status pills), Extended FAB "Add listing". Inquiries: large title + search action, dynamic status chip strip with live counts, flat thread rows (44dp gradient avatar + name + listing link in brand color + 2-line preview + uppercase status pill), batch-reply FAB. buildStatus → in-progress, redesignStatus → applied.
- 2026-05-09 — agent: design analyzed (pages/agency-dashboard.html — 4 states: default + empty(onboarding) + loading + error, 6 tabs, 4 quickstats, 8-row activity feed, 3-card top-listings); flipped reality-web redesignStatus → in-progress; attached designSource; populated functionality checklist (6 sections), all 4 states, design-specific notes; linked UC-49/50/51; declared 6 sharedComponents; added 1 relatedScreen
- 2026-05-08 — init: created from scan (source: sitemap)
