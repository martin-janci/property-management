# Org & Property

_Server: api-server. Modules: buildings.rs, my_units.rs, unit_residents.rs, agencies.rs, agency_provisioning.rs, tenant_config.rs, building_certifications.rs, organizations/ (core, members, roles, settings)._

Mount prefixes resolved from `lib.rs` / `main.rs`:
- buildings → `/api/v1/buildings`
- my_units → `/api/v1/users/me/units`
- unit_residents → nested in buildings at `/api/v1/buildings/{building_id}/units/{unit_id}/residents`
- agencies → `/api/v1/agencies`
- agency_provisioning → merged into `platform_admin::router()` → `/api/v1/platform-admin`
- tenant_config → `/tenant-config` (main.rs)
- building_certifications → `/api/v1/building-certifications`
- organizations → `/api/v1/organizations` (core/members/roles/settings all merged into one router)

No stub markers (`todo!`/`unimplemented!`/501/ROADMAP) found in any module — all handlers are real. Status differences below are driven by test coverage.

## buildings.rs  (mount: /api/v1/buildings)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/buildings | create_building | done | building_manager_rbac_tests.rs, endpoints_smoke_tests.rs | only FORBIDDEN/auth-rejection asserted, no happy-path |
| GET | /api/v1/buildings | list_buildings | done | building_manager_rbac_tests.rs | `list_allows_own_org` asserts 200 with organization_id filter |
| POST | /api/v1/buildings/bulk | bulk_import_buildings | done | building_manager_rbac_tests.rs | only FORBIDDEN asserted |
| GET | /api/v1/buildings/{id} | get_building | done | none | smoke only hits non-existent/auth paths |
| PUT | /api/v1/buildings/{id} | update_building | done | none | |
| DELETE | /api/v1/buildings/{id} | archive_building | done | building_manager_rbac_tests.rs | only FORBIDDEN asserted |
| POST | /api/v1/buildings/{id}/restore | restore_building | done | none | |
| GET | /api/v1/buildings/{id}/statistics | get_building_statistics | done | none | |
| GET | /api/v1/buildings/{id}/units | list_units | done | none | |
| POST | /api/v1/buildings/{id}/units | create_unit | done | none | |
| GET | /api/v1/buildings/{building_id}/units/{unit_id} | get_unit | done | none | |
| PUT | /api/v1/buildings/{building_id}/units/{unit_id} | update_unit | done | none | |
| DELETE | /api/v1/buildings/{building_id}/units/{unit_id} | archive_unit | done | none | |
| POST | /api/v1/buildings/{building_id}/units/{unit_id}/restore | restore_unit | done | none | |
| GET | /api/v1/buildings/{building_id}/units/{unit_id}/owners | list_unit_owners | done | none | |
| POST | /api/v1/buildings/{building_id}/units/{unit_id}/owners | assign_unit_owner | done | none | |
| PUT | /api/v1/buildings/{building_id}/units/{unit_id}/owners/{user_id} | update_unit_owner | done | none | |
| DELETE | /api/v1/buildings/{building_id}/units/{unit_id}/owners/{user_id} | remove_unit_owner | done | none | |

## my_units.rs  (mount: /api/v1/users/me/units)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/users/me/units | list_my_units | done | my_units_resident_view_tests.rs | multiple 200 happy-path assertions (resident/owner/PII-filter) + 401 |

## unit_residents.rs  (mount: /api/v1/buildings/{building_id}/units/{unit_id}/residents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | .../residents | list_residents | done | none | no test hits the residents subpath |
| POST | .../residents | add_resident | done | none | |
| GET | .../residents/{resident_id} | get_resident | done | none | |
| PUT | .../residents/{resident_id} | update_resident | done | none | |
| DELETE | .../residents/{resident_id} | remove_resident | done | none | |
| POST | .../residents/{resident_id}/end | end_residency | done | none | |
| GET | .../residents/history | list_resident_history | done | none | |

## agencies.rs  (mount: /api/v1/agencies)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/agencies | create_agency | done | none | |
| GET | /api/v1/agencies/{id} | get_agency | done | agency_authz_idor_tests.rs | authz-only (asserts != OK without auth) |
| PUT | /api/v1/agencies/{id} | update_agency | done | none | |
| PUT | /api/v1/agencies/{id}/branding | update_branding | done | none | |
| GET | /api/v1/agencies/{id}/members | list_members | done | agency_authz_idor_tests.rs | authz-only |
| POST | /api/v1/agencies/{id}/members/invite | invite_member | done | none | |
| PUT | /api/v1/agencies/{id}/members/{user_id}/role | update_member_role | done | none | |
| DELETE | /api/v1/agencies/{id}/members/{user_id} | remove_member | done | none | |
| POST | /api/v1/agencies/{id}/members/{user_id}/reassign/{to_user_id} | reassign_listings | done | none | |
| POST | /api/v1/agencies/invitations/accept | accept_invitation | done | none | |
| PUT | /api/v1/agencies/{id}/listings/{listing_id}/visibility | update_visibility | done | none | |
| GET | /api/v1/agencies/{id}/listings/{listing_id}/history | get_listing_history | done | none | |
| POST | /api/v1/agencies/{id}/import | create_import_job | done | agency_authz_idor_tests.rs | authz-only |
| GET | /api/v1/agencies/{id}/import/{job_id} | get_import_job | done | agency_authz_idor_tests.rs | authz-only |
| GET | /api/v1/agencies/{id}/import | list_import_jobs | done | agency_authz_idor_tests.rs | authz-only |

