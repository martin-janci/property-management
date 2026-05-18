---
id: ppt/faults-list
name: Faults List
product: ppt
sitemapRefs:
  mobile: mobile-faults-list
implementations:
  ppt-web:
    component: FaultsListPage
    buildStatus: planned
    redesignStatus: in-progress
    apiStatus: stub
  mobile:
    component: FaultsListScreen
    buildStatus: shipped
    redesignStatus: in-progress
    apiStatus: partial
endpoints:
  - faults_list
relatedScreens:
  - id: ppt/home
    rel: parent
  - id: ppt/report-fault
    rel: sibling
sharedComponents:
  - status-pill
  - data-table
  - search-bar
  - segmented-control
  - sidebar-filter
designSources:
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/ppt-web/faults-list.html
    frame: faults-list-table
  - adapter: claude-design
    file: guest-registration-v2-design-system/project/ui_kits/mobile/screens.jsx
    frame: mobile-faults-list
useCases:
  - UC-03
epics: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Header (web)
- [ ] [w] PPT manager header (60px, sticky) with nav (Dashboard / Faults active / Announcements / Votes / Residents / Documents)
- [ ] [w] Breadcrumb "Dashboard / Faults" + H1 "Faults" + subtitle "12 open · 3 awaiting triage · SLA breach in 2 items"

### Toolbar (web)
- [ ] [w] Search input (max 360px, magnifier icon left): "Search faults, unit, reporter…"
- [ ] [w] Segmented filter: All · 38 / Open · 12 / Mine · 5 / Breached · 2
- [ ] [w] Right actions: Export CSV (secondary) + "+ New fault" (primary)

### Sidebar filters (web, 220px)
- [ ] [w] Group "Priority": All priorities · Urgent · High · Normal · Low (with counts right-aligned tabular-nums)
- [ ] [w] Group "Status": Reported · Triaged · Assigned · In progress · Awaiting parts · Resolved · Closed · Reopened (8-state machine — UC-03)
- [ ] [w] Group "Category": Plumbing · Electrical · Heating · Structural · Other
- [ ] [w] Active filter row gets brand-soft bg + brand-600 ink + bold

### Table (web)
- [ ] [w] Header row (uppercase, 11px / 600, .06em tracking): ID · Fault · Status · Priority · Assignee · Age
- [ ] [w] Row anatomy: monospace ID (`#F-284`) + Fault title bold + small unit/reporter line + status pill (with colored dot) + priority pill + assignee (24px mini-avatar + name OR italic muted "Unassigned") + relative age tabular-nums
- [ ] [w] Row hover bg-subtle; click → fault detail (planned)
- [ ] [w] Footer: "Showing N of M" + pagination "Page X of Y"

### Status pills (UC-03 8-state machine)
- [ ] [w,m] Reported → amber bg/ink with amber dot
- [ ] [w,m] Triaged → indigo soft (not depicted in main rows but present in sidebar count)
- [ ] [w,m] Assigned → indigo soft + indigo dot
- [ ] [w,m] In progress → blue soft + blue dot
- [ ] [w,m] Awaiting parts → orange soft (`#fed7aa` / `#9a3412`) + orange dot
- [ ] [w,m] Resolved → green soft + emerald dot
- [ ] [w,m] Closed → neutral gray + gray dot
- [ ] [w,m] Reopened → red soft + red dot

### Mobile (RN)
- [ ] [m] Bottom-tab "Faults" (Lucide wrench icon, replacing legacy 🔧)
- [ ] [m] List view of fault cards stacked vertically; each card shows ID, title, unit, status pill, priority pill, age — same token system, single-column compact layout
- [ ] [m] Pull-to-refresh; empty/loading/error states (per state-pattern card spec)

## States

- **Empty**: not depicted in design; recommended — empty card with check-circle icon + "All caught up" + "No open faults right now." (per project README empty-state voice).
- **Loading**: not depicted; recommend table skeleton (8 rows × 6 cols) + sidebar group title skeletons; reduced-motion → static.
- **Error**: not depicted; per voice → blunt single-line "Unable to load faults. Try again". Sidebar + toolbar retained.
- **Success**: 7-row sample of 38 total, mixed priorities and statuses, demonstrating all 8 status-pill colors via varied row coverage.

## Notes

### Broader context

UC-03 faults list — the manager's working surface for the 8-state fault machine. The design exposes filter density (priority × status × category) on a 220px left rail and the data-table on the right. Mobile mirror is the resident's report+watch surface.

### Specific (recent)

- **Drift note**: The sitemap doesn't include a `ppt-faults` route, but the design exists and the codebase has the underlying state machine. Bumping this screen-map's `ppt-web.buildStatus` from `n/a` → `planned` so the screen is tracked. Add the route + page when the redesign is implemented.
- Status-pill colors per the 8-state machine come from `colors_and_type.css` `--status-fault-*` token pairs — the bundle currently inlines hex values per-pill (legacy artifact); production must use tokens.
- Sidebar counts are dynamic (must update on filter change). All-priorities/All-status defaults; multi-select filtering (clicking multiple priorities ANDs them together).
- "SLA breached" tag in subtitle is a meta count — should expose breach reason on hover (which SLA tier and by how long).
- Data-table column widths: `72px 1fr 140px 140px 140px 100px` — ID + flexible fault + 3 fixed status columns + age. Below 1024px, collapse status/priority/assignee into a stacked sub-line under the title (mobile-style).
- The bundle's faults page does not depict empty/loading/error states — these need design at implementation time. Following project voice: blunt, single-line, action-tagged.
- Mobile nav shouldn't carry the legacy emoji `🔧` — substitute Lucide `wrench` per SKILL.md non-negotiable.
- Sidebar group headers use uppercase 11/600/.06em — matches the spec for "category eyebrow" type.

## Agent Log

<!-- newest entries on top -->

- 2026-05-09 — agent: design analyzed (ui_kits/ppt-web/faults-list.html + ui_kits/mobile/screens.jsx); flipped ppt-web from n/a → planned + redesignStatus in-progress (drift: route not in sitemap but design + state machine exist); flipped mobile redesignStatus → in-progress; attached 2 designSources; populated functionality checklist (5 sections + 8-state pill set), states, design-specific notes; declared 5 sharedComponents; added 2 relatedScreens
- 2026-05-08 — init: created from scan (source: sitemap)
