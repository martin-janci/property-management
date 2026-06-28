---
id: ppt/admin-organizations
name: Organization Management Dashboard (Admin)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: platform/organizations
    component: OrganizationsPage
    buildStatus: planned
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - list_organizations
  - get_organization
  - suspend_organization
  - reactivate_organization
  - get_platform_stats
relatedScreens:
  - id: ppt/admin-platform-health
    rel: sibling
  - id: ppt/admin-system-announcements
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-27
epics:
  - Epic-10B-1
designSources: []
owner: pm-frontend
---

## Functionality Checklist

- [ ] Organizations list view: name, member count, building count, created date, status badge (active / suspended / pending)
- [ ] Pagination, status filter, and search-by-name/slug on the list
- [ ] Platform stats summary header (total orgs, total members, usage) from `get_platform_stats`
- [ ] Drill into organization details: members, buildings, usage metrics, billing status
- [ ] Suspend organization action with reason prompt (cascade-invalidates all org sessions)
- [ ] Reactivate suspended organization action
- [ ] Suspend / reactivate gated by the super-admin platform capability
- [ ] Error banner if list / details load fails

## States

- **Loading**: list and details skeletons while fetching
- **Error**: red alert banner on failed load
- **Empty**: "No organizations." message when the platform has none
- **Suspended row**: visually flagged with suspension reason + suspended-by metadata

## Notes

### Broader context

Part of Epic 10B Story 10B.1 (super-admin operator view of all organizations).
Backend is complete: routes live under `/api/v1/platform-admin/organizations/*`
plus `/api/v1/platform-admin/stats`
(`backend/servers/api-server/src/routes/platform_admin/tenants.rs`), backed by
the `organizations` table + `organization_metrics` view from migration
`00029_create_platform_admin.sql`. Sibling admin screens (Platform Health 10B-3,
System Announcements 10B-4) follow the same `@ppt/api-client` admin-module
pattern with the shared MFA-aware `authenticatedFetchJson`.

### Specific (recent)

- 2026-06-28 — Screen-map created to close the Epic-10B-1 orphan-epic coverage
  gap. Backend shipped; the admin-web `OrganizationsPage` (route
  `platform/organizations`) is not yet built — `buildStatus: planned`. The five
  platform-admin org endpoints were also registered in `@ppt/sitemap`
  (`api-server` data) so this screen-map's `endpoints` validate.

## Agent Log

<!-- newest entries on top -->
- 2026-06-28 — agent: 10b-1-organization-management-dashboard — created
  screen-map for Organization Management Dashboard (Epic-10B-1); registered the
  five `platform-admin/organizations*` + `stats` endpoints in `@ppt/sitemap`
  api-server data so endpoint refs resolve. Frontend page still TODO
  (buildStatus: planned, apiStatus: complete).
