# Auth & Identity

_Server: api-server. Modules: auth.rs, oauth.rs, mfa.rs, gdpr.rs, onboarding.rs, help.rs, push_tokens.rs, data_residency.rs._

## auth.rs  (mount: /api/v1/auth)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/auth/register | register | done | auth_enumeration_tests.rs | 201 happy-path asserted (no-id-echo, idempotent) |
| GET | /api/v1/auth/verify-email | verify_email | partial | — | real handler, no test |
| POST | /api/v1/auth/resend-verification | resend_verification | partial | — | real handler, no test |
| POST | /api/v1/auth/login | login | done | auth_enumeration_tests.rs | 200 + accessToken success asserted |
| POST | /api/v1/auth/refresh | refresh_token | partial | endpoints_smoke_tests.rs | smoke only asserts 400/401 (no success path) |
| POST | /api/v1/auth/logout | logout | partial | — | real handler, no test |
| POST | /api/v1/auth/forgot-password | forgot_password | partial | — | real handler, no test |
| POST | /api/v1/auth/reset-password | reset_password | partial | — | real handler, no test |
| GET | /api/v1/auth/sessions | list_sessions | partial | — | real handler, no test |
| POST | /api/v1/auth/sessions/revoke | revoke_session | partial | — | real handler, no test |
| POST | /api/v1/auth/sessions/revoke-all | revoke_all_sessions | partial | — | real handler, no test |
| GET | /api/v1/auth/me | get_me | partial | — | real handler, no test |
| PATCH | /api/v1/auth/me | update_me | partial | — | real handler, no test |

## oauth.rs — router()  (mount: /api/v1/oauth)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/oauth/authorize | authorize_get | done | oauth_authorization_server_test.rs, oauth_integration_tests.rs, oauth_authz_server_edge_tests.rs | happy-path + edge |
| POST | /api/v1/oauth/authorize | authorize_post | done | oauth_authorization_server_test.rs, oauth_integration_tests.rs | consent flow 200 |
| POST | /api/v1/oauth/token | token | done | oauth_integration_tests.rs, oauth_authorization_server_test.rs, oauth_token_introspection_rotation_test.rs | code+refresh rotation |
| POST | /api/v1/oauth/revoke | revoke | done | oauth_integration_tests.rs, oauth_authz_server_edge_tests.rs | |
| POST | /api/v1/oauth/introspect | introspect | done | oauth_integration_tests.rs, oauth_token_introspection_rotation_test.rs | |
| GET | /api/v1/oauth/grants | list_user_grants | done | oauth_integration_tests.rs | 200 |
| DELETE | /api/v1/oauth/grants/{client_id} | revoke_user_grant | done | oauth_integration_tests.rs | 204 NO_CONTENT |

## oauth.rs — admin_router()  (mount: /api/v1/admin/oauth)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/admin/oauth/clients | register_client | done | oauth_client_registration_test.rs | 201 CREATED; router-level OauthClientWrite cap |
| GET | /api/v1/admin/oauth/clients | list_clients | done | oauth_client_registration_test.rs | 200 |
| GET | /api/v1/admin/oauth/clients/{id} | get_client | done | oauth_client_registration_test.rs | 200 |
| PATCH | /api/v1/admin/oauth/clients/{id} | update_client | done | oauth_client_registration_test.rs | 200 |
| DELETE | /api/v1/admin/oauth/clients/{id} | revoke_client | done | oauth_client_registration_test.rs | 204 NO_CONTENT |
| POST | /api/v1/admin/oauth/clients/{id}/regenerate-secret | regenerate_client_secret | done | oauth_client_registration_test.rs | 200 |

## mfa.rs — router()  (mount: /api/v1/auth/mfa)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/auth/mfa/setup | setup_mfa | partial | — | real handler, no test (recovery idor test explicitly skips setup) |
| POST | /api/v1/auth/mfa/verify | verify_mfa_setup | partial | — | real handler; admin_mfa_step_up only hits /api/v1/admin/mfa/verify (different route) |
| POST | /api/v1/auth/mfa/disable | disable_mfa | done | mfa_disable_rls_scope_tests.rs | full success path (200 + recovery-code invalidation) |
| GET | /api/v1/auth/mfa/status | mfa_status | partial | — | real handler, no test |
| POST | /api/v1/auth/mfa/backup-codes/regenerate | regenerate_backup_codes | partial | — | real handler, no test |

## mfa.rs — recovery_codes_router()  (mount: /api/v1/users/me/mfa/recovery-codes)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/users/me/mfa/recovery-codes/verify | verify_recovery_code | done | mfa_recovery_cross_user_idor_tests.rs | 200 success path asserted |

## gdpr.rs  (mount: /api/v1/gdpr)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/gdpr/export/request | request_data_export | partial | — | real (data_export_repo + audit_log), no test |
| GET | /api/v1/gdpr/export/status/{request_id} | get_export_status | partial | — | real handler, no test |
| GET | /api/v1/gdpr/export/download/{token} | download_export | partial | — | real handler, no test |
| GET | /api/v1/gdpr/export/categories | get_export_categories | partial | — | real handler, no test |
| GET | /api/v1/gdpr/export/history | get_export_history | partial | — | real handler, no test |
| POST | /api/v1/gdpr/deletion/request | request_data_deletion | partial | — | real handler, no test |
| GET | /api/v1/gdpr/deletion/status | get_deletion_status | partial | — | real handler, no test |
| POST | /api/v1/gdpr/deletion/cancel | cancel_deletion_request | partial | — | real handler, no test |
| GET | /api/v1/gdpr/privacy | get_privacy_settings | partial | — | real handler, no test |
| POST | /api/v1/gdpr/privacy | update_privacy_settings | partial | — | real handler, no test |

