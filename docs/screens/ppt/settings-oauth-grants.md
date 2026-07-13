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
  - id: ppt/settings-two-factor
    rel: sibling
  - id: ppt/settings-profile
    rel: sibling
sharedComponents: []
diagrams: []
useCases:
  - UC-14
epics:
  - Epic-10A
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
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-10A-3 → Epic-10A (strip story suffix); /screens validate clean.
- 2026-05-24 — agent: gap-10a-3-user-grants-ui — built OAuthGrantsPage under features/oauth-grants/; added lazy route at /settings/oauth-grants; wired useUserGrants + useRevokeUserGrant; toast feedback on success/error; confirmation dialog before revoke.
