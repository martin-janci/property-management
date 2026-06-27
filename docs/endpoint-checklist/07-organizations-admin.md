# Organizations & Admin endpoints

Mount prefixes (from `src/lib.rs` `route_table()` unless noted):
`organizations` → `/api/v1/organizations`; `admin` → `/api/v1/admin`;
`admin_tenant_lifecycle` → `/api/v1/admin`; `agencies` → `/api/v1/agencies`;
`agency_provisioning` → merged into `platform_admin` → `/api/v1/platform-admin`.
`admin_tenants` (branding + feature-flags) is mounted ONLY in `main.rs::serve`,
NOT in `route_table()`, so integration tests cannot reach it → `partial`.

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `POST /api/v1/organizations` | `organizations/core.rs:create_organization` | done | `endpoints_smoke_tests.rs` | auth-protection asserted |
| `GET /api/v1/organizations` | `organizations/core.rs:list_organizations` | done | `endpoints_smoke_tests.rs` | auth-protection asserted |
| `GET /api/v1/organizations/my` | `organizations/core.rs:list_my_organizations` | done | `endpoints_smoke_tests.rs` | auth-protection asserted |
| `GET /api/v1/organizations/{id}` | `organizations/core.rs:get_organization` | done | `endpoints_smoke_tests.rs` | invalid-uuid/auth asserted |
| `PUT /api/v1/organizations/{id}` | `organizations/core.rs:update_organization` | partial | — | real handler, no test |
| `DELETE /api/v1/organizations/{id}` | `organizations/core.rs:delete_organization` | partial | — | real handler, no test |
| `GET /api/v1/organizations/{id}/members` | `organizations/members.rs:list_organization_members` | partial | — | real handler, no test |
| `POST /api/v1/organizations/{id}/members` | `organizations/members.rs:add_organization_member` | partial | — | real handler, no test |
| `PUT /api/v1/organizations/{id}/members/{user_id}` | `organizations/members.rs:update_organization_member` | partial | — | real handler, no test |
| `DELETE /api/v1/organizations/{id}/members/{user_id}` | `organizations/members.rs:remove_organization_member` | partial | — | real handler, no test |
| `GET /api/v1/organizations/{id}/roles` | `organizations/roles.rs:list_organization_roles` | partial | — | real handler, no test |
| `POST /api/v1/organizations/{id}/roles` | `organizations/roles.rs:create_organization_role` | partial | — | real handler, no test |
| `GET /api/v1/organizations/{id}/roles/{role_id}` | `organizations/roles.rs:get_organization_role` | partial | — | real handler, no test |
| `PUT /api/v1/organizations/{id}/roles/{role_id}` | `organizations/roles.rs:update_organization_role` | partial | — | real handler, no test |
| `DELETE /api/v1/organizations/{id}/roles/{role_id}` | `organizations/roles.rs:delete_organization_role` | partial | — | real handler, no test |
| `GET /api/v1/organizations/{id}/settings` | `organizations/settings.rs:get_organization_settings` | partial | — | real handler, no test |
| `PUT /api/v1/organizations/{id}/settings` | `organizations/settings.rs:update_organization_settings` | partial | — | real handler, no test |
| `GET /api/v1/organizations/{id}/branding` | `organizations/settings.rs:get_organization_branding` | partial | — | real handler, no test |
| `PUT /api/v1/organizations/{id}/branding` | `organizations/settings.rs:update_organization_branding` | partial | — | real handler, no test |
| `GET /api/v1/organizations/{id}/export` | `organizations/settings.rs:export_organization_data` | partial | — | real handler, no test |
| `GET /api/v1/organizations/{id}/features` | `organizations/settings.rs:list_organization_features` | partial | — | real handler, no test |
| `PUT /api/v1/organizations/{id}/features` | `organizations/settings.rs:bulk_update_organization_features` | partial | — | real handler, no test |
| `PUT /api/v1/organizations/{id}/features/{key}` | `organizations/settings.rs:toggle_organization_feature` | partial | — | real handler, no test |
| `GET /api/v1/admin/users` | `admin/users_lifecycle.rs:list_users` | partial | — | capability-gated, no test |
| `GET /api/v1/admin/users/{id}` | `admin/users_lifecycle.rs:get_user` | partial | — | capability-gated, no test |
| `POST /api/v1/admin/users/{id}/suspend` | `admin/users_lifecycle.rs:suspend_user` | partial | — | capability-gated, no test |
| `POST /api/v1/admin/users/{id}/reactivate` | `admin/users_lifecycle.rs:reactivate_user` | partial | — | capability-gated, no test |
| `POST /api/v1/admin/users/{id}/delete` | `admin/users_lifecycle.rs:delete_user` | partial | — | capability-gated, no test |
| `POST /api/v1/admin/memberships/invite` | `admin/memberships.rs:invite` | partial | — | capability-gated, no test |
| `POST /api/v1/admin/memberships/accept` | `admin/memberships.rs:accept` | partial | — | invitee-gated, no test |
| `DELETE /api/v1/admin/memberships/{user_id}` | `admin/memberships.rs:revoke` | partial | — | capability-gated, no test |
| `GET /api/v1/admin/memberships/merge-collisions` | `admin/memberships.rs:merge_collisions` | partial | — | capability-gated, no test |
| `GET /api/v1/admin/agencies` | `admin/agencies.rs:list_agencies` | partial | — | AgenciesRead-gated, no test |
| `GET /api/v1/admin/agencies/{id}` | `admin/agencies.rs:get_agency` | partial | — | AgenciesRead-gated, no test |
| `POST /api/v1/admin/agencies/{id}/suspend` | `admin/agencies.rs:suspend_agency` | partial | — | AgenciesSuspend-gated, no test |
| `POST /api/v1/admin/agencies/{id}/domains` | `admin/agencies.rs:add_domain` | partial | — | AgenciesWrite-gated, no test |
| `GET /api/v1/admin/principals` | `admin/users.rs:search_users` | partial | — | UsersRead-gated, no test |
| `GET /api/v1/admin/principals/{id}` | `admin/users.rs:get_user` | partial | — | UsersRead-gated, no test |
| `POST /api/v1/admin/principals/{id}/principal-kind` | `admin/users.rs:set_principal_kind` | partial | — | escalate-gated, real DB call |
| `GET /api/v1/admin/audit/csv` | `admin/audit.rs:export_csv` | partial | — | AuditRead-gated, no test |
| `GET /api/v1/admin/audit/` | `admin/audit.rs:list_audit_events` | partial | — | AuditRead-gated, no test |
| `GET /api/v1/admin/capabilities/registry` | `admin/capabilities.rs:list_registry` | partial | — | AuditRead-gated, no test |
| `GET /api/v1/admin/capabilities/me` | `admin/capabilities.rs:list_for_me` | partial | — | principal-only, no test |
| `GET /api/v1/admin/capabilities/users/{user_id}` | `admin/capabilities.rs:list_for_user` | partial | — | AuditRead-gated, no test |
| `POST /api/v1/admin/capabilities/users/{user_id}/grant` | `admin/capabilities.rs:grant_capability` | partial | — | MembershipsGrant-gated |
| `DELETE /api/v1/admin/capabilities/users/{user_id}/grant/{grant_id}` | `admin/capabilities.rs:revoke_capability` | partial | — | MembershipsRevoke-gated |
| `GET /api/v1/admin/impersonation/active` | `admin/impersonation.rs:list_active` | partial | — | UsersImpersonate-gated |
| `POST /api/v1/admin/impersonation/start` | `admin/impersonation.rs:start` | partial | — | UsersImpersonate-gated |
| `DELETE /api/v1/admin/impersonation/{token_id}` | `admin/impersonation.rs:stop` | partial | — | UsersImpersonate-gated |
| `GET /api/v1/admin/metrics/summary` | `admin/metrics.rs:metrics_summary` | partial | — | AuditRead-gated, no test |
| `GET /api/v1/admin/notifications/analytics` | `admin/notifications.rs:get_analytics` | partial | — | AuditRead-gated, no test |
| `POST /api/v1/admin/tenants/{id}/export` | `admin_tenant_lifecycle.rs:export_handler` | partial | — | TenantExport-gated, no test |
| `POST /api/v1/admin/tenants/{id}/purge` | `admin_tenant_lifecycle.rs:purge_handler` | partial | — | TenantPurge-gated, no test |
| `POST /api/v1/admin/tenants/restore` | `admin_tenant_lifecycle.rs:restore_handler` | partial | — | TenantRestore-gated, no test |
| `GET /admin/tenants/{org_id}/branding` | `admin_tenants.rs:get_tenant_branding` | partial | — | main.rs-only mount; untestable in route_table |
| `PUT /admin/tenants/{org_id}/branding` | `admin_tenants.rs:update_tenant_branding` | partial | — | main.rs-only mount; untestable in route_table |
| `GET /admin/tenants/{org_id}/feature-flags` | `admin_tenants.rs:list_tenant_feature_flags` | partial | — | main.rs-only mount; untestable in route_table |
| `PUT /admin/tenants/{org_id}/feature-flags` | `admin_tenants.rs:upsert_tenant_feature_flag` | partial | — | main.rs-only mount; untestable in route_table |
| `POST /api/v1/agencies` | `agencies.rs:create_agency` | partial | — | real handler, no test |
| `GET /api/v1/agencies/{id}` | `agencies.rs:get_agency` | done | `agency_authz_idor_tests.rs` | anon-rejection asserted |
| `PUT /api/v1/agencies/{id}` | `agencies.rs:update_agency` | partial | — | real handler, no test |
| `PUT /api/v1/agencies/{id}/branding` | `agencies.rs:update_branding` | partial | — | real handler, no test |
| `GET /api/v1/agencies/{id}/members` | `agencies.rs:list_members` | done | `agency_authz_idor_tests.rs` | anon-rejection asserted |
| `POST /api/v1/agencies/{id}/members/invite` | `agencies.rs:invite_member` | partial | — | real handler, no test |
| `PUT /api/v1/agencies/{id}/members/{user_id}/role` | `agencies.rs:update_member_role` | partial | — | real handler, no test |
| `DELETE /api/v1/agencies/{id}/members/{user_id}` | `agencies.rs:remove_member` | partial | — | real handler, no test |
| `POST /api/v1/agencies/{id}/members/{user_id}/reassign/{to_user_id}` | `agencies.rs:reassign_listings` | partial | — | real handler, no test |
| `POST /api/v1/agencies/invitations/accept` | `agencies.rs:accept_invitation` | partial | — | real handler, no test |
| `PUT /api/v1/agencies/{id}/listings/{listing_id}/visibility` | `agencies.rs:update_visibility` | partial | — | real handler, no test |
| `GET /api/v1/agencies/{id}/listings/{listing_id}/history` | `agencies.rs:get_listing_history` | partial | — | real handler, no test |
| `POST /api/v1/agencies/{id}/import` | `agencies.rs:create_import_job` | done | `agency_authz_idor_tests.rs` | anon-rejection asserted |
| `GET /api/v1/agencies/{id}/import/{job_id}` | `agencies.rs:get_import_job` | done | `agency_authz_idor_tests.rs` | anon-rejection asserted |
| `GET /api/v1/agencies/{id}/import` | `agencies.rs:list_import_jobs` | done | `agency_authz_idor_tests.rs` | anon-rejection asserted |
| `POST /api/v1/platform-admin/agencies` | `agency_provisioning.rs:create_agency` | partial | — | super-admin-gated, only unit tests |

## Tally
done: 9  partial: 65  stub: 0  missing: 0  total: 74
