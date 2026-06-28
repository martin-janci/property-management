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

- 2026-06-28 — agent: gap-81-1 follow-up (#1368): reconciled frontend `isValidCron` (CronPicker) with the backend `validate_cron_expression` parse order — split each field on `,` first, then `/`, then `-`. Previously `1-5/2,10` was a frontend false-negative; because `scheduleToInitialCron` gates surfacing the persisted `cron_expression` on `isValidCron`, that silently flattened a backend-accepted cron back to the legacy time-derived form (silent #616 reintroduction). Flipped cron-validator-drift regression test to assert the validators now agree and the read path surfaces the persisted cron verbatim. apiStatus still partial (schedule create stub remains).
- 2026-05-27 — agent: gap-81-1: EditScheduleModal rebuilt for cron-based PUT endpoint. CronPicker added (preset tabs + custom free-text with live validation). Pause/Resume props replaced by enabled toggle in modal (maps to `enabled` field in CronScheduleUpdateRequest). useUpdateScheduleCron hook wired. apiStatus remains partial (schedule create stub still absent).
- 2026-05-25 — agent: Created screen-map. Route /reports wrapped in ProtectedRoute (PR #489 fix). Hooks pagination cache key fixed (executionOffset included). apiStatus=partial (schedule CRUD + execution history wired; download/retry stubs).

## Functionality Checklist

### Schedule Management (Story 81.1)
- [x] [w] List report schedules
- [x] [w] Update schedule (cron expression, recipients, enabled)
- [x] [w] Enable / disable schedule (enabled toggle in EditScheduleModal)
- [ ] [w] Create new schedule

### Execution History (Story 81.2)
- [x] [w] List executions with status filter
- [x] [w] Date range filter
- [x] [w] Pagination (offset/limit)
- [x] [w] Retry failed execution
- [x] [w] Download completed report
