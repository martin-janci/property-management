---
id: ppt/home
name: Home
product: ppt
sitemapRefs:
  ppt-web: ppt-home
implementations:
  ppt-web:
    component: Home
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
relatedScreens:
  - id: ppt/faults-list
    rel: child
  - id: ppt/announcements
    rel: child
  - id: ppt/voting
    rel: child
sharedComponents:
  - quickstat
  - action-queue
  - status-pill
  - command-palette
  - sparkline
  - timeline
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/ppt-web/manager-dashboard.html
    frame: manager-dashboard
useCases:
  - UC-15
  - UC-03
  - UC-02
  - UC-04
  - UC-17
endpoints: []
epics:
  - Epic-3
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header
- [ ] [w] Sticky 60px top bar: PPT logo (`em PPT · span Manager`) + tab nav (Dashboard / Faults · 12 / Announcements / Votes / Residents / Documents) — Faults gets a danger-500 count chip
- [ ] [w] Right side: Command palette `cmd` input ("Search or jump to…" + `⌘K` kbd) + notifications icon-btn with red dot + help icon-btn + 34px gradient avatar

### Greeting strip
- [ ] [w] Eyebrow with pulsing dot: "3 items need attention · Tue, Jul 16"
- [ ] [w] H1 "Good morning, Jana" (26px / 700, -0.02em)
- [ ] [w] Subhead: contextual single-paragraph hint pointing to the most-urgent item
- [ ] [w] Header actions: Export (secondary, download icon) + "New announcement" (primary, plus icon)

### QuickStats (4-up)
- [ ] [w] **Awaiting triage** — danger variant (red gradient bg, ink #991b1b): big number + "↑ N new since yesterday" trend pill
- [ ] [w] **Open faults** — wrench icon, 12 number, sparkline (60×26 SVG polyline, success-500 stroke), "↓ N from last week" green trend
- [ ] [w] **Pending votes** — checkmark icon, 2 number, "→ unchanged" flat trend
- [ ] [w] **Occupancy** — building icon, "94%" with `<small>%</small>` muted suffix, "↑ 2pt this month" trend

### Action queue (left, 2-col main)
- [ ] [w] Panel header: "Action queue" + danger count chip + filters segmented (All · 4 / Faults · 2 / Votes · 1 / Anno. · 1) + "View all →" link
- [ ] [w] Selected row: brand-soft-bg + 3px brand inset-border on left
- [ ] [w] Row anatomy: 40×40 colored icon block + title + severity pill (Urgent danger / High orange / Normal blue / Draft gray) + status pill (New, Assigned) + meta with category + reporter/assignee + SLA breach (danger-600 bold) + "N ago" right-aligned tabular-nums + inline action buttons (Triage now / Assign / Message resident — selected row only shows these)
- [ ] [w] Footer: keyboard hint "Use J/K to navigate, Enter to open, A to assign" + total count

### Right column (340px, 4 stacked tiles)
- [ ] [w] **Occupancy** — 84px conic-gradient ring (brand-600 fill at `--p%`, bg-subtle remainder) with inner cutout + center percentage; right-side info (Current / 17 of 18 units / vacancy hint); 12-month bar chart strip below with last-3 hi-bars in brand-600 + month-range labels
- [ ] [w] **Active votes · 2** — vote tiles, each with title + meta (close date · quorum status) + progress bar (brand-600 default, success-500 when quorum reached) + label "<b>N</b> voted · context" + percentage right-aligned
- [ ] [w] **Recent activity** — timeline list: 28×28 colored bubble icon (success/info/warn/violet per event type) + body sentence (with bold actors/IDs) + relative time right-aligned
- [ ] [w] **This month** — 2×2 tile grid: Fees collected (€4,280) · Paid on time (18/18) · Faults resolved (8) · Avg. triage time (1.4d)

## States

- **Empty**: not depicted; for a fresh manager with no buildings: empty-state hero + onboarding CTA "Add your first building" replacing main + right column. Header stays.
- **Loading**: not depicted explicitly; recommend skeleton 4-up quickstats + 4-row action-queue skeletons + right-rail skeleton matching mini cards. Pulsing eyebrow dot becomes static.
- **Error**: not depicted; per voice → `Unable to load dashboard` banner inside main grid with `Try again`. Header + greeting retained.
- **Success**: full populated dashboard as designed (4 quickstats with sparkline + danger highlight, 4-row action queue with selected-row inline actions, 4-tile right rail).

## Notes

### Broader context

PPT manager landing — the daily-use surface for property managers. Action-queue is the working list (UC-03 faults + UC-04 votes + UC-02 announcement reviews); right rail surfaces standing KPIs (occupancy, money, monthly metrics). Eyebrow + subhead read the *most urgent* item — copy must be data-driven, not static.

### Specific (recent)

- Greeting is third-person summary ("3 faults awaiting triage") not "you" — per project README, managers get summaries, residents get "you/your".
- Eyebrow pulse dot uses `animation: pulse 2s infinite` — must be disabled under `prefers-reduced-motion`.
- Danger quickstat uses gradient bg `linear-gradient(180deg, #fef2f2, #fee2e2)` and dark-red ink `#991b1b` — rare hard-coded values; the danger token set should expose this triple via `--status-danger-bg-strong/ink-strong`.
- Sparkline is inline SVG (60×26 viewport, polyline) — emerald-500 stroke. Implementation should generate from the last 7-day metric series (e.g. open-faults daily snapshot).
- Trend pill colors: `up` and `dn` both use success when the *direction* is good (e.g. faults ↓ is good, occupancy ↑ is good). Implementation must track `direction-good: 'asc' | 'desc'` per metric — can't infer from arrow symbol alone.
- Severity pills: Urgent → danger-soft-bg + danger-700 ink; High → orange-soft + orange-700; Normal → blue-soft + blue-700; Draft → neutral-soft + neutral-700. Match the 8-state-machine token set.
- Action-queue "selected" row visual is critical for keyboard nav — the J/K navigation footer hint demands the selected state be visually distinct and the inline-action affordance only appear on the selection, not all rows.
- Command palette (`⌘K`) is the only escape hatch from any deep page; ppt-web nav is non-sticky per project README. Must wire `mod+k` handler globally.
- Conic-gradient ring `background: conic-gradient(var(--accent) calc(var(--p)*1%), var(--bg-subtle) 0)` — older browsers without conic-gradient need a fallback (Safari pre-12.1 etc., but those are out of support per project minimums).
- Activity timeline `border-bottom: 1px dashed` per item — last item drops the border. Watch for hairline rendering on retina + dark mode (alpha doubling already handled).
- Trend strings ("3 new since yesterday", "↓ 3 from last week") need `Intl.RelativeTimeFormat` for sk/cs/de/en/pl/hu — and pluralization rules.
- All metrics on the right rail are **read-only summaries** — links go elsewhere; no editing happens here.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (ui_kits/ppt-web/manager-dashboard.html); flipped ppt-web redesignStatus → in-progress; attached designSource; populated functionality checklist (5 sections), all 4 states, design-specific notes; linked UC-15/03/02/04/17; declared 6 sharedComponents; added 3 relatedScreens
- 2026-05-08 — init: created from scan (source: sitemap)
