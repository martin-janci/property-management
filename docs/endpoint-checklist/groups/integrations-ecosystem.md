# Integrations & Ecosystem

_Server: api-server. Modules: marketplace.rs, public_api.rs, api_ecosystem.rs, portal_webhooks.rs, voice_webhooks.rs, migration.rs, feature_packages.rs, features.rs, integrations/ (install, oauth, webhook, booking_channel, token_rotation, airbnb_connections; sync = unmounted)._

## marketplace.rs  (mount: /api/v1/marketplace)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/marketplace/providers | create_profile | done | integrations_batch4_tests.rs | happy-path 201 |
| GET | /api/v1/marketplace/providers | search_providers | done | integrations_batch4_tests.rs | happy-path 200 |
| GET | /api/v1/marketplace/providers/me | get_my_profile | done | integrations_batch4_tests.rs | happy-path 200 |
| PATCH | /api/v1/marketplace/providers/me | update_my_profile | done | integrations_batch4_tests.rs | happy-path 200 |
| GET | /api/v1/marketplace/providers/me/dashboard | get_provider_dashboard | done | integrations_batch4_tests.rs | happy-path 200 |
| GET | /api/v1/marketplace/providers/statistics | get_marketplace_statistics | done | integrations_batch4_tests.rs | happy-path 200 |
| GET | /api/v1/marketplace/providers/{id} | get_provider | done | integrations_batch4_tests.rs | happy-path 200 |
| GET | /api/v1/marketplace/providers/{id}/complete | get_provider_complete | done | integrations_batch4_tests.rs | happy-path 200 |
| POST | /api/v1/marketplace/rfqs | create_rfq | partial | — | |
| GET | /api/v1/marketplace/rfqs | list_rfqs | partial | — | |
| GET | /api/v1/marketplace/rfqs/{id} | get_rfq | done | marketplace_voting_investor_cross_org_idor_tests.rs | happy-path OK (owner reads own RFQ) |
| PATCH | /api/v1/marketplace/rfqs/{id} | update_rfq | partial | — | |
| DELETE | /api/v1/marketplace/rfqs/{id} | delete_rfq | partial | — | |
| GET | /api/v1/marketplace/rfqs/{id}/quotes | list_rfq_quotes | partial | — | |
| GET | /api/v1/marketplace/rfqs/{id}/compare | compare_quotes | partial | — | |
| POST | /api/v1/marketplace/rfqs/{id}/award | award_quote | partial | — | |
| POST | /api/v1/marketplace/rfqs/{id}/cancel | cancel_rfq | partial | — | |
| POST | /api/v1/marketplace/quotes | submit_quote | partial | — | |
| GET | /api/v1/marketplace/quotes/my | list_my_quotes | partial | — | |
| GET | /api/v1/marketplace/quotes/{id} | get_quote | partial | — | |
| PATCH | /api/v1/marketplace/quotes/{id} | update_quote | partial | — | |
| DELETE | /api/v1/marketplace/quotes/{id} | withdraw_quote | partial | — | |
| GET | /api/v1/marketplace/invitations | list_my_invitations | partial | — | |
| POST | /api/v1/marketplace/invitations/{id}/view | mark_invitation_viewed | done | marketplace_voting_investor_cross_org_idor_tests.rs | happy-path OK |
| POST | /api/v1/marketplace/invitations/{id}/decline | decline_invitation | done | marketplace_voting_investor_cross_org_idor_tests.rs | happy-path OK |
| POST | /api/v1/marketplace/verifications | submit_verification | partial | — | |
| GET | /api/v1/marketplace/verifications | list_verifications | partial | — | |
| GET | /api/v1/marketplace/verifications/queue | get_verification_queue | partial | — | |
| GET | /api/v1/marketplace/verifications/expiring | get_expiring_verifications | partial | — | |
| GET | /api/v1/marketplace/verifications/{id} | get_verification | done | marketplace_voting_investor_cross_org_idor_tests.rs | happy-path OK |
| POST | /api/v1/marketplace/verifications/{id}/review | review_verification | partial | marketplace_voting_investor_cross_org_idor_tests.rs | authz-only (403) |
| GET | /api/v1/marketplace/providers/{id}/badges | list_provider_badges | partial | — | |
| POST | /api/v1/marketplace/providers/{id}/badges | award_badge | partial | — | |
| DELETE | /api/v1/marketplace/badges/{id} | revoke_badge | partial | marketplace_voting_investor_cross_org_idor_tests.rs | authz-only (403) |
| POST | /api/v1/marketplace/providers/{id}/reviews | create_review | partial | — | |
| GET | /api/v1/marketplace/providers/{id}/reviews | list_provider_reviews | partial | — | |
| GET | /api/v1/marketplace/providers/{id}/ratings | get_rating_breakdown | partial | — | |
| GET | /api/v1/marketplace/reviews | list_reviews | partial | — | |
| GET | /api/v1/marketplace/reviews/{id} | get_review | partial | — | |
| PATCH | /api/v1/marketplace/reviews/{id} | update_review | partial | — | |
| DELETE | /api/v1/marketplace/reviews/{id} | delete_review | partial | — | |
| POST | /api/v1/marketplace/reviews/{id}/respond | respond_to_review | partial | — | |
| POST | /api/v1/marketplace/reviews/{id}/moderate | moderate_review | partial | — | |
| POST | /api/v1/marketplace/reviews/{id}/helpful | mark_review_helpful | partial | — | |
| GET | /api/v1/marketplace/dashboard | get_manager_dashboard | partial | — | |

