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

- 2026-07-01 — agent: gap-81-2 verify-to-done. Story 81.2 (Execution History) frontend is fully shipped and green: `ExecutionHistory`/`HistoryFilters` (status + date-range filter), offset/limit pagination, download and retry actions are wired end-to-end through the real api-client hooks (`useReportExecutionHistory`, `useDownloadReport`, `useRetryReportExecution`) in both `routes/groups/reports.tsx` route containers (`ReportsPageRoute`, `ScheduleDetailPageRoute`). The download hook guards empty presigned URLs and cleans up the temporary `<a>` in a `finally`; retry invalidates the executions query. The earlier download/retry test-gap follow-up (d1785e5fb) is closed: `routes/groups/reports.download-retry.route.test.tsx` now asserts the PRODUCTION route's `onError` download toast + retry-success toast (the sibling page Harness diverged there). Verified: `pnpm -F @ppt/web test --run reports` → 6 files / 70 tests pass; `pnpm -F @ppt/web typecheck` clean. `apiStatus` stays **partial** — pinned reason below — because the backend HTTP routes the hooks target do NOT exist yet (only DB models/repos in `crates/db`): the frontend is a `rust-backend` task away from live data. Frontend side of 81.2 is done; no frontend work remains.
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

> **apiStatus: partial — pinned reason (2026-07-01).** The Story 81.2 frontend is
> complete and tested; `partial` reflects a *backend* gap, not frontend work.
> The api-client hooks call `GET /api/v1/reports/schedules/{id}/executions`,
> `GET /api/v1/reports/executions/{id}/download`, and
> `POST /api/v1/reports/executions/{id}/retry`, but the backend only ships DB
> models/repos for report executions (`crates/db/.../report_schedule.rs`,
> migration `00162_create_report_schedules_executions.sql`) — no axum routes are
> registered. Flipping `apiStatus` to `wired` is a `rust-backend` task (register
> the three routes + presigned-download URL), out of scope for `pm-frontend`.