## onboarding.rs  (mount: /api/v1/onboarding)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/onboarding/tours | get_user_tours | partial | — | real (onboarding_repo), no test |
| GET | /api/v1/onboarding/tours/{tour_id} | get_tour | partial | — | real handler, no test |
| POST | /api/v1/onboarding/tours/{tour_id}/start | start_tour | partial | — | real handler, no test |
| POST | /api/v1/onboarding/tours/{tour_id}/steps/{step_id}/complete | complete_step | partial | — | real handler, no test |
| POST | /api/v1/onboarding/tours/{tour_id}/complete | complete_tour | partial | — | real handler, no test |
| POST | /api/v1/onboarding/tours/{tour_id}/skip | skip_tour | partial | — | real handler, no test |
| POST | /api/v1/onboarding/tours/{tour_id}/reset | reset_tour | partial | — | real handler, no test |
| GET | /api/v1/onboarding/status | get_onboarding_status | partial | — | real handler, no test |

## help.rs  (mount: /api/v1/help)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/help/articles | list_articles | done | help_tests.rs | 200 |
| GET | /api/v1/help/articles/search | search_articles | done | help_tests.rs | 200 |
| GET | /api/v1/help/articles/category/{category} | list_articles_by_category | done | help_tests.rs | 200 |
| GET | /api/v1/help/articles/context/{context_key} | list_articles_by_context | done | help_tests.rs | 200 |
| GET | /api/v1/help/articles/{slug} | get_article | partial | help_tests.rs | only 404 not-found test; no success path |
| POST | /api/v1/help/articles/{slug}/feedback | submit_article_feedback | partial | help_tests.rs | only no-auth rejection test |
| GET | /api/v1/help/categories | list_categories | done | help_tests.rs | 200 |
| GET | /api/v1/help/categories/{slug} | get_category | partial | help_tests.rs | only 404 not-found test |
| GET | /api/v1/help/faq | list_faq | done | help_tests.rs | 200 |
| GET | /api/v1/help/faq/search | search_faq | done | help_tests.rs | 200 |
| GET | /api/v1/help/faq/category/{category} | list_faq_by_category | done | help_tests.rs | 200 |
| GET | /api/v1/help/tooltips | list_tooltips | done | help_tests.rs | 200 |
| GET | /api/v1/help/tooltips/{key} | get_tooltip | partial | help_tests.rs | only 404 not-found test |
| GET | /api/v1/help/tooltips/prefix/{prefix} | list_tooltips_by_prefix | done | help_tests.rs | 200 |

## push_tokens.rs  (mount: /api/v1/users/me/push-tokens)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/users/me/push-tokens | register_push_token | done | push_token_tests.rs | 200 success path asserted |
| DELETE | /api/v1/users/me/push-tokens/{token} | unregister_push_token | partial | push_token_tests.rs, push_fanout_stale_token_isolation_tests.rs | only no-auth/cross-user rejection + direct-DB checks; no authed endpoint success |

## data_residency.rs  (mount: /api/v1/data-residency)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/data-residency/config | get_residency_config | stub | — | ignores _state; returns hardcoded EuWest config + Uuid::new_v4()/Utc::now() |
| POST | /api/v1/data-residency/config | configure_residency | stub | — | ignores _state; returns synthesized config, no persistence |
| PUT | /api/v1/data-residency/config | update_residency_config | stub | — | delegates to configure_residency (mock) |
| GET | /api/v1/data-residency/regions | list_available_regions | stub | — | ignores _state; hardcoded region list |
| GET | /api/v1/data-residency/routing/status | get_routing_status | stub | — | ignores _state; mock data |
| POST | /api/v1/data-residency/routing/log-access | log_cross_region_access | stub | — | ignores _state; returns fake Uuid::new_v4() entry, no persistence |
| GET | /api/v1/data-residency/routing/access-logs | list_access_logs | stub | — | ignores _state; synthesized log entries |
| POST | /api/v1/data-residency/compliance/verify | run_compliance_verification | stub | — | ignores _state; hardcoded verification result |
| GET | /api/v1/data-residency/compliance/verification/{id} | get_verification_result | stub | — | ignores _state; empty/mock result |
| GET | /api/v1/data-residency/compliance/export | export_compliance_report | stub | — | ignores _state; fake download_url with random UUID |
| GET | /api/v1/data-residency/audit | list_audit_logs | stub | — | ignores _state and _query; hardcoded 2-entry list |
| GET | /api/v1/data-residency/audit/{id} | get_audit_entry | stub | — | ignores _state; synthesized entry |
| POST | /api/v1/data-residency/audit/verify-chain | verify_audit_chain | stub | — | ignores _state; hardcoded chain-verify JSON |
| GET | /api/v1/data-residency/dashboard | get_residency_dashboard | stub | — | ignores _state; synthesized dashboard |

## Summary
- done: 28 | partial: 38 | stub: 14 | missing: 0 | total: 80