## public_api.rs  (mount: UNMOUNTED — ROADMAP(PAP-24) /api/v1/developer not nested in lib.rs)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | (/api/v1/developer)/accounts | create_developer_account | stub | — | unmounted; roadmap stub returns 501 |
| GET | (/api/v1/developer)/accounts/me | get_my_developer_account | stub | — | unmounted |
| PATCH | (/api/v1/developer)/accounts/me | update_my_developer_account | stub | — | unmounted |
| GET | (/api/v1/developer)/accounts/me/usage | get_my_usage_summary | stub | — | unmounted |
| POST | (/api/v1/developer)/keys | create_api_key | stub | — | unmounted |
| GET | (/api/v1/developer)/keys | list_api_keys | stub | — | unmounted |
| GET | (/api/v1/developer)/keys/{id} | get_api_key | stub | — | unmounted |
| PATCH | (/api/v1/developer)/keys/{id} | update_api_key | stub | — | unmounted |
| DELETE | (/api/v1/developer)/keys/{id} | revoke_api_key | stub | — | unmounted |
| POST | (/api/v1/developer)/keys/{id}/rotate | rotate_api_key | stub | — | unmounted |
| GET | (/api/v1/developer)/keys/{id}/usage | get_api_key_usage | stub | — | unmounted |
| GET | (/api/v1/developer)/docs/endpoints | list_api_endpoints | stub | — | unmounted |
| GET | (/api/v1/developer)/docs/endpoints/{id} | get_api_endpoint | stub | — | unmounted |
| GET | (/api/v1/developer)/docs/changelog | list_api_changelog | stub | — | unmounted |
| GET | (/api/v1/developer)/docs/openapi | get_openapi_spec | stub | — | unmounted |
| POST | (/api/v1/developer)/sandbox | create_sandbox | stub | — | unmounted |
| POST | (/api/v1/developer)/sandbox/test | test_sandbox_request | stub | — | unmounted |
| GET | (/api/v1/developer)/sandbox/{id} | get_sandbox | stub | — | unmounted |
| DELETE | (/api/v1/developer)/sandbox/{id} | delete_sandbox | stub | — | unmounted |
| POST | (/api/v1/developer)/webhooks | create_webhook | stub | — | unmounted |
| GET | (/api/v1/developer)/webhooks | list_webhooks | stub | — | unmounted |
| GET | (/api/v1/developer)/webhooks/{id} | get_webhook | stub | — | unmounted |
| PATCH | (/api/v1/developer)/webhooks/{id} | update_webhook | stub | — | unmounted |
| DELETE | (/api/v1/developer)/webhooks/{id} | delete_webhook | stub | — | unmounted |
| POST | (/api/v1/developer)/webhooks/{id}/test | test_webhook | stub | — | unmounted |
| POST | (/api/v1/developer)/webhooks/{id}/rotate-secret | rotate_webhook_secret | stub | — | unmounted |
| GET | (/api/v1/developer)/webhooks/{id}/deliveries | list_webhook_deliveries | stub | — | unmounted |
| GET | (/api/v1/developer)/webhooks/events | list_webhook_event_types | stub | — | unmounted |
| GET | (/api/v1/developer)/rate-limits/status | get_rate_limit_status | stub | — | unmounted |
| GET | (/api/v1/developer)/rate-limits/tiers | list_rate_limit_tiers | stub | — | unmounted |
| GET | (/api/v1/developer)/sdks | list_sdk_languages | stub | — | unmounted |
| GET | (/api/v1/developer)/sdks/{language} | get_sdk_info | stub | — | unmounted |
| GET | (/api/v1/developer)/sdks/{language}/download | download_sdk | stub | — | unmounted |
| GET | (/api/v1/developer)/sdks/{language}/versions | list_sdk_versions | stub | — | unmounted |
| GET | (/api/v1/developer)/admin/developers | list_developers | stub | — | unmounted |
| GET | (/api/v1/developer)/admin/developers/{id} | get_developer | stub | — | unmounted |
| PATCH | (/api/v1/developer)/admin/developers/{id} | update_developer | stub | — | unmounted |
| POST | (/api/v1/developer)/admin/developers/{id}/verify | verify_developer | stub | — | unmounted |
| POST | (/api/v1/developer)/admin/developers/{id}/suspend | suspend_developer | stub | — | unmounted |
| POST | (/api/v1/developer)/admin/rate-limits | create_rate_limit_config | stub | — | unmounted |
| PATCH | (/api/v1/developer)/admin/rate-limits/{id} | update_rate_limit_config | stub | — | unmounted |
| GET | (/api/v1/developer)/admin/stats | get_portal_stats | stub | — | unmounted |
| GET | (/api/v1/developer)/admin/request-logs | list_request_logs | stub | — | unmounted |

