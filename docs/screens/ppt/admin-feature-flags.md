---
id: ppt/admin-feature-flags
name: Feature Flag Management (Admin)
product: ppt
sitemapRefs: {}
implementations:
  ppt-web:
    route: /ops/feature-flags
    component: FeatureFlagsPage
    buildStatus: shipped
    redesignStatus: not-started
    apiStatus: complete
endpoints:
  - list_feature_flags
  - create_feature_flag
  - get_feature_flag
  - update_feature_flag
  - delete_feature_flag
  - toggle_feature_flag
  - create_feature_flag_override
  - delete_feature_flag_override
  - get_resolved_feature_flags
relatedScreens: []
sharedComponents:
  - SettingsForm
  - AuditReasonPrompt
  - HelpTooltip
useCases: []
epics:
  - Epic-10B-2
designSources: []
owner: pm-frontend
---

## Functionality Checklist

Tag each item with the platforms it applies to: `[w]`, `[m]`, `[w,m]`, or `[-]`.

- [w] List all platform feature flags (key, name, enabled state) loaded from `GET /api/v1/platform-admin/feature-flags`
- [w] Toggle a flag on/off; on submit, open AuditReasonPrompt (minLength=20) and fire `POST /api/v1/platform-admin/feature-flags/{id}/toggle` per toggled flag (audit-logged)
- [w] Create a new flag via `POST /api/v1/platform-admin/feature-flags` when a referenced key is not yet registered on the server
- [w] Inspect a single flag with its per-scope overrides via `GET /api/v1/platform-admin/feature-flags/{id}`
- [w] Add a per-scope override (organization / user / role) via `POST /api/v1/platform-admin/feature-flags/{id}/overrides`
- [w] Remove a per-scope override via `DELETE /api/v1/platform-admin/feature-flags/{id}/overrides/{override_id}`
- [w] Resolve effective flags for the current caller via `GET /api/v1/feature-flags` (org/user/role overrides applied)
- [w] Entire screen + sidebar nav link gated by the `feature_flags_write` capability (ProtectedRoute under OPS group)

## States

- **Empty**: No flags registered on the server — referenced keys can be created via POST.
- **Loading**: "Loading…" status while the flag list query is in flight.
- **Error**: Error message with retry; unknown keys surface a toast hinting to create them via `POST /platform-admin/feature-flags` first.
- **Toggle-not-implemented**: If the toggle endpoint returns 404 for a flag, a "Save not yet implemented" toast is shown for that flag.

## Notes

### Broader context

Epic 10B.2 — Feature Flag Management. The backend ships full CRUD plus
toggle, per-scope overrides (organization/user/role) and a resolved-flags
endpoint under `backend/servers/api-server/src/routes/platform_admin/features.rs`
(repositories: `feature_flag.rs`, `tenant_feature_flag.rs`, `feature_analytics.rs`).
The admin-web page provides the write-only management console; toggles are
audit-logged behind an AuditReasonPrompt (≥20-char reason).

### Specific (recent)

- 2026-06-28 — agent: 10b-2-feature-flag-management — coverage-gap closure:
  Epic 10B.2 was an orphan epic (shipped backend + admin-web page, but no
  screen-map). Added this screen-map and registered the 9 platform-admin
  feature-flag endpoints in `@ppt/sitemap` (`data/api-server/index.ts`) so the
  screen-map's `endpoints` references validate.

## Agent Log

<!-- newest entries on top -->
- 2026-06-28 — agent: 10b-2-feature-flag-management — created screen-map for
  the shipped admin-web Feature Flag Management screen (route `/ops/feature-flags`,
  `FeatureFlagsPage`, `feature_flags_write`-gated); added the 9 platform-admin
  feature-flag endpoints to `@ppt/sitemap`. `/screens validate` clean.