## agency_provisioning.rs  (mount: /api/v1/platform-admin, merged into platform_admin::router())
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/platform-admin/agencies | create_agency | partial | none | no test hits platform-admin/agencies |

## tenant_config.rs  (mount: /tenant-config)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /tenant-config | get_tenant_config | done | tenant_config_tests.rs | asserts 200 happy paths (default + configured tenant) |

## building_certifications.rs  (mount: /api/v1/building-certifications)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/building-certifications/dashboard | get_dashboard | partial | none | |
| GET | /api/v1/building-certifications | list_certifications | partial | none | |
| POST | /api/v1/building-certifications | create_certification | partial | none | |
| GET | /api/v1/building-certifications/expiring | get_expiring_certifications | partial | none | |
| GET | /api/v1/building-certifications/{cert_id} | get_certification | partial | none | |
| PUT | /api/v1/building-certifications/{cert_id} | update_certification | partial | none | |
| DELETE | /api/v1/building-certifications/{cert_id} | delete_certification | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/with-credits | get_certification_with_credits | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/credits | list_credits | partial | none | |
| POST | /api/v1/building-certifications/{cert_id}/credits | create_credit | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/credits/{credit_id} | get_credit | partial | none | |
| PUT | /api/v1/building-certifications/{cert_id}/credits/{credit_id} | update_credit | partial | none | |
| DELETE | /api/v1/building-certifications/{cert_id}/credits/{credit_id} | delete_credit | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/documents | list_documents | partial | none | |
| POST | /api/v1/building-certifications/{cert_id}/documents | create_document | partial | none | |
| DELETE | /api/v1/building-certifications/{cert_id}/documents/{doc_id} | delete_document | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/milestones | list_milestones | partial | none | |
| POST | /api/v1/building-certifications/{cert_id}/milestones | create_milestone | partial | none | |
| PUT | /api/v1/building-certifications/{cert_id}/milestones/{milestone_id} | update_milestone | partial | none | |
| DELETE | /api/v1/building-certifications/{cert_id}/milestones/{milestone_id} | delete_milestone | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/benchmarks | list_benchmarks | partial | none | |
| POST | /api/v1/building-certifications/{cert_id}/benchmarks | create_benchmark | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/costs | list_costs | partial | none | |
| POST | /api/v1/building-certifications/{cert_id}/costs | create_cost | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/costs/total | get_total_costs | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/reminders | list_reminders | partial | none | |
| POST | /api/v1/building-certifications/{cert_id}/reminders | create_reminder | partial | none | |
| GET | /api/v1/building-certifications/{cert_id}/audit-logs | list_audit_logs | partial | none | |

## organizations/ (core.rs, members.rs, roles.rs, settings.rs)  (mount: /api/v1/organizations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/organizations | create_organization | partial | endpoints_smoke_tests.rs | smoke asserts only auth-rejection statuses |
| GET | /api/v1/organizations | list_organizations | partial | endpoints_smoke_tests.rs | auth-rejection only |
| GET | /api/v1/organizations/my | list_my_organizations | partial | endpoints_smoke_tests.rs | auth-rejection only |
| GET | /api/v1/organizations/{id} | get_organization | partial | endpoints_smoke_tests.rs | auth-rejection only |
| PUT | /api/v1/organizations/{id} | update_organization | partial | none | |
| DELETE | /api/v1/organizations/{id} | delete_organization | partial | none | |
| GET | /api/v1/organizations/{id}/members | list_organization_members | partial | none | |
| POST | /api/v1/organizations/{id}/members | add_organization_member | partial | none | |
| PUT | /api/v1/organizations/{id}/members/{user_id} | update_organization_member | partial | none | |
| DELETE | /api/v1/organizations/{id}/members/{user_id} | remove_organization_member | partial | none | |
| GET | /api/v1/organizations/{id}/roles | list_organization_roles | partial | none | |
| POST | /api/v1/organizations/{id}/roles | create_organization_role | partial | none | |
| GET | /api/v1/organizations/{id}/roles/{role_id} | get_organization_role | partial | none | |
| PUT | /api/v1/organizations/{id}/roles/{role_id} | update_organization_role | partial | none | |
| DELETE | /api/v1/organizations/{id}/roles/{role_id} | delete_organization_role | partial | none | |
| GET | /api/v1/organizations/{id}/settings | get_organization_settings | partial | none | |
| PUT | /api/v1/organizations/{id}/settings | update_organization_settings | partial | none | |
| GET | /api/v1/organizations/{id}/branding | get_organization_branding | partial | none | |
| PUT | /api/v1/organizations/{id}/branding | update_organization_branding | partial | none | |
| GET | /api/v1/organizations/{id}/export | export_organization_data | partial | none | |
| GET | /api/v1/organizations/{id}/features | list_organization_features | partial | none | |
| PUT | /api/v1/organizations/{id}/features | bulk_update_organization_features | partial | none | |
| PUT | /api/v1/organizations/{id}/features/{key} | toggle_organization_feature | partial | none | |

## Summary
- done: 42 | partial: 52 | stub: 0 | missing: 0 | total: 94