## api_ecosystem.rs  (mount: /api/v1/ecosystem)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/ecosystem/marketplace | list_marketplace_integrations | partial | — | router asserted in router_single_source_tests (static only, no path-exercising test) |
| POST | /api/v1/ecosystem/marketplace | create_marketplace_integration | partial | — | |
| GET | /api/v1/ecosystem/marketplace/{id} | get_marketplace_integration | partial | — | |
| PUT | /api/v1/ecosystem/marketplace/{id} | update_marketplace_integration | partial | — | |
| DELETE | /api/v1/ecosystem/marketplace/{id} | delete_marketplace_integration | partial | — | |
| GET | /api/v1/ecosystem/marketplace/categories | list_integration_categories | partial | — | |
| GET | /api/v1/ecosystem/marketplace/{id}/ratings | list_integration_ratings | partial | — | |
| POST | /api/v1/ecosystem/marketplace/{id}/ratings | create_integration_rating | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/integrations | list_organization_integrations | partial | — | |
| POST | /api/v1/ecosystem/organizations/{org_id}/integrations | install_integration | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/integrations/{id} | get_organization_integration | partial | — | |
| PUT | /api/v1/ecosystem/organizations/{org_id}/integrations/{id} | update_organization_integration | partial | — | |
| DELETE | /api/v1/ecosystem/organizations/{org_id}/integrations/{id} | uninstall_integration | partial | — | |
| POST | /api/v1/ecosystem/organizations/{org_id}/integrations/{id}/sync | sync_integration | partial | — | |
| GET | /api/v1/ecosystem/connectors | list_connectors | partial | — | |
| POST | /api/v1/ecosystem/connectors | create_connector | partial | — | |
| GET | /api/v1/ecosystem/connectors/{id} | get_connector | partial | — | |
| PUT | /api/v1/ecosystem/connectors/{id} | update_connector | partial | — | |
| DELETE | /api/v1/ecosystem/connectors/{id} | delete_connector | partial | — | |
| GET | /api/v1/ecosystem/connectors/{id}/actions | list_connector_actions | partial | — | |
| POST | /api/v1/ecosystem/connectors/{id}/actions | create_connector_action | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/connector-logs | list_connector_logs | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/webhooks | list_enhanced_webhooks | partial | — | |
| POST | /api/v1/ecosystem/organizations/{org_id}/webhooks | create_enhanced_webhook | partial | — | |
| GET | /api/v1/ecosystem/webhooks/{id} | get_enhanced_webhook | partial | — | |
| PUT | /api/v1/ecosystem/webhooks/{id} | update_enhanced_webhook | partial | — | |
| DELETE | /api/v1/ecosystem/webhooks/{id} | delete_enhanced_webhook | partial | — | |
| POST | /api/v1/ecosystem/webhooks/{id}/test | test_enhanced_webhook | partial | — | |
| GET | /api/v1/ecosystem/webhooks/{id}/logs | list_webhook_delivery_logs | partial | — | |
| GET | /api/v1/ecosystem/webhooks/{id}/stats | get_enhanced_webhook_stats | partial | — | |
| GET | /api/v1/ecosystem/webhooks/events | list_webhook_event_types | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/prebuilt | list_prebuilt_connections | partial | — | |
| POST | /api/v1/ecosystem/organizations/{org_id}/prebuilt | create_prebuilt_connection | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type} | get_prebuilt_connection | partial | — | |
| PUT | /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type} | update_prebuilt_connection | partial | — | |
| DELETE | /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type} | delete_prebuilt_connection | partial | — | |
| POST | /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}/sync | sync_prebuilt_connection | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}/oauth | get_prebuilt_oauth_url | partial | — | |
| POST | /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}/oauth/callback | handle_prebuilt_oauth_callback | partial | — | |
| POST | /api/v1/ecosystem/developers/register | register_developer | partial | — | |
| GET | /api/v1/ecosystem/developers/{id} | get_developer_registration | partial | — | |
| POST | /api/v1/ecosystem/developers/{id}/review | review_developer_registration | partial | — | |
| GET | /api/v1/ecosystem/developers/{id}/keys | list_developer_api_keys | partial | — | |
| POST | /api/v1/ecosystem/developers/{id}/keys | create_developer_api_key | partial | — | |
| DELETE | /api/v1/ecosystem/developers/{id}/keys/{key_id} | revoke_developer_api_key | partial | — | |
| POST | /api/v1/ecosystem/developers/{id}/keys/{key_id}/rotate | rotate_developer_api_key | partial | — | |
| GET | /api/v1/ecosystem/developers/{id}/usage | get_developer_usage_stats | partial | — | |
| POST | /api/v1/ecosystem/developers/{id}/sandbox | create_sandbox_environment | partial | — | |
| GET | /api/v1/ecosystem/developers/{id}/sandbox | get_sandbox_environment | partial | — | |
| POST | /api/v1/ecosystem/developers/{id}/sandbox/test | test_sandbox_request | partial | — | |
| GET | /api/v1/ecosystem/docs | list_api_documentation | partial | — | |
| POST | /api/v1/ecosystem/docs | create_api_documentation | partial | — | |
| GET | /api/v1/ecosystem/docs/{slug} | get_api_documentation | partial | — | |
| PUT | /api/v1/ecosystem/docs/{slug} | update_api_documentation | partial | — | |
| DELETE | /api/v1/ecosystem/docs/{slug} | delete_api_documentation | partial | — | |
| GET | /api/v1/ecosystem/docs/{slug}/code-samples | list_code_samples | partial | — | |
| POST | /api/v1/ecosystem/docs/{slug}/code-samples | create_code_sample | partial | — | |
| GET | /api/v1/ecosystem/portal/stats | get_developer_portal_stats | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/dashboard | get_ecosystem_dashboard | partial | — | |
| GET | /api/v1/ecosystem/organizations/{org_id}/stats | get_ecosystem_statistics | partial | — | |

