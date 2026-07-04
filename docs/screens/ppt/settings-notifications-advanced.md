---
id: ppt/settings-notifications-advanced
name: Advanced Notification Settings
product: ppt
sitemapRefs:
  ppt-web: ppt-settings-notifications-advanced
implementations:
  ppt-web:
    route: "/settings/notifications/advanced"
    component: AdvancedNotificationSettingsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: partial
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
epics:
  - Epic-40
relatedScreens:
  - id: ppt/notification-settings
    rel: parent
  - id: ppt/accessibility-settings
    rel: sibling
  - id: ppt/privacy-settings
    rel: sibling
sharedComponents:
  - switch
  - section-card
  - tabs
useCases: []
endpoints: []
diagrams: []
owner: pm-frontend
---

# Advanced Notification Settings

Manager/tenant-web surface for Epic 40 — advanced notification preferences that
layer on top of the channel-level toggles in `ppt/notification-settings`
(`/settings/notifications`). Route `/settings/notifications/advanced` is wired in
`routes/groups/core.tsx` behind `<ProtectedRoute>` to the lazy
`AdvancedNotificationSettingsPage`
(`features/settings/notifications/advanced/AdvancedNotificationSettingsPage.tsx`).

The page is a tabbed surface (`categories` / `schedule` / `grouping`) backed by
the `@ppt/api-client` hooks `useCategoryPreferences`, `useQuietHours`,
`useDigestPreferences`, `useGroupingPreferences` and their matching
`useUpdate*` mutations.

- **Story 40.1 — Granular category preferences**: per-category channel toggles
  (`CategoryPreferenceCard`).
- **Story 40.2 — Quiet hours**: quiet-hours window configuration
  (`QuietHoursConfig`).
- **Story 40.3 — Digest preferences**: batched digest cadence
  (`DigestPreferences`).
- **Story 40.4 — Smart notification grouping**: grouping rules
  (`GroupingSettings`).

## States

- **Loading**: combined `isLoading` across the four preference queries drives a
  loading state before any tab renders.
- **Loaded**: active tab (`categories` by default) renders its section from the
  corresponding query data.
- **Saving**: per-section update mutations disable their controls while pending.
- **Error**: any of the four query errors surfaces an error state; per-update
  failures set a dismissible `updateError` banner
  ("Failed to update … preference. Please try again.").

## Notes

### Broader context

Sitemap route `ppt-settings-notifications-advanced` (child of
`ppt-settings-notifications`, `feature: Epic-40`). This screen-map was
previously left unmapped — `ppt/notification-settings` catalogued the route in
prose but scoped it out of the 8A.1 map, so the sitemap ID had no screen-map
`sitemapRefs` owner. This map closes that route↔screen gap.

### Specific (recent)

- `apiStatus: partial` — the four advanced-preference endpoint families
  (category / quiet-hours / digest / grouping) are consumed via `@ppt/api-client`
  hooks; endpoints are left unlisted here until their sitemap `apiEndpoint` IDs
  are catalogued, to avoid introducing unknown-endpoint drift.

## Agent Log

<!-- newest entries on top -->

- 2026-07-04 — agent (test-gap-screen-map-drift-pr-922): created this screen-map
  to reconcile screen-map drift after ppt-web route changes. Maps the
  already-shipped `/settings/notifications/advanced` route
  (`AdvancedNotificationSettingsPage`, Epic 40) to sitemap
  `ppt-settings-notifications-advanced`, which no screen-map previously
  referenced. Docs-only.
