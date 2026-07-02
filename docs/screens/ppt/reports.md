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
- [ ] [w] Create new schedule

### Execution History (Story 81.2)
- [x] [w] List executions with status filter
- [x] [w] Date range filter
- [x] [w] Pagination (offset/limit)
- [x] [w] Retry failed execution
- [x] [w] Download completed report

> **apiStatus: partial — pinned reason (2026-07-01, corrected).** Story 81.2
> (Execution History) is fully wired on BOTH sides: the api-client hooks call
> `GET /api/v1/reports/schedules/{id}/executions`,
> `GET /api/v1/reports/executions/{id}/download`, and
> `POST /api/v1/reports/executions/{id}/retry`, and all three routes are
> registered on `dev` in `backend/servers/api-server/src/routes/reports.rs`
> (`router()`, lines 222-228: `list_schedule_executions`, `get_execution_download_url`
> which builds the presigned-download URL, and `retry_execution`), nested at
> `/api/v1/reports` in `backend/servers/api-server/src/lib.rs:293`. (An earlier
> version of this note incorrectly claimed those routes were missing — that was
> false; corrected in PR #2004 round-1 follow-up.)
>
> `apiStatus` remains **partial** because of the Story 81.1 **"Create new
> schedule"** action (see the unchecked checklist item above): there is no
> backend create route — `router()` registers only `PUT /schedules/{id}` plus
> `/schedules/{id}/pause` and `/schedules/{id}/resume`, no `POST /schedules`
> handler. Flipping `apiStatus` to `wired` is a `rust-backend` task (add the
> schedule-create route + handler), out of scope for `pm-frontend`.