## portal_webhooks.rs  (mount: /api/v1/webhooks/portals)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/webhooks/portals/reality-portal/views | reality_portal_views_webhook | done | portal_webhook_signature_tests.rs | valid-signature happy-path OK + 401 cases |
| POST | /api/v1/webhooks/portals/reality-portal/inquiries | reality_portal_inquiry_webhook | partial | — | |
| POST | /api/v1/webhooks/portals/sreality/views | sreality_views_webhook | partial | — | |
| POST | /api/v1/webhooks/portals/sreality/inquiries | sreality_inquiry_webhook | partial | — | |
| POST | /api/v1/webhooks/portals/bezrealitky/views | bezrealitky_views_webhook | partial | — | |
| POST | /api/v1/webhooks/portals/bezrealitky/inquiries | bezrealitky_inquiry_webhook | partial | — | |
| POST | /api/v1/webhooks/portals/nehnutelnosti/views | nehnutelnosti_views_webhook | partial | — | |
| POST | /api/v1/webhooks/portals/nehnutelnosti/inquiries | nehnutelnosti_inquiry_webhook | partial | — | |
| POST | /api/v1/webhooks/portals/{portal}/events | generic_portal_webhook | partial | — | |

## voice_webhooks.rs  (mount: /api/v1/webhooks/voice)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/webhooks/voice/alexa | alexa_webhook | partial | — | real handler, no test |
| POST | /api/v1/webhooks/voice/alexa/health | alexa_health_check | partial | — | |
| POST | /api/v1/webhooks/voice/google | google_actions_webhook | partial | — | |
| POST | /api/v1/webhooks/voice/oauth/exchange | oauth_token_exchange | partial | voice_oauth_exchange_auth_tests.rs | authz-only (401/403), no success path |
| POST | /api/v1/webhooks/voice/oauth/refresh | oauth_token_refresh | partial | — | |
| POST | /api/v1/webhooks/voice/verify | verify_webhook_signature | partial | — | |

