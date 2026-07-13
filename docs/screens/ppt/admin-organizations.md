---
id: ppt/admin-organizations
name: Organization Management Dashboard (Admin)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: platform/organizations
    component: OrganizationsPage
    buildStatus: shipped
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
  - Epic-10B
designSources: []
owner: pm-frontend
---

## Functionality Checklist

- [x] Organizations list view: name, member count, building count, created date, status badge (active / suspended / pending)
- [x] Pagination, status filter, and search-by-name/slug on the list
- [x] Platform stats summary header (total orgs, total members, usage) from `get_platform_stats`
- [x] Drill into organization details: members, buildings, usage metrics (billing status not yet exposed by the API)
- [x] Suspend organization action with reason prompt (cascade-invalidates all org sessions)
- [x] Reactivate suspended organization action
- [x] Suspend / reactivate gated by the `agencies_suspend` capability
- [x] Error banner if list / details load fails

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

- 2026-07-07 — `OrganizationsPage` shipped in admin-web (route
  `platform/organizations`, sidebar PLATFORM group, gated by `agencies_read`;
  suspend/reactivate buttons gated by `agencies_suspend`). Detail drill-in is a
  modal dialog (no `:id` sub-route). Billing status is not shown — the backend
  detail DTO doesn't expose it yet.
- 2026-06-28 — Screen-map created to close the Epic-10B-1 orphan-epic coverage
  gap. Backend shipped; the admin-web `OrganizationsPage` (route
  `platform/organizations`) is not yet built — `buildStatus: planned`. The five
  platform-admin org endpoints were also registered in `@ppt/sitemap`
  (`api-server` data) so this screen-map's `endpoints` validate.

## Agent Log
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-10B-1 → Epic-10B (strip story suffix); /screens validate clean.

<!-- newest entries on top -->
- 2026-07-07 — agent: gap-10b-1 — built admin-web `OrganizationsPage`
  (list + pagination + status filter + search, platform-stats cards,
  suspend/reactivate confirmation dialogs, detail drill-in modal); added
  platform-admin org endpoints to `@ppt/api-client` admin module (types, api,
  hooks); i18n en/sk/cs; page tests. buildStatus planned → shipped.
- 2026-06-28 — agent: 10b-1-organization-management-dashboard — created
  screen-map for Organization Management Dashboard (Epic-10B-1); registered the
  five `platform-admin/organizations*` + `stats` endpoints in `@ppt/sitemap`
  api-server data so endpoint refs resolve. Frontend page still TODO
  (buildStatus: planned, apiStatus: complete).
