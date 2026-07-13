---
id: ppt/admin-system-announcements
name: System Announcements (Admin)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: platform/announcements
    component: SystemAnnouncementsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - list_system_announcements
  - create_system_announcement
  - get_system_announcement
  - update_system_announcement
  - delete_system_announcement
relatedScreens:
  - id: ppt/admin-platform-health
    rel: sibling
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-10B
designSources: []
owner: pm-frontend
---

## Functionality Checklist

- [w] List all platform-wide announcements with status badges (Active / Scheduled / Expired)
- [w] Toggle to include soft-deleted announcements ("Show deleted")
- [w] Create announcement: title, message, severity (info / warning / critical), start_at, end_at (optional), is_dismissible, requires_acknowledgment
- [w] Live banner preview in the create/edit form
- [w] Edit announcement: all fields pre-populated, same form as create
- [w] Delete announcement (soft-delete) with confirmation dialog
- [w] Severity colour-coding: info = blue, warning = amber, critical = red
- [w] All write actions gated by `site_settings_write` capability
- [w] Navigation item in PLATFORM sidebar group, gated by `site_settings_write`

## States

- **Loading**: "Loading…" placeholder while fetching announcements
- **Error**: Error message displayed when fetch fails
- **Empty list**: "No announcements found." message
- **Create form**: Inline form with live banner preview above input fields
- **Edit form**: Same form layout, pre-populated with existing announcement data
- **Read-only**: List view only (no create/edit/delete buttons) when `site_settings_write` is not held

## Notes

### Specific (recent)
- 2026-05-25 — agent: implemented SystemAnnouncementsPage with full CRUD, banner preview, severity badges, and capability gating for gap-10b-4-sysannounce-admin-impl
