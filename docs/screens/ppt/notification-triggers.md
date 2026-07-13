---
id: ppt/notification-triggers
name: Notification Triggers
product: ppt
implementations:
  ppt-web:
    route: /notifications/triggers
    component: NotificationTriggersPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens:
  - ppt/notification-analytics
useCases:
  - UC-01
sharedComponents:
  - data-table
  - toggle
epics: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Trigger list (web)
- [x] [w] Triggers (event types) grouped by category (fault / vote / announcement / document / message / critical / finance / facility)
- [x] [w] Per-category header shows the display name and `{enabled} of {total} enabled` rollup
- [x] [w] Priority triggers (e.g. emergency alerts) show a "Priority" badge

### Channel toggles (web)
- [x] [w] Push / Email / In-app checkbox per trigger → `PUT …/granular/events/{eventType}`
- [x] [w] Priority triggers keep the in-app channel forced on + disabled (backend enforces)
- [x] [w] Row inputs disabled while its update is in flight (`savingEventType`)
- [x] [w] Save failure surfaces an error toast; the query invalidates on success to refresh rollups

### Reset (web)
- [x] [w] "Reset to defaults" → `POST …/granular/events/reset`, success/error toast, cache updated

## States

- **Empty**: when `preferences` is empty, a single-line "No notification triggers are available for your account." is shown.
- **Loading**: centered spinner with `aria-busy` while `useNotificationTriggers` is fetching.
- **Error**: inline `role="alert"` danger tile ("Failed to load notification triggers") + `common.retry` button wired to `refetch()`.
- **Forbidden**: a backend `401`/`403` surfaces an amber `role="alert"` access notice and no table.

## Notes

### Broader context

User-facing management UI for the Notification Trigger System (Story 84.4). Lets
a user choose which events notify them and on which channels, over the granular
notification-event API (Epic 8B) at
`/api/v1/users/me/notification-preferences/granular/events`. Closes the PM gap
"No frontend notification-trigger management UI (84-4)". Complements the operator
delivery dashboard at `ppt/notification-analytics`.

### Specific (recent)

- **Endpoint not catalogued**: the granular `/events` endpoints have no
  `operationId` in `@ppt/sitemap` and are absent from the generated
  `@ppt/api-client`, so `endpoints: []` here (keeps `/screens validate` green) and
  the hooks call the REST path directly via TanStack Query — the
  `notification-analytics` / `admin/audit` precedent.
- **Route not in sitemap**: `/notifications/triggers` is not in `@ppt/sitemap`
  `pptWebRoutes`, so `sitemapRefs` is omitted. It shares the `routes/groups/notifications.tsx`
  group with `/notifications/analytics`.
- Auth: the bearer token is read from `localStorage` (`ppt_access_token`) in the
  fetch helpers, matching `authApiClient` / `gdprClient` / `useNotificationAnalytics`.

## Agent Log

<!-- newest entries on top -->

- 2026-07-13 — agent: gap-84-4 (react-web) — created from scratch. New `/notifications/triggers` route + `NotificationTriggersPage` + `useNotificationTriggers` / `useUpdateNotificationTrigger` / `useResetNotificationTriggers` (direct REST over granular `/events`, analytics precedent). Category-grouped trigger list, per-channel toggles, forced-on in-app for priority triggers, reset-to-defaults, toast feedback. ppt-web buildStatus shipped / apiStatus complete; mobile n/a. 10 page tests; typecheck + biome + vitest + Vite build green.
