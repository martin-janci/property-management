---
id: ppt/admin-oauth-clients
name: OAuth Client Management (Admin)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: /identity/oauth-clients
    component: OAuthClientsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - list_oauth_clients
  - get_oauth_client
  - register_oauth_client
  - update_oauth_client
  - revoke_oauth_client
  - regenerate_oauth_client_secret
relatedScreens: []
sharedComponents:
  - DestructiveConfirmDialog
  - ScopeSelector
  - RedirectUriEditor
diagrams: []
useCases:
  - UC-14
epics:
  - Epic-10A
designSources: []
owner: pm-frontend
---

## Functionality Checklist

- [w] List all registered OAuth clients with name, clientId, scopes, status, and creation date
- [w] Register new OAuth client with name, description, redirect URIs, scope selector, confidential/public flag, token rotation option
- [w] Show new client secret exactly once after registration (copy-to-clipboard)
- [w] Edit client name, description, scopes, active status via PATCH
- [w] Revoke (deactivate) client with DestructiveConfirmDialog (type-name confirmation + 2s delay)
- [w] Regenerate client secret with confirmation dialog (secret shown once)
- [w] Sidebar nav link gated by `oauth_client_write` capability

## States

- **Empty**: Dashed-border placeholder with "Register the first client" link
- **Loading**: "Loading…" status message
- **Error**: Error message with Retry button
- **Revoked client**: Card shown with reduced opacity + REVOKED badge, regenerate/revoke actions disabled

## Notes

### Broader context

Part of Epic 10A-2 OAuth Authorization Server. Backend CRUD for OAuth clients
was fully implemented (register/list/get/update/revoke/regenerate_secret routes
under `/api/v1/admin/oauth/clients`). This screen provides the missing
admin UI to manage those clients.

### Specific (recent)

- 2026-05-24 — agent: gap-10a-2-oauth-admin-ui — built OAuthClientsPage in
  admin-web; added OAuth types/api/hooks to @ppt/api-client admin module;
  added oauth_client_write to admin-ui capabilities; sidebar link under
  DEVELOPER group; created oauth-grants stub module to fix missing import;
  typecheck + biome + vite build all green.

## Agent Log
- 2026-07-13 — agent: gap-screens-normalize-frontmatter — normalized story-id-style epic ref(s) Epic-10A-2 → Epic-10A (strip story suffix); /screens validate clean.

<!-- newest entries on top -->
- 2026-05-24 — agent: gap-10a-2-oauth-admin-ui — initial implementation; full
  CRUD UI (register, edit, revoke, regenerate secret); Band A + Band B verify
  gate passed.
