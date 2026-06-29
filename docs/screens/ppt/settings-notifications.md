---
id: ppt/settings-notifications
name: Notification Preferences
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/settings/notifications"
    component: NotificationSettingsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
endpoints: []
relatedScreens:
  - id: ppt/settings-two-factor
    rel: sibling
  - id: ppt/notification-analytics
    rel: sibling
sharedComponents:
  - channel-toggle
  - disable-all-warning-dialog
diagrams: []
useCases:
  - UC-01.3
epics:
  - Epic-8A
designSources: []
owner: pm-frontend
---

# Notification Preferences

Channel-level notification toggles for the signed-in user (Epic 8A, Stories 8A.1 + 8A.3).
`NotificationSettingsPage` lets the user enable/disable each delivery channel (push, email,
in-app). Preferences are stored server-side and synced cross-device on page load via
`useNotificationPreferences` / `useUpdateNotificationPreference` from `@ppt/api-client`.

Disabling the last remaining enabled channel surfaces a confirmation guard
(`DisableAllWarningDialog`): the PATCH returns `409 ConfirmationRequired` and the UI
re-issues the update with the confirm flag once the user accepts.

## Backend contract

These operations are not yet registered in `@ppt/sitemap`, so `endpoints:` is left empty
(consistent with `ppt/notification-analytics`). The live api-server routes are:

- `GET /api/v1/users/me/notification-preferences` — read all channel preferences (RLS-scoped).
- `PATCH /api/v1/users/me/notification-preferences/{channel}` — update one channel; on
  Story 8A.3 it publishes a `preference.updated` event to Redis `notifications:{user_id}`
  for WebSocket clients to consume.

Source: `backend/servers/api-server/src/routes/notification_preferences.rs`,
`backend/crates/db/src/repositories/notification_preference.rs`.

## Notes

### Specific (recent)
- 2026-06-28 — agent: orphan-epic reconciliation for Epic 8A. AC-1 (cross-device sync via
  server-side storage) is shipped. AC-2 (real-time WebSocket push of `preference.updated`)
  is deferred — `usePreferenceSync` in `@ppt/api-client` is a placeholder returning
  `isConnected: false` pending WebSocket infrastructure (Epic 2B). AC-3 (mobile OS
  integration) is deferred — no RN/mobile screen yet (`mobile` impl = n/a).
- The advanced/granular category preferences live on a separate route
  (`/settings/notifications/advanced`, Epic 40) and are out of scope for this screen-map.

## Agent Log
- 2026-06-28 — agent: 8a-3-notification-preference-sync — verify-first coverage gap. Created
  the missing screen-map for the orphan Epic 8A route `/settings/notifications`; recorded
  AC-1 shipped, AC-2/AC-3 deferred. No code change — feature was already on dev.
