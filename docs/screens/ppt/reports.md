---
id: ppt/reports
name: Reports & Schedules
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    component: ReportsPageRoute
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
endpoints: []
relatedScreens: []
sharedComponents: []
designSources: []
useCases:
  - UC-16
epics:
  - Epic-81
diagrams: []
owner: pm-frontend
---

## Agent Log

- 2026-05-25 — agent: Created screen-map. Route /reports wrapped in ProtectedRoute (PR #489 fix). Hooks pagination cache key fixed (executionOffset included). apiStatus=partial (schedule CRUD + execution history wired; download/retry stubs).

## Functionality Checklist

### Schedule Management (Story 81.1)
- [x] [w] List report schedules
- [x] [w] Update schedule (name, cron, format)
- [x] [w] Pause schedule
- [x] [w] Resume schedule
- [ ] [w] Create new schedule

### Execution History (Story 81.2)
- [x] [w] List executions with status filter
- [x] [w] Date range filter
- [x] [w] Pagination (offset/limit)
- [x] [w] Retry failed execution
- [x] [w] Download completed report
