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
    apiStatus: complete
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

- 2026-07-15 — agent: react-web (gh-issue-2324, follow-up to PR #2292). Closed the "flow still broken in prod" regression PR #2292 introduced: its two list hooks called routes that did not exist. **Backend:** added `GET /api/v1/reports/schedules` → `list_schedules` (org-scoped via `RlsConnection`, reusing new `ReportScheduleRepository::list_by_org`) so `useReportSchedules` → `listSchedules()` loads instead of 405ing (the path was POST-only). **Report selector:** dropped the dead `useReports` → `/definitions` call (no report-definitions table by design, #2198). The "New Schedule" report selector is now fed by a fixed built-in report set (`features/reports/builtinReports.ts`: faults / voting / occupancy / consumption → well-known non-nil UUIDs); `useReports`/`listReports` removed from `@ppt/api-client`. **Frequency:** ScheduleForm no longer offers quarterly/yearly (backend rejects them 400 INVALID_FREQUENCY); create error toast now surfaces the backend message. **Cron:** `CreateReportSchedule` gained optional `cron_expression`, derived from the form fields on submit so new rows are cron-canonical. **i18n:** added `reports.schedule.{created,createFailed,updated,updateFailed}` to all 6 locales. **Tests:** new fetch-layer contract test (`packages/api-client/src/reports/schedules.api.test.ts`) pins GET/POST `/schedules` URL+method so a route/method drift fails CI; updated the create-schedule route test to the built-in selector.
- 2026-07-15 — agent: pm-backend (gap-tracking-stale-status-flips). apiStatus flipped **partial → complete**. The sole remaining blocker recorded in the pinned reason — the missing Story 81.1 "Create new schedule" backend route — is now closed. `backend/servers/api-server/src/routes/reports.rs::router()` registers `POST /schedules → create_schedule` (real handler: manager-tier RBAC from the DB-backed `RlsConnection`, frequency/day-of-week/day-of-month validation, tenant-scoped insert), landed via PR #2313 (report_schedules due-work consumer + unified cadence). Frontend wired end-to-end: `@ppt/api-client` `createSchedule` (POST /schedules) + `useCreateSchedule`, consumed by `ReportsPageRoute` (`routes/groups/reports.tsx`) → `handleCreateSchedule` → `mutateAsync` with success/error toast; the "New Schedule" form's report selector is fed by `useReports`. Story 81.2 (execution history: list/filter/paginate/retry/download) was already wired both sides. All checklist items now satisfied.
- 2026-07-01 — agent: react-web PR #2004 follow-up (round 1). CORRECTION to the pinned reason below: the earlier claim that the backend axum routes for report executions are NOT registered was FALSE. All three routes ARE registered on `dev` in `backend/servers/api-server/src/routes/reports.rs::router()` — `GET /schedules/{id}/executions` → `list_schedule_executions` (line 222), `GET /executions/{id}/download` → `get_execution_download_url` (line 224, builds the presigned-download URL), `POST /executions/{id}/retry` → `retry_execution` (lines 225-228); the router is nested at `/api/v1/reports` in `backend/servers/api-server/src/lib.rs:293`. The Story 81.2 execution-history flow is therefore backend-wired end-to-end. `apiStatus` stays **partial** for a DIFFERENT, still-genuine reason: the Story 81.1 "Create new schedule" action (`POST /api/v1/reports/schedules`) has no backend route — `router()` only registers PUT `/schedules/{id}` plus pause/resume, no create handler. See corrected pinned reason below.
- 2026-07-01 — agent: gap-81-2 verify-to-done. Story 81.2 (Execution History) frontend is fully shipped and green: `ExecutionHistory`/`HistoryFilters` (status + date-range filter), offset/limit pagination, download and retry actions are wired end-to-end through the real api-client hooks (`useReportExecutionHistory`, `useDownloadReport`, `useRetryReportExecution`) in both `routes/groups/reports.tsx` route containers (`ReportsPageRoute`, `ScheduleDetailPageRoute`). The download hook guards empty presigned URLs and cleans up the temporary `<a>` in a `finally`; retry invalidates the executions query. The earlier download/retry test-gap follow-up (d1785e5fb) is closed: `routes/groups/reports.download-retry.route.test.tsx` now asserts the PRODUCTION route's `onError` download toast + retry-success toast (the sibling page Harness diverged there). Verified: `pnpm -F @ppt/web test --run reports` → 6 files / 70 tests pass; `pnpm -F @ppt/web typecheck` clean. (NOTE: this entry's original claim that the 81.2 backend routes do not exist was later corrected — see the 2026-07-01 react-web follow-up entry above; the routes ARE registered.) Frontend side of 81.2 is done; no frontend work remains.
- 2026-06-28 — agent: gap-81-1 follow-up (#1368): reconciled frontend `isValidCron` (CronPicker) with the backend `validate_cron_expression` parse order — split each field on `,` first, then `/`, then `-`. Previously `1-5/2,10` was a frontend false-negative; because `scheduleToInitialCron` gates surfacing the persisted `cron_expression` on `isValidCron`, that silently flattened a backend-accepted cron back to the legacy time-derived form (silent #616 reintroduction). Flipped cron-validator-drift regression test to assert the validators now agree and the read path surfaces the persisted cron verbatim. apiStatus still partial (schedule create stub remains).
- 2026-05-27 — agent: gap-81-1: EditScheduleModal rebuilt for cron-based PUT endpoint. CronPicker added (preset tabs + custom free-text with live validation). Pause/Resume props replaced by enabled toggle in modal (maps to `enabled` field in CronScheduleUpdateRequest). useUpdateScheduleCron hook wired. apiStatus remains partial (schedule create stub still absent).
- 2026-05-25 — agent: Created screen-map. Route /reports wrapped in ProtectedRoute (PR #489 fix). Hooks pagination cache key fixed (executionOffset included). apiStatus=partial (schedule CRUD + execution history wired; download/retry stubs).

## Functionality Checklist

### Schedule Management (Story 81.1)
- [x] [w] List report schedules
- [x] [w] Update schedule (cron expression, recipients, enabled)
- [x] [w] Enable / disable schedule (enabled toggle in EditScheduleModal)
- [x] [w] Create new schedule (POST /schedules → create_schedule; useCreateSchedule wired in ReportsPageRoute)

### Execution History (Story 81.2)
- [x] [w] List executions with status filter
- [x] [w] Date range filter
- [x] [w] Pagination (offset/limit)
- [x] [w] Retry failed execution
- [x] [w] Download completed report

> **apiStatus: complete — resolved 2026-07-15 (gap-tracking-stale-status-flips).**
> Both Story 81.1 (Schedule Management) and Story 81.2 (Execution History) are
> now fully wired on BOTH sides.
>
> Story 81.2: api-client hooks call `GET /api/v1/reports/schedules/{id}/executions`,
> `GET /api/v1/reports/executions/{id}/download`, and
> `POST /api/v1/reports/executions/{id}/retry`; all three routes are registered
> in `backend/servers/api-server/src/routes/reports.rs::router()`
> (`list_schedule_executions`, `get_execution_download_url` which builds the
> presigned-download URL, and `retry_execution`), nested at `/api/v1/reports` in
> `backend/servers/api-server/src/lib.rs`.
>
> Story 81.1 **"Create new schedule"** is closed: `router()` registers
> `POST /schedules → create_schedule` (real handler with manager-tier RBAC
> derived from the DB-backed `RlsConnection`, frequency + day-of-week/day-of-month
> validation, tenant-scoped insert), landed in PR #2313. The frontend "New
> Schedule" form submits through `@ppt/api-client` `createSchedule` (POST
> /schedules) + `useCreateSchedule`, wired in `ReportsPageRoute`
> (`routes/groups/reports.tsx`).
>
> **List/selector wiring (gh-issue-2324, follow-up to PR #2292).** The Schedules
> tab list is now backed by `GET /api/v1/reports/schedules` → `list_schedules`
> (org-scoped, `ReportScheduleRepository::list_by_org`); before this the router
> registered `/schedules` POST-only, so `useReportSchedules` 405'd. The report
> selector no longer calls a nonexistent `/definitions` list route — it uses the
> fixed built-in report set (`features/reports/builtinReports.ts`), since
> `report_id` is opaque with no definitions table (#2198). No wiring gaps remain.
