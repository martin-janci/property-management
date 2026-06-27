# Admin & Platform

_Server: api-server. Modules: admin_tenants, admin_tenant_lifecycle, infrastructure, operations, health, caddy_ask, admin/ (agencies, audit, capabilities, impersonation, memberships, metrics, mfa, notifications, users, users_lifecycle), platform_admin/ (tenants, features, ops, audit)._

Mount prefixes resolved from `lib.rs` + `main.rs`:
- `health::liveness` → `/health`, `health::readiness` → `/readiness` (root, lib.rs)
- `admin::router()` → `/api/v1/admin` (lib.rs)
- `admin_tenant_lifecycle::router()` → `/api/v1/admin` (lib.rs — DUPLICATE mount with admin::router; paths are disjoint: lifecycle owns `/tenants/...`)
- `admin_tenants::branding_router()` → `/admin/tenants/{org_id}/branding` (main.rs, production-only)
- `admin_tenants::feature_flags_router()` → `/admin/tenants/{org_id}/feature-flags` (main.rs, production-only)
- `caddy_ask::router()` → `/internal/caddy-ask` (main.rs, production-only, pre-TLS)
- `platform_admin::router()` → `/api/v1/platform-admin` (lib.rs)
- `platform_admin::public_feature_flags_router()` → `/api/v1/feature-flags` (lib.rs)
- `platform_admin::public_announcements_router()` → `/api/v1/system-announcements` (lib.rs)
- `platform_admin::public_maintenance_router()` → `/api/v1/maintenance` (lib.rs)
- `infrastructure::router()` → `/api/v1/infrastructure` (lib.rs)
- `operations::router()` → `/api/v1/operations` (lib.rs)

## health.rs  (mount: / )
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /health | liveness | done | integration/health_tests.rs | happy-path 200 + format/version/uptime/idempotent |
| GET | /readiness | readiness | done | integration/health_tests.rs | happy-path 200, DB dep, redis-degraded-stays-200 |

## caddy_ask.rs  (mount: /internal/caddy-ask)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /internal/caddy-ask | caddy_ask | partial | caddy_ask_tests.rs (indirect) | test REIMPLEMENTS the handler in a `mod test_handler` and mounts that copy; real `routes::caddy_ask::caddy_ask` is never exercised. Only `#[cfg(test)]` unit tests cover helpers (normalize/constant_time_eq/rate_limiter). No integration test hits the real handler |

## admin_tenants.rs  (mount: /admin/tenants/{org_id}/branding | /admin/tenants/{org_id}/feature-flags)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /admin/tenants/{org_id}/branding | get_tenant_branding | partial | none | real handler (AgencyBrandingRepository); no test hits path |
| PUT | /admin/tenants/{org_id}/branding | update_tenant_branding | partial | none | real upsert + css sanitizer (sanitizer has unit tests, handler path untested) |
| GET | /admin/tenants/{org_id}/feature-flags | list_tenant_feature_flags | partial | none | real handler; no path test |
| PUT | /admin/tenants/{org_id}/feature-flags | upsert_tenant_feature_flag | partial | none | real handler; no path test |

## admin_tenant_lifecycle.rs  (mount: /api/v1/admin)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/admin/tenants/{id}/export | export_handler | done | none | real (tenant_ops::export_tenant); no test |
| POST | /api/v1/admin/tenants/{id}/purge | purge_handler | done | none | real (tenant_ops::purge_tenant); no test |
| POST | /api/v1/admin/tenants/restore | restore_handler | done | none | real (multipart + restore_tenant_export); no test |

## admin/agencies.rs  (mount: /api/v1/admin/agencies)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/admin/agencies | list_agencies | partial | none | real (platform_admin_repo); no path test |
| GET | /api/v1/admin/agencies/{id} | get_agency | partial | none | real; no path test |
| POST | /api/v1/admin/agencies/{id}/suspend | suspend_agency | partial | none | real; no path test |
| POST | /api/v1/admin/agencies/{id}/domains | add_domain | partial | none | real; no path test |

## admin/audit.rs  (mount: /api/v1/admin/audit)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/admin/audit/csv | export_csv | partial | none | real (fetch_rows + CSV); no path test |
| GET | /api/v1/admin/audit | list_audit_events | partial | none | real (fetch_rows); no path test |

