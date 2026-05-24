---
id: ppt/settings-oauth-grants
name: OAuth Authorized Applications
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: "/settings/oauth-grants"
    component: OAuthGrantsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - list_user_grants
  - revoke_user_grant
relatedScreens:
  - ppt/settings-two-factor
  - ppt/settings-profile
sharedComponents: []
diagrams: []
useCases:
  - UC-14
epics:
  - Epic-10A-3
designSources: []
owner: pm-frontend
---

# OAuth Authorized Applications

Allows the authenticated user to list all third-party applications that have
been granted OAuth access, and to revoke any grant individually.

Wired to live backend endpoints in Epic 10A, Story 10A-3:

- `GET  /api/v1/oauth/grants`             — `useUserGrants()` hook
- `DELETE /api/v1/oauth/grants/{clientId}` — `useRevokeUserGrant()` mutation

## Notes

### Specific (recent)
- 2026-05-24 — agent: OAuthGrantsPage created; full list + revoke with confirmation dialog wired to @ppt/api-client oauth-grants hooks; apiStatus complete.

## Agent Log
- 2026-05-24 — agent: gap-10a-3-user-grants-ui — built OAuthGrantsPage under features/oauth-grants/; added lazy route at /settings/oauth-grants; wired useUserGrants + useRevokeUserGrant; toast feedback on success/error; confirmation dialog before revoke.
