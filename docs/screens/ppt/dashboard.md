---
id: ppt/dashboard
name: Dashboard
product: ppt
sitemapRefs:
  mobile: mobile-dashboard
implementations:
  ppt-web:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
  mobile:
    component: DashboardScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
endpoints:
  - dashboard_get
relatedScreens:
  - id: ppt/faults-list
    rel: child
  - id: ppt/announcements
    rel: child
  - id: ppt/report-fault
    rel: child
sharedComponents:
  - mobile-tab-bar
  - status-pill
  - announcement-pin-card
  - mobile-row-list
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: MobHomeScreen
epics: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Top bar
- [ ] [m] Greeting block: "Good morning," (13/muted) + first name (22/700)
- [ ] [m] Right: 36px circular notifications icon-btn (surface bg + border) with red 8px dot indicator on unread (offset top:-2/right:-2)

### Address strip
- [ ] [m] Tiny meta with map-pin icon: "Miletičova 45 · Unit 4B · Ružinov" (12/muted)

### Pinned announcement (top-of-fold)
- [ ] [m] Warning-50 bg + amber-300 border card (radius 12, 12×14 padding) with 📌 emoji (legacy — Lucide pin needed) + uppercase "Pinned · Water outage" eyebrow + bold 14px title + muted "Tap to read · Acknowledge" sub-line
- [ ] [m] Tap → opens announcement detail screen with single-tap acknowledge affordance

### "My home" section
- [ ] [m] Uppercase 11/700/.06em section label
- [ ] [m] Stacked card (radius 14, surface, soft shadow) with row list:
  - **Report a fault** — danger-bg icon + title + "Takes about 30 seconds" sub
  - **Vote: Parking rules** — violet (#8b5cf6) icon + title + "Closes in 3 days" sub + violet "Open" pill right
  - **Building documents** — accent icon + "AGM minutes · Annual report" sub
- [ ] [m] Each row: 38×38 colored square (10 radius) icon + title (14/600) + sub (12/muted) + optional right pill

### "Recent activity" section
- [ ] [m] Uppercase section label
- [ ] [m] Card with row list:
  - **Fault #F-276 resolved** — success icon + "Hallway light · 2 days ago"
  - **Lift maintenance scheduled** — accent icon + "Jul 22 · 10:00–12:00"

### Bottom tab bar
- [ ] [m] 5 tabs: Home (active) / Faults / News / Votes / Docs — each with Lucide-style stroke icon (replacing legacy emoji 🏠 🔧 📢 🗳️ 📄) + 10px label
- [ ] [m] Active tab: brand-600 ink + 600 weight + slightly thicker stroke (2.2 vs 1.8); inactive tab: muted ink + 500 weight
- [ ] [m] Bottom safe-area padding (24px) for home-indicator clearance

## States

- **Empty**: not depicted; first-launch state should hide pinned announcement card and show single onboarding prompt "Tap report-fault to get started" instead of activity card.
- **Loading**: not depicted; recommend skeleton greeting + skeleton address + 3-row card skeletons. Tab bar always visible.
- **Error**: not depicted; per voice → blunt single-line "Couldn't reach your building" + retry CTA inline above the My-home card.
- **Success**: full populated dashboard with pinned announcement, 3-row My-home card, 2-row recent-activity card.

## Notes

### Broader context

UC-15 + UC-02 + UC-03 + UC-04 hub for residents on mobile (RN). The home screen consolidates the most-used affordances (Report fault, current votes, documents) and a digest of recent activity. Pinned announcement has prime placement above the fold to drive read-rate.

### Specific (recent)

- Load errors surface via a retryable inline banner above the stats grid (`hasError` across all four dashboard queries; #2282/#2304). The banner is now the shared `components/QueryErrorBanner.tsx` (extracted #2323) so sibling screens can reuse the same contract — it renders a caller-provided, already-localized message and never a raw `error.message`. The whole mobile screens tree was swept in #2323 to drop raw `error.message`/`combinedError.message` renders (backend-internals leak).
- Dashboard dates now localize to `i18n.language` (was hardcoded `en-US`); the announcement category badge is translated via `dashboard.category.*` (was rendering the raw enum).
- Mobile tokens defined as a JS object (`MOB_TOKENS`) inline in screens.jsx — production must replace with imports from a shared RN theme file consumed from `@ppt/ui-kit` or equivalent. Don't ship inline-token objects.
- 📌 emoji in pinned-announcement card violates SKILL.md non-negotiable — replace with Lucide `pin` SVG. Same for any other emoji glyph elsewhere.
- Greeting "Good morning," is time-of-day dependent; use locale-aware ranges (sk: "Dobré ráno" until 10:00, "Dobrý deň" 10–18, "Dobrý večer" 18–22, "Dobrú noc" 22+).
- Notifications dot uses 2px border colored to match the parent bg — not surface; this avoids a thin line gap. When parent bg changes (dark mode), border color must follow.
- Pinned announcement uses non-token colors (`#fffbeb`/`#fde68a`/`#92400e`/`#78350f`) — should derive from `--warning-soft-bg`/`--warning-soft-border`/`--warning-700`/`--warning-800` tokens.
- Row icons currently inline SVG `path` strings — RN must use a centralized icon component (Lucide RN or equivalent).
- Right pill colors: violet pair for "Open" vote — match the 8-state-machine vote tokens.
- Tab-bar uses 24px round-cap stroke icons; icon paths are condensed (e.g. faults wrench: `M14.7 6.3...`). Keep stroke 2 for inactive, 2.2 for active per the design.
- This screen-map's `mobile-dashboard` sitemap ID maps to RN `DashboardScreen` — naming consistency is "Dashboard" on RN even though copy says "Home" (greeting).
- Recent-activity card has 100px bottom margin to clear the tab bar — production must use `SafeAreaView` insets, not hard-coded margins.

## Agent Log

<!-- newest entries on top -->

- 2026-07-19 — agent: page now renders via resolved-layout section registry (defensive rendering, spec 2026-07-19-layout-content-manager-design)
- 2026-07-15 — agent: extracted reusable `QueryErrorBanner` from DashboardScreen and swept sibling mobile screens to drop raw `error.message` leaks (#2282/#2304 follow-up, #2323); localized dashboard dates (`i18n.language`) + announcement category badge (`dashboard.category.*`); fixed cs/de `dashboard.loadError` retry-label copy; added QueryErrorBanner regression suite
- 2026-05-09 — agent: design analyzed (ui_kits/mobile/screens.jsx — MobHomeScreen); flipped mobile redesignStatus → in-progress; attached designSource; populated functionality checklist (6 sections), states, design-specific notes; declared 4 sharedComponents; added 3 relatedScreens (faults-list / announcements / report-fault as children of dashboard)
- 2026-05-08 — init: created from scan (source: sitemap)