## admin/capabilities.rs  (mount: /api/v1/admin/capabilities)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/admin/capabilities/registry | list_registry | partial | none | real (sqlx runtime query); no path test |
| GET | /api/v1/admin/capabilities/me | list_for_me | partial | none | real; gated only by RequestPrincipal (bootstrap); no path test |
| GET | /api/v1/admin/capabilities/users/{user_id} | list_for_user | partial | none | real; no path test |
| POST | /api/v1/admin/capabilities/users/{user_id}/grant | grant_capability | partial | none | real; no path test |
| DELETE | /api/v1/admin/capabilities/users/{user_id}/grant/{grant_id} | revoke_capability | partial | none | real; no path test |

## admin/impersonation.rs  (mount: /api/v1/admin/impersonation)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/admin/impersonation/active | list_active | partial | none | real (ImpersonationService); no path test |
| POST | /api/v1/admin/impersonation/start | start | partial | none | real; no path test |
| DELETE | /api/v1/admin/impersonation/{token_id} | stop | partial | none | real; no path test |

## admin/memberships.rs  (mount: /api/v1/admin)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/admin/memberships/invite | invite | partial | none | real (AuthPolicyEnforcer + UserRepository); no path test |
| POST | /api/v1/admin/memberships/accept | accept | partial | none | real; gated by principal identity binding (no capability); no path test |
| DELETE | /api/v1/admin/memberships/{user_id} | revoke | partial | none | real; no path test |
| GET | /api/v1/admin/memberships/merge-collisions | merge_collisions | partial | none | real; no path test |

## admin/metrics.rs  (mount: /api/v1/admin/metrics)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/admin/metrics/summary | metrics_summary | partial | none | real (sqlx count queries); no path test |

## admin/notifications.rs  (mount: /api/v1/admin/notifications)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/admin/notifications/analytics | get_analytics | done | none | real (NotificationEventRepository); no path test |

## admin/users.rs  (mount: /api/v1/admin/principals)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/admin/principals | search_users | partial | none | real (platform_admin_repo); no path test |
| GET | /api/v1/admin/principals/{id} | get_user | partial | none | real; no path test |
| POST | /api/v1/admin/principals/{id}/principal-kind | set_principal_kind | partial | none | real; no path test |

## admin/users_lifecycle.rs  (mount: /api/v1/admin)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/admin/users | list_users | done | none | real (user_repo); no path test |
| GET | /api/v1/admin/users/{id} | get_user | done | none | real; no path test |
| POST | /api/v1/admin/users/{id}/suspend | suspend_user | done | none | real; no path test |
| POST | /api/v1/admin/users/{id}/reactivate | reactivate_user | done | none | real; no path test |
| POST | /api/v1/admin/users/{id}/delete | delete_user | done | none | real; no path test |

