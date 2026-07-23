---
id: ppt/fault-reports
name: Fault Reports
product: ppt
implementations:
  ppt-web:
    route: /faults/reports
    component: FaultReportsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: ppt/faults-list
    rel: parent
useCases:
  - UC-03
sharedComponents:
  - kpi-card
  - bar-chart
  - date-range-filter
  - building-selector
epics:
  - Epic-4
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Filters (web)
- [x] [w] Building selector (all buildings or a single building) → drives `useFaultStatistics`
- [x] [w] Date range (from / to) with min/max cross-bounding → `date_from` / `date_to`
- [x] [w] Manager-only gating — non-managers see an access notice (mirrors `isManagerRole`)

### KPIs (web)
- [x] [w] Total / Open / Closed counts
- [x] [w] Average resolution time (hours), `—` when null
- [x] [w] Average resident rating (1–5), `—` when null

### Charts (web)
- [x] [w] By-status breakdown bar chart (accessible labelled bars, share %)
- [x] [w] By-category breakdown bar chart
- [x] [w] By-priority breakdown bar chart
- [x] [w] Drill-down: clicking a bar navigates to the faults list pre-filtered (`/faults?status=…`)

### Export (web)
- [x] [w] Export CSV (client-side Blob, BOM-prefixed for Excel)
- [x] [w] Export PDF (browser print pipeline; print stylesheet hides controls)

## States

- **Empty**: when the filtered statistics return `total_count === 0`, a blunt single-line "No faults match the selected filters." is shown (filters retained).
- **Loading**: centered spinner with `aria-busy` while `useFaultStatistics` is fetching.
- **Error**: inline `role="alert"` danger tile ("Failed to load fault statistics") + `common.retry` button wired to `refetch()`.
- **Forbidden**: non-managers see an amber `role="alert"` access notice and no data/charts.

## Notes

### Broader context

UC-03 fault analytics (Story 4.7, BIT-186 / PM #970 gap 5). Consumes the
already-implemented `GET /faults/statistics` endpoint (`FaultStatistics`:
total/open/closed, by_status, by_category, by_priority,
average_resolution_time_hours, average_rating). Previously the endpoint was only
read by the faults-list summary bar (total/open/closed); this page is the
dedicated manager reporting surface.

### Specific (recent)

- **Endpoint catalogue drift**: the statistics endpoint has no `operationId` in
  `@ppt/sitemap` yet (only `faults_list` / `faults_get` / `faults_create` are
  catalogued), so `endpoints: []` here to keep `/screens validate` green. Add a
  `faults_statistics` operationId to the sitemap and reference it when the
  catalogue is extended.
- **Route not in sitemap**: like `ppt/faults-list`, `/faults/reports` is not in
  `@ppt/sitemap` `pptWebRoutes`, so `sitemapRefs` is omitted. Reachable from the
  faults list via a manager-only "View reports →" link.
- Status enum is the backend's (`new` / `waiting_parts` / …), not
  `reported` / `on_hold`; bucket keys are humanised for display
  (`waiting_parts` → "Waiting parts").
- Charts are dependency-free (Tailwind div-bars), matching the existing
  `features/reports` convention of avoiding a charting library. CSV/PDF export
  is client-side (no server export endpoint for statistics).

## Agent Log

<!-- newest entries on top -->

- 2026-06-22 — agent: BIT-186 (FrontendEngineer) — created from scratch for Story 4.7. New `/faults/reports` route + `FaultReportsPage` + `FaultBreakdownChart` + CSV/PDF export; extended `useFaultStatistics` to accept building + date-range filters; manager-only gating mirrors `isManagerRole`; drill-down into the pre-filtered faults list. ppt-web buildStatus shipped / apiStatus complete; mobile n/a. Tests added for page + chart; typecheck + biome green.