## migration.rs  (mount: /api/v1/migration)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/migration/templates | list_templates | done | migration_db_tests.rs | db-backed and tested |
| POST | /api/v1/migration/templates | create_template | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/templates/system | list_system_templates | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/templates/{template_id} | get_template | done | migration_db_tests.rs | db-backed and tested |
| PUT | /api/v1/migration/templates/{template_id} | update_template | done | migration_db_tests.rs | db-backed and tested |
| DELETE | /api/v1/migration/templates/{template_id} | delete_template | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/templates/{template_id}/download | download_template | done | migration_db_tests.rs | db-backed and tested |
| POST | /api/v1/migration/templates/{template_id}/duplicate | duplicate_template | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/categories/import | get_import_categories | done | migration_db_tests.rs | static metadata |
| POST | /api/v1/migration/import/upload | upload_import_file | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/import/jobs | list_import_jobs | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/import/jobs/{job_id} | get_import_job_status | done | migration_db_tests.rs | db-backed and tested |
| POST | /api/v1/migration/import/jobs/{job_id}/cancel | cancel_import_job | done | migration_db_tests.rs | db-backed and tested |
| POST | /api/v1/migration/import/jobs/{job_id}/retry | retry_import_job | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/import/jobs/{job_id}/errors | get_import_job_errors | done | migration_db_tests.rs | db-backed and tested |
| POST | /api/v1/migration/export | request_migration_export | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/export/{export_id} | get_export_status | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/export/{export_id}/download | download_export | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/export/history | get_export_history | done | migration_db_tests.rs | db-backed and tested |
| GET | /api/v1/migration/categories/export | get_export_categories | done | migration_db_tests.rs | static metadata |
| GET | /api/v1/migration/import/jobs/{job_id}/preview | get_import_preview | done | migration_db_tests.rs | db-backed and tested |
| POST | /api/v1/migration/import/jobs/{job_id}/approve | approve_import | done | migration_db_tests.rs | db-backed and tested |
| POST | /api/v1/migration/import/jobs/{job_id}/validate | validate_import | done | migration_db_tests.rs | db-backed and tested |