## admin/mfa/mod.rs  (mount: /api/v1/admin/mfa)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/admin/mfa/enroll/start | start_enroll | done | none | real handler; no test hits enroll/* |
| POST | /api/v1/admin/mfa/enroll/verify | verify_enroll | done | none | real handler; no test hits enroll/* |
| POST | /api/v1/admin/mfa/verify | verify_step_up | done | admin_mfa_step_up_tests.rs | happy-path 200 step-up |
| POST | /api/v1/admin/mfa/recovery/use | use_recovery | done | admin_mfa_recovery_tests.rs, mfa_recovery_cross_user_idor_tests.rs | happy-path 200 (first-use) |
| POST | /api/v1/admin/mfa/disable | disable_mfa | done | admin_mfa_disable_tests.rs, mfa_disable_rls_scope_tests.rs | happy-path 200 |

## infrastructure.rs  (mount: /api/v1/infrastructure)
All handlers are real (state.infrastructure_repo / background_job_repo / feature_flag / health-monitoring repos). The ONLY test (`infra_migration_platform_admin_tests.rs`) is authz-only (asserts 401 unauth + 403 non-admin) on a representative slice and NEVER exercises any success path → every endpoint is `partial`.
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/infrastructure/dashboard | get_dashboard | done | infra_migration_platform_admin_tests.rs (authz-only) | 401/403 only |
| GET | /api/v1/infrastructure/traces | list_traces | done | infra_migration_platform_admin_tests.rs (authz-only) | 401/403 only |
| GET | /api/v1/infrastructure/traces/{trace_id} | get_trace | done | infra_migration_platform_admin_tests.rs (authz-only) | 401/403 only |
| GET | /api/v1/infrastructure/traces/{trace_id}/spans | get_trace_spans | done | none | |
| GET | /api/v1/infrastructure/feature-flags | list_feature_flags | done | infra_migration_platform_admin_tests.rs (authz-only) | 401/403 only |
| POST | /api/v1/infrastructure/feature-flags | create_feature_flag | done | infra_migration_platform_admin_tests.rs (authz-only) | 401/403 only |
| GET | /api/v1/infrastructure/feature-flags/{id} | get_feature_flag | done | none | |
| PUT | /api/v1/infrastructure/feature-flags/{id} | update_feature_flag | done | none | |
| DELETE | /api/v1/infrastructure/feature-flags/{id} | delete_feature_flag | done | none | |
| POST | /api/v1/infrastructure/feature-flags/{id}/toggle | toggle_feature_flag | done | none | |
| GET | /api/v1/infrastructure/feature-flags/{id}/overrides | list_flag_overrides | done | none | |
| POST | /api/v1/infrastructure/feature-flags/{id}/overrides | create_flag_override | done | none | |
| DELETE | /api/v1/infrastructure/feature-flags/{id}/overrides/{override_id} | delete_flag_override | done | none | |
| GET | /api/v1/infrastructure/feature-flags/{id}/audit-log | get_flag_audit_log | done | none | |
| POST | /api/v1/infrastructure/feature-flags/evaluate | evaluate_feature_flag | done | none | |
| GET | /api/v1/infrastructure/jobs | list_jobs | done | infra_migration_platform_admin_tests.rs (authz-only) | 401/403 only |
| POST | /api/v1/infrastructure/jobs | create_job | done | none | |
| GET | /api/v1/infrastructure/jobs/{id} | get_job | done | none | |
| POST | /api/v1/infrastructure/jobs/{id}/retry | retry_job | done | none | |
| POST | /api/v1/infrastructure/jobs/{id}/cancel | cancel_job | done | none | |
| GET | /api/v1/infrastructure/jobs/{id}/executions | get_job_executions | done | none | |
| GET | /api/v1/infrastructure/jobs/queues/stats | get_queue_stats | done | none | |
| GET | /api/v1/infrastructure/jobs/types/stats | get_job_type_stats | done | none | |
| GET | /api/v1/infrastructure/health/detailed | get_detailed_health | done | infra_migration_platform_admin_tests.rs (authz-only) | 401/403 only |
| GET | /api/v1/infrastructure/health/checks | list_health_checks | done | none | |
| GET | /api/v1/infrastructure/health/checks/{id} | get_health_check | done | none | |
| GET | /api/v1/infrastructure/health/checks/{id}/results | get_health_check_results | done | none | |
| GET | /api/v1/infrastructure/health/alerts | list_alerts | done | infra_migration_platform_admin_tests.rs (authz-only) | 401/403 only |
| GET | /api/v1/infrastructure/health/alerts/{id} | get_alert | done | none | |
| POST | /api/v1/infrastructure/health/alerts/{id}/acknowledge | acknowledge_alert | done | none | |
| POST | /api/v1/infrastructure/health/alerts/{id}/resolve | resolve_alert | done | none | |
| GET | /api/v1/infrastructure/health/alert-rules | list_alert_rules | done | none | |
| POST | /api/v1/infrastructure/health/alert-rules | create_alert_rule | done | none | |
| GET | /api/v1/infrastructure/health/alert-rules/{id} | get_alert_rule | done | none | |
| PUT | /api/v1/infrastructure/health/alert-rules/{id} | update_alert_rule | done | none | |
| DELETE | /api/v1/infrastructure/health/alert-rules/{id} | delete_alert_rule | done | none | |
| POST | /api/v1/infrastructure/health/alert-rules/{id}/toggle | toggle_alert_rule | done | none | |
| GET | /api/v1/infrastructure/health/metrics | get_prometheus_metrics | done | none | Prometheus scrape; per test docstring this is the one infra endpoint intentionally not platform-admin-gated |

## operations.rs  (mount: /api/v1/operations)
All handlers are real (deployment / migration-safety / DR / cost-monitoring repos). No test references `/api/v1/operations` at all → every endpoint is `partial`.
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/operations/deployments | list_deployments | done | none | |
| POST | /api/v1/operations/deployments | create_deployment | done | none | |
| GET | /api/v1/operations/deployments/dashboard | get_deployment_dashboard | done | none | |
| GET | /api/v1/operations/deployments/{id} | get_deployment | done | none | |
| PUT | /api/v1/operations/deployments/{id}/status | update_deployment_status | done | none | |
| POST | /api/v1/operations/deployments/{id}/switch | switch_traffic | done | none | |
| POST | /api/v1/operations/deployments/{id}/rollback | rollback_deployment | done | none | |
| GET | /api/v1/operations/deployments/{id}/health-checks | list_deployment_health_checks | done | none | |
| POST | /api/v1/operations/deployments/{id}/health-checks | run_health_checks | done | none | |
| GET | /api/v1/operations/migrations | list_migrations | done | none | |
| POST | /api/v1/operations/migrations | create_migration | done | none | |
| GET | /api/v1/operations/migrations/{id} | get_migration | done | none | |
| PUT | /api/v1/operations/migrations/{id}/progress | update_migration_progress | done | none | |
| GET | /api/v1/operations/migrations/{id}/logs | list_migration_logs | done | none | |
| POST | /api/v1/operations/migrations/{id}/rollback | rollback_migration | done | none | |
| GET | /api/v1/operations/migrations/{id}/safety-check | check_migration_safety | done | none | |
| GET | /api/v1/operations/schema/versions | list_schema_versions | done | none | |
| GET | /api/v1/operations/schema/current | get_current_schema_version | done | none | |
| GET | /api/v1/operations/backups | list_backups | done | none | |
| POST | /api/v1/operations/backups | create_backup | done | none | |
| GET | /api/v1/operations/backups/dashboard | get_dr_dashboard | done | none | |
| GET | /api/v1/operations/backups/{id} | get_backup | done | none | |
| POST | /api/v1/operations/backups/{id}/verify | verify_backup | done | none | |
| POST | /api/v1/operations/recovery | initiate_recovery | done | none | |
| GET | /api/v1/operations/recovery/{id} | get_recovery_status | done | none | |
| GET | /api/v1/operations/dr/drills | list_dr_drills | done | none | |
| POST | /api/v1/operations/dr/drills | record_dr_drill | done | none | |
| GET | /api/v1/operations/costs | list_costs | done | none | |
| POST | /api/v1/operations/costs | record_cost | done | none | |
| GET | /api/v1/operations/costs/dashboard | get_cost_dashboard | done | none | |
| GET | /api/v1/operations/costs/budgets | list_budgets | done | none | |
| POST | /api/v1/operations/costs/budgets | create_budget | done | none | |
| GET | /api/v1/operations/costs/budgets/{id} | get_budget | done | none | |
| PUT | /api/v1/operations/costs/budgets/{id} | update_budget | done | none | |
| GET | /api/v1/operations/costs/alerts | list_cost_alerts | done | none | |
| POST | /api/v1/operations/costs/alerts/{id}/acknowledge | acknowledge_cost_alert | done | none | |
| GET | /api/v1/operations/costs/utilization | list_resource_utilization | done | none | |
| GET | /api/v1/operations/costs/recommendations | list_optimization_recommendations | done | none | |
| POST | /api/v1/operations/costs/recommendations/{id}/implement | mark_recommendation_implemented | done | none | |

## platform_admin/ (mod.rs router → /api/v1/platform-admin; tenants/features/ops/audit handlers)
All handlers are real (platform_admin_repo / feature_flag_repo / health_monitoring_repo / system_announcement_repo). No test references `/api/v1/platform-admin` → every endpoint is `partial`.
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/platform-admin/organizations | list_organizations | done | none | tenants.rs |
| GET | /api/v1/platform-admin/organizations/{id} | get_organization | done | none | tenants.rs |
| POST | /api/v1/platform-admin/organizations/{id}/suspend | suspend_organization | done | none | tenants.rs |
| POST | /api/v1/platform-admin/organizations/{id}/reactivate | reactivate_organization | done | none | tenants.rs |
| GET | /api/v1/platform-admin/stats | get_platform_stats | done | none | tenants.rs |
| GET | /api/v1/platform-admin/feature-flags | list_feature_flags | done | none | features.rs |
| POST | /api/v1/platform-admin/feature-flags | create_feature_flag | done | none | features.rs |
| GET | /api/v1/platform-admin/feature-flags/{id} | get_feature_flag | done | none | features.rs |
| PUT | /api/v1/platform-admin/feature-flags/{id} | update_feature_flag | done | none | features.rs |
| DELETE | /api/v1/platform-admin/feature-flags/{id} | delete_feature_flag | done | none | features.rs |
| POST | /api/v1/platform-admin/feature-flags/{id}/toggle | toggle_feature_flag | done | none | features.rs |
| POST | /api/v1/platform-admin/feature-flags/{id}/overrides | create_feature_flag_override | done | none | features.rs |
| DELETE | /api/v1/platform-admin/feature-flags/{id}/overrides/{override_id} | delete_feature_flag_override | done | none | features.rs |
| GET | /api/v1/platform-admin/health/dashboard | get_health_dashboard | done | none | ops.rs |
| GET | /api/v1/platform-admin/health/metrics/{name}/history | get_metric_history | done | none | ops.rs |
| GET | /api/v1/platform-admin/health/alerts | get_health_alerts | done | none | ops.rs |
| POST | /api/v1/platform-admin/health/alerts/{id}/acknowledge | acknowledge_alert | done | none | ops.rs |
| GET | /api/v1/platform-admin/health/thresholds | get_thresholds | done | none | ops.rs |
| PUT | /api/v1/platform-admin/health/thresholds/{name} | update_threshold | done | none | ops.rs |
| GET | /api/v1/platform-admin/announcements | list_system_announcements | done | none | ops.rs |
| POST | /api/v1/platform-admin/announcements | create_system_announcement | done | none | ops.rs |
| GET | /api/v1/platform-admin/announcements/{id} | get_system_announcement | done | none | ops.rs |
| PUT | /api/v1/platform-admin/announcements/{id} | update_system_announcement | done | none | ops.rs |
| DELETE | /api/v1/platform-admin/announcements/{id} | delete_system_announcement | done | none | ops.rs |
| POST | /api/v1/platform-admin/maintenance | schedule_maintenance | done | none | ops.rs |
| GET | /api/v1/platform-admin/maintenance | get_upcoming_maintenance_admin | done | none | ops.rs |
| DELETE | /api/v1/platform-admin/maintenance/{id} | delete_scheduled_maintenance | done | none | ops.rs |
| GET | /api/v1/platform-admin/support-data | get_support_data | done | none | audit.rs |
| GET | /api/v1/platform-admin/support/users | search_users_for_support | done | none | audit.rs |
| GET | /api/v1/platform-admin/support/users/{id} | get_user_for_support | done | none | audit.rs |
| GET | /api/v1/platform-admin/support/users/{id}/memberships | get_user_memberships | done | none | audit.rs |
| GET | /api/v1/platform-admin/support/users/{id}/sessions | get_user_sessions | done | none | audit.rs |
| POST | /api/v1/platform-admin/support/users/{id}/sessions/revoke | revoke_user_sessions | done | none | audit.rs |
| GET | /api/v1/platform-admin/support/users/{id}/activity | get_user_activity | done | none | audit.rs |
| GET | /api/v1/platform-admin/onboarding-config | get_onboarding_config | done | none | audit.rs |

## platform_admin/ public routers (mod.rs)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/feature-flags | get_resolved_feature_flags | done | none | public resolved flags; real (feature_flag_repo); no path test |
| GET | /api/v1/system-announcements/active | get_active_announcements | done | none | real; no path test |
| POST | /api/v1/system-announcements/{id}/acknowledge | acknowledge_announcement | done | none | real; no path test |
| GET | /api/v1/maintenance/upcoming | get_upcoming_maintenance | done | none | real; no path test |

## Summary
- done: 132 | partial: 27 | stub: 0 | missing: 0 | total: 159
