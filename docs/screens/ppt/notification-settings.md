---
id: ppt/notification-settings
name: Notification Settings
product: ppt
sitemapRefs:
  ppt-web: ppt-settings-notifications
implementations:
  ppt-web:
    component: NotificationSettingsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
  mobile:
    buildStatus: n/a
    redesignStatus: n/a
    apiStatus: n/a
epics:
  - Epic-8A
relatedScreens:
  - id: ppt/accessibility-settings
    rel: sibling
  - id: ppt/privacy-settings
    rel: sibling
sharedComponents:
  - switch
  - banner
  - modal-drawer
  - section-card
  - settings-side-nav
useCases: []
endpoints:
  - notification_preferences_list
  - notification_preference_update
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Layout shell
- [x] [w] Page container (`max-w-2xl`, centered) with H1 "Notification Settings" + lede "Choose how you want to receive notifications."
- [ ] [w] Settings shell side-nav (Profil / Predvoľby / Prístupnosť / Súkromie / **Upozornenia**) — currently the page renders standalone; side-nav integration is pending (matches accessibility/privacy-settings notes that flag Upozornenia as a placeholder target).

### A · Channel toggles (Story 8A.1, AC-1/AC-2/AC-3)
- [x] [w] One `ChannelToggle` row per preference returned by the API. Each row = label (`CHANNEL_LABELS[channel]`) + description (`CHANNEL_DESCRIPTIONS[channel]`) on the left, switch on the right.
- [x] [w] Three channels supported: `push`, `email`, `in_app`.
- [x] [w] Switch is an accessible `role="switch"` button with `aria-checked`; disabled (opacity-50) while a mutation is in flight (`updatePreference.isPending`).
- [x] [w] Toggle calls `PATCH /api/v1/users/me/notification-preferences/{channel}` with `{ enabled }`; optimistic via TanStack mutation.

### B · Disable-all guard (Story 8A.1, AC-3)
- [x] [w] Turning off the **last** active channel triggers a `ConfirmationRequiredError` (backend 409) → opens `DisableAllWarningDialog` instead of applying.
- [x] [w] Confirm re-sends the PATCH with `{ enabled: false, confirmDisableAll: true }`; cancel closes the dialog and leaves state unchanged.
- [x] [w] When all channels are already disabled, the GET response carries `allDisabledWarning`, shown as an amber warning banner above the toggles.

### C · Feedback / errors
- [x] [w] Mutation failure (non-confirmation) shows a dismissible red error alert ("Failed to update preference. Please try again.").
- [x] [w] Initial load failure shows a red "Failed to load notification preferences" panel.
- [x] [w] Loading state shows a 3-row skeleton (animate-pulse).
- [x] [w] "Last updated: <timestamp>" line derived from `max(preferences[].updatedAt)`.

### D · Realtime sync (Story 8A.3)
- [x] [w] After a successful update the backend publishes `preference.updated` on `notifications:{user_id}` (Redis pub/sub); the api-client `sync.ts` refreshes cached preference state over WebSocket without a full poll.

## States

- **Loading**: 3-row skeleton; no toggles interactable.
- **Loaded (default)**: 3 channel rows (push / email / in-app) with their enabled state; optional amber all-disabled banner; "Last updated" footer.
- **Mutating**: switches disabled (opacity-50) while `isPending`.
- **Disable-all confirm**: modal dialog gates the final-channel-off action; PATCH only fires on confirm with `confirmDisableAll: true`.
- **Update error**: dismissible red alert above the toggle card; local switch state reverts to server truth on refetch.
- **Load error**: full red panel, no toggles.

## Notes

### Broader context

Epic 8A, Story 8A.1 — channel-level notification toggles. The screen is the user-facing surface over `notification_preferences` (push / email / in_app), backed by `GET` + `PATCH /api/v1/users/me/notification-preferences`. Default preferences (all enabled) are auto-created per user by a DB trigger (migration `00021_create_notification_preferences.sql`). Story 8A.3 layers realtime sync on top so multiple sessions stay consistent.

There is a sibling **Advanced** route (`/settings/notifications/advanced`, `AdvancedNotificationSettingsPage`, Epic 40) for per-category preferences, digests, quiet hours and grouping — catalogued in the sitemap as `ppt-settings-notifications-advanced` and out of scope for this 8A.1 screen-map. It now has its own screen-map at `ppt/settings-notifications-advanced` (child of this screen).

### Specific (recent)

- Channels are a closed set of three (`push`, `email`, `in_app`) defined by the `notification_channel` Postgres ENUM and mirrored in the api-client `NotificationChannel` type. Adding a channel requires a migration + type + label/description constant.
- The disable-all guard is enforced **server-side** (the PATCH returns 409 / `ConfirmationRequiredError` for the last active channel); the dialog is purely the client surface for that contract. Do not rely on client-only counting.
- The page currently renders standalone (no shared settings side-nav). `accessibility-settings` and `privacy-settings` both ship the side-nav and flag "Upozornenia" as a placeholder target — wiring this page into that shell is the natural redesign step (`redesignStatus: not-started`).
- Labels/descriptions come from `CHANNEL_LABELS` / `CHANNEL_DESCRIPTIONS` in `@ppt/api-client` (English only today); i18n is a follow-up.
- 2026-08-31 — realtime sync (PR #2889): the Story 8A.3 cross-session refresh path was only partly live. The api-client `sync.ts` WebSocket path documented above is separate from the app-wide TanStack invalidation in `WebSocketContext`, which was keyed on dead `entity:*` names the api-server never emits — so `preference.updated` (and `notification.created`) never invalidated the `notifications` query root. PR #2889 re-keyed `eventToQueryKeys['preference.updated'] → ['notifications']` and wired `App.tsx`'s `onEntityEvent`, so the notifications root now refetches on a preference change from another session. No change to this screen's own PATCH/GET wiring or status.

## Agent Log

<!-- newest entries on top -->

- 2026-08-31 — agent: screen-map-drift-pr-2889-ppt — reconcile drift from PR #2889 (realtime ws→query-invalidation fix). ppt-web `WebSocketContext` re-keyed cache invalidation from dead `entity:*` names to canonical `domain.action` events (`preference.updated → ['notifications']`, plus a polymorphic `notification.created` subscriber routing by `payload.category`), and `App.tsx` now wires `onEntityEvent → queryClient.invalidateQueries`. This is the app-wide invalidation counterpart to the Story 8A.3 `sync.ts` path already noted in Functionality D. No route/component/endpoint/status change — frontmatter unchanged; docs-only.

- 2026-06-28 — agent (coverage gap 8a-1): created screen-map for the already-shipped Channel-Level Notification Toggles feature (orphan epic — code on dev, no map). Added sitemap routes `ppt-settings-notifications` + `ppt-settings-notifications-advanced` and api-server endpoints `notification_preferences_list` / `notification_preference_update`; linked as sibling of accessibility/privacy-settings. buildStatus shipped, apiStatus complete, redesignStatus not-started.