## feature_packages.rs  (mount: /api/v1/feature-packages; public_router nested at /public)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/feature-packages/ | list_packages | done | integrations_batch4_tests.rs | happy-path 200 (super_admin) |
| POST | /api/v1/feature-packages/ | create_package | done | integrations_batch4_tests.rs | happy-path 201 |
| GET | /api/v1/feature-packages/{id} | get_package | done | integrations_batch4_tests.rs | happy-path 200 |
| PUT | /api/v1/feature-packages/{id} | update_package | done | integrations_batch4_tests.rs | happy-path 200 |
| DELETE | /api/v1/feature-packages/{id} | delete_package | done | integrations_batch4_tests.rs | happy-path 204 |
| POST | /api/v1/feature-packages/{id}/features | add_features | done | integrations_batch4_tests.rs | happy-path 201 |
| DELETE | /api/v1/feature-packages/{id}/features/{fid} | remove_feature | done | integrations_batch4_tests.rs | happy-path 204 |
| GET | /api/v1/feature-packages/organizations/{org_id} | get_org_packages | done | integrations_batch4_tests.rs | happy-path 200 |
| POST | /api/v1/feature-packages/organizations/{org_id}/assign | assign_package | done | integrations_batch4_tests.rs | happy-path 201 |
| DELETE | /api/v1/feature-packages/organizations/{org_id}/packages/{pid} | deactivate_org_package | done | integrations_batch4_tests.rs | happy-path 204 |
| GET | /api/v1/feature-packages/public/ | list_public_packages | done | integrations_batch4_tests.rs | happy-path 200 (no auth) |
| GET | /api/v1/feature-packages/public/compare | compare_packages | done | integrations_batch4_tests.rs | happy-path 200 (no auth) |
| GET | /api/v1/feature-packages/public/{id} | get_public_package | done | integrations_batch4_tests.rs | happy-path 200 (no auth) |

## features.rs  (mount: /api/v1/features)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/features/resolved | get_resolved_features | done | integrations_batch4_tests.rs | happy-path 200 |
| GET | /api/v1/features/{key}/check | check_feature | done | integrations_batch4_tests.rs | happy-path 200 |
| GET | /api/v1/features/{key}/upgrade-options | get_upgrade_options | done | integrations_batch4_tests.rs | happy-path 200 |
| POST | /api/v1/features/{key}/preference | set_feature_preference | partial | — | |
| POST | /api/v1/features/analytics/event | log_feature_event | done | integrations_batch4_tests.rs | happy-path 200 |
| GET | /api/v1/features/analytics/{feature_id}/stats | get_feature_stats | partial | — | |

