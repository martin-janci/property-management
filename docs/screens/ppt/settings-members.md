---
id: ppt/settings-members
name: People & Roles
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/settings/members"
    component: OrganizationMembersPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints: []
relatedScreens: []
sharedComponents: []
diagrams: []
useCases: []
epics:
  - Epic-1
designSources: []
owner: pm-frontend
---

# People & Roles

Manager-only list of organization members: add an existing account by e-mail with a role, change role, remove. Backed by /api/v1/organizations/{id}/members and /roles.

## Notes

### Specific (recent)
- 2026-11-06 — added in gap audit round 6 (P0).

## Agent Log
- 2026-11-06 — agent: created with the page.
