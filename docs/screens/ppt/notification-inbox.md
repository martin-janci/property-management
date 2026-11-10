---
id: ppt/notification-inbox
name: Notification Inbox
product: ppt
implementations:
  ppt-web:
    route: /notifications
    component: NotificationInboxPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
  mobile:
    buildStatus: planned
    redesignStatus: n/a
    apiStatus: complete
endpoints: []
relatedScreens:
  - id: ppt/notification-triggers
    rel: sibling
  - id: ppt/settings-notifications
    rel: sibling
useCases:
  - UC-01
sharedComponents: []
epics: []
diagrams: []
owner: pm-frontend
---

## Functionality Checklist

<!-- tag with [w] / [m] / [w,m] / [-] -->

### Inbox (web)
- [x] [w] Notification groups, newest first, with up to 5 latest entries each → `GET …/granular/groups`
- [x] [w] Unread groups highlighted; "Show read" toggle (`includeRead=true`)
- [x] [w] Mark one group read → `POST …/granular/groups/{id}/read`
- [x] [w] Mark all read → `POST …/granular/groups/read-all`
- [x] [w] Remove a group → `DELETE …/granular/groups/{id}`
- [x] [w] Top-bar bell with unread badge (polled every 60 s) links here
- [ ] [m] Mobile inbox (RN `NotificationsScreen` still on mock data)

## States

- **Empty**: "You are all caught up." (unread only) or "You have no notifications." (with read).
- **Loading**: centered spinner with `aria-busy`.
- **Error**: inline `role="alert"` tile + retry.

## Notes

### Broader context

Gap audit R6 P1-8. The backend NotificationService has always stored in-app
notifications in `notification_groups` / `grouped_notifications` (Epic 29), but
ppt-web never read them, so a notification missed in the toast was lost.

### Specific (recent)

- The inbox lives under the granular notification-preferences base path
  (`/api/v1/users/me/notification-preferences/granular/groups`); hooks are in
  `@ppt/api-client` (`granular-notifications`: `useNotificationInbox`,
  `useUnreadNotificationCount`, mark/delete mutations).
- Groups created by NotificationService carry `entityType = "notification"`, so
  there is no deep link to the source entity yet.

## Agent Log

<!-- newest entries on top -->

- 2026-11-09 — agent: gap-audit-r7 — created. `/notifications` inbox page + top-bar bell; NotificationService now writes in-app groups through `add_notification_to_group_for_user` (RLS-safe).