## integrations/install.rs  (mount: /api/v1/integrations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/integrations/organizations/{org_id}/airbnb/status | get_airbnb_status | done | integrations_cross_org_idor_tests.rs | happy-path OK (member, no connection) |
| POST | /api/v1/integrations/organizations/{org_id}/airbnb/connect | connect_airbnb | partial | — | |
| POST | /api/v1/integrations/organizations/{org_id}/airbnb/sync | sync_airbnb | partial | — | |
| DELETE | /api/v1/integrations/organizations/{org_id}/airbnb | disconnect_airbnb | partial | — | |
| POST | /api/v1/integrations/organizations/{org_id}/airbnb/direct-connect | direct_connect_airbnb | partial | — | |
| POST | /api/v1/integrations/organizations/{org_id}/airbnb/availability-sync | enqueue_airbnb_availability_sync | partial | — | |
| GET | /api/v1/integrations/organizations/{org_id}/booking/status | get_booking_status | partial | — | |
| POST | /api/v1/integrations/organizations/{org_id}/booking/connect | connect_booking | partial | — | |
| POST | /api/v1/integrations/organizations/{org_id}/booking/sync | sync_booking | partial | — | |
| POST | /api/v1/integrations/organizations/{org_id}/booking/push-availability | push_booking_availability | partial | — | |
| POST | /api/v1/integrations/organizations/{org_id}/booking/push-rates | push_booking_rates | partial | — | |
| DELETE | /api/v1/integrations/organizations/{org_id}/booking | disconnect_booking | partial | — | |
| GET | /api/v1/integrations/organizations/{org_id}/portals | list_portal_connections | partial | — | |
| POST | /api/v1/integrations/organizations/{org_id}/portals | create_portal_connection | partial | — | |
| GET | /api/v1/integrations/portals/{id} | get_portal_connection | partial | — | |
| DELETE | /api/v1/integrations/portals/{id} | delete_portal_connection | partial | — | |
| GET | /api/v1/integrations/organizations/{org_id}/portal-inquiries | list_portal_inquiries | partial | — | |
| GET | /api/v1/integrations/portal-inquiries/{id} | get_portal_inquiry | partial | — | |
| POST | /api/v1/integrations/portal-inquiries/{id}/read | mark_inquiry_read | partial | — | |
| POST | /api/v1/integrations/portal-inquiries/{id}/archive | archive_inquiry | partial | — | |

## integrations/oauth.rs  (mount: /api/v1/integrations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/integrations/organizations/{org_id}/airbnb/callback | airbnb_oauth_callback | partial | booking_oauth_csrf_tests.rs | CSRF/error paths only (BAD_REQUEST), no success |
| POST | /api/v1/integrations/organizations/{org_id}/airbnb/token/exchange | airbnb_token_exchange | partial | airbnb_oauth_routes_tests.rs | authz/error only, no 2xx |
| GET | /api/v1/integrations/organizations/{org_id}/airbnb/listings | list_airbnb_listings | partial | airbnb_oauth_routes_tests.rs | authz/error only, no 2xx |
| GET | /api/v1/integrations/organizations/{org_id}/airbnb/reservations | list_airbnb_reservations | partial | airbnb_oauth_routes_tests.rs | authz/error only, no 2xx |
| POST | /api/v1/integrations/organizations/{org_id}/booking/token/exchange | booking_token_exchange | partial | booking_oauth_routes_tests.rs, booking_oauth_csrf_tests.rs | authz/validation only (400/403/503), no success path |

## integrations/webhook.rs  (mount: /api/v1/integrations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/integrations/booking/push | booking_push_notification | partial | — | |
| POST | /api/v1/integrations/webhooks/portal/{connection_id} | handle_portal_webhook | partial | — | |
| POST | /api/v1/integrations/airbnb/webhook | handle_airbnb_webhook | done | airbnb_webhook_routes_tests.rs, airbnb_sync_reconciliation_tests.rs | multiple happy-path OK |
| POST | /api/v1/integrations/webhooks/payments/{provider} | handle_payment_webhook | done | payment_webhook_settlement_tests.rs | happy-path OK (settlement) |

## integrations/booking_channel.rs  (mount: /api/v1/integrations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/integrations/organizations/{org_id}/booking/listing-push | push_booking_listing | partial | — | |
| GET | /api/v1/integrations/organizations/{org_id}/booking/conflicts | get_booking_conflicts | partial | — | |

## integrations/token_rotation.rs  (mount: /api/v1/integrations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/integrations/organizations/{org_id}/airbnb/token/revoke | revoke_airbnb_token | partial | — | |

## integrations/airbnb_connections.rs  (mount: /api/v1/integrations)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/integrations/organizations/{org_id}/airbnb/connections | list_airbnb_connections | done | airbnb_connections_routes_tests.rs | happy-path OK (list 200) |

## integrations/sync.rs  (mount: UNMOUNTED — ROADMAP(PAP-122), backing Epic-61 tables exist in no migration)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | (unmounted)/organizations/{org_id}/stats | get_integration_stats | stub | — | unmounted; Epic-61 tables missing |
| GET | (unmounted)/organizations/{org_id}/calendars | list_calendar_connections | stub | — | unmounted |
| POST | (unmounted)/organizations/{org_id}/calendars | create_calendar_connection | stub | — | unmounted |
| GET | (unmounted)/calendars/{id} | get_calendar_connection | stub | — | unmounted |
| PUT | (unmounted)/calendars/{id} | update_calendar_connection | stub | — | unmounted |
| DELETE | (unmounted)/calendars/{id} | delete_calendar_connection | stub | — | unmounted |
| POST | (unmounted)/calendars/{id}/sync | sync_calendar | stub | — | unmounted |
| GET | (unmounted)/calendars/{id}/events | list_calendar_events | stub | — | unmounted |
| POST | (unmounted)/calendars/{id}/events | create_calendar_event | stub | — | unmounted |
| GET | (unmounted)/organizations/{org_id}/accounting/exports | list_accounting_exports | stub | — | unmounted |
| POST | (unmounted)/organizations/{org_id}/accounting/exports | create_accounting_export | stub | — | unmounted |
| GET | (unmounted)/accounting/exports/{id} | get_accounting_export | stub | — | unmounted |
| GET | (unmounted)/accounting/exports/{id}/download | download_accounting_export | stub | — | unmounted |
| GET | (unmounted)/organizations/{org_id}/accounting/settings/{system} | get_accounting_settings | stub | — | unmounted |
| PUT | (unmounted)/organizations/{org_id}/accounting/settings/{system} | update_accounting_settings | stub | — | unmounted |
| GET | (unmounted)/organizations/{org_id}/esignatures | list_esignature_workflows | stub | — | unmounted |
| POST | (unmounted)/organizations/{org_id}/esignatures | create_esignature_workflow | stub | — | unmounted |
| GET | (unmounted)/esignatures/{id} | get_esignature_workflow | stub | — | unmounted |
| POST | (unmounted)/esignatures/{id}/send | send_esignature_workflow | stub | — | unmounted |
| POST | (unmounted)/esignatures/{id}/void | void_esignature_workflow | stub | — | unmounted |
| POST | (unmounted)/esignatures/{id}/remind | send_esignature_reminder | stub | — | unmounted |
| GET | (unmounted)/organizations/{org_id}/video/connections | list_video_connections | stub | — | unmounted |
| POST | (unmounted)/organizations/{org_id}/video/connections | create_video_connection | stub | — | unmounted |
| DELETE | (unmounted)/video/connections/{id} | delete_video_connection | stub | — | unmounted |
| GET | (unmounted)/organizations/{org_id}/video/meetings | list_video_meetings | stub | — | unmounted |
| POST | (unmounted)/organizations/{org_id}/video/meetings | create_video_meeting | stub | — | unmounted |
| GET | (unmounted)/video/meetings/{id} | get_video_meeting | stub | — | unmounted |
| PUT | (unmounted)/video/meetings/{id} | update_video_meeting | stub | — | unmounted |
| DELETE | (unmounted)/video/meetings/{id} | delete_video_meeting | stub | — | unmounted |
| POST | (unmounted)/video/meetings/{id}/start | start_video_meeting | stub | — | unmounted |

## Summary
- done: 32 | partial: 163 | stub: 73 | missing: 0 | total: 268
</content>
