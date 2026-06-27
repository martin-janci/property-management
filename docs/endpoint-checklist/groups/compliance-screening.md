# Compliance & Screening

_Server: api-server. Modules: compliance.rs, regional_compliance.rs, aml_dsa/, enhanced_tenant_screening/._

## compliance.rs  (mount: /api/v1/compliance)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/compliance/audit-logs | get_audit_logs | done | compliance_wave1b_tests.rs | SuperAdmin-gated; 403 for non-super-admin + 200 with logs/total. |
| GET | /api/v1/compliance/audit-logs/summary | get_audit_summary | done | compliance_wave1b_tests.rs | SuperAdmin-gated; 403 + 200 with total_entries/action_counts. |
| GET | /api/v1/compliance/audit-logs/user/{user_id} | get_user_audit_logs | done | compliance_wave1b_tests.rs | SuperAdmin-gated; 403 + 200 with user-filtered logs. |
| GET | /api/v1/compliance/audit-logs/integrity | verify_audit_integrity | done | compliance_wave1b_tests.rs | SuperAdmin-gated; 403 + 200 with verified bool. |
| GET | /api/v1/compliance/gdpr/data-exports | get_data_export_report | stub | none | Ignores params; returns exports=[], completed_count=0, downloaded_count=0 with "For now, return a summary ... in production this would query with filters" comment. Only a pending count is real → mock/TODO data. |
| GET | /api/v1/compliance/gdpr/deletion-requests | get_deletion_requests_report | partial | none | Real: raw SQL over users.scheduled_deletion_at. No test. |
| GET | /api/v1/compliance/gdpr/privacy-report | get_privacy_settings_report | partial | none | Real: raw SQL aggregations over users. No test. |
| GET | /api/v1/compliance/security/login-activity | get_login_activity_report | partial | none | Real: raw SQL over audit_logs. No test. |
| GET | /api/v1/compliance/security/mfa-status | get_mfa_status_report | partial | none | Real: raw SQL over user_2fa/users. No test. |
| GET | /api/v1/compliance/security/failed-logins | get_failed_logins_report | partial | none | Real: raw SQL over audit_logs. No test. |

## regional_compliance.rs  (mount: /api/v1/regional-compliance)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/regional-compliance/jurisdiction | get_jurisdiction | stub | none | Returns Jurisdiction::default(); ignores state. |
| PUT | /api/v1/regional-compliance/jurisdiction | set_jurisdiction | stub | none | Echoes payload back; no persistence. |
| POST | /api/v1/regional-compliance/slovak/voting/config | configure_slovak_voting | stub | none | Mock: Uuid::new_v4() ids, no DB. |
| GET | /api/v1/regional-compliance/slovak/voting/config/{building_id} | get_slovak_voting_config | stub | none | Hardcoded mock config. |
| POST | /api/v1/regional-compliance/slovak/voting/validate | validate_slovak_vote | stub | none | Hardcoded participation/approval percentages. |
| GET | /api/v1/regional-compliance/slovak/voting/minutes/{vote_id} | get_slovak_vote_minutes | stub | none | Fully hardcoded minutes. |
| POST | /api/v1/regional-compliance/slovak/accounting/config | configure_slovak_accounting | stub | none | Mock: Uuid::new_v4(), no DB. |
| GET | /api/v1/regional-compliance/slovak/accounting/config | get_slovak_accounting_config | stub | none | Hardcoded mock config. |
| POST | /api/v1/regional-compliance/slovak/accounting/export | export_slovak_accounting | stub | none | Hardcoded counts/totals; fake download_url. |
| POST | /api/v1/regional-compliance/slovak/gdpr/config | configure_slovak_gdpr | stub | none | Mock: Uuid::new_v4(), no DB. |
| GET | /api/v1/regional-compliance/slovak/gdpr/config | get_slovak_gdpr_config | stub | none | Hardcoded mock config. |
| POST | /api/v1/regional-compliance/slovak/gdpr/consent | record_gdpr_consent | stub | none | Mock: Uuid::new_v4(), no persistence. |
| GET | /api/v1/regional-compliance/slovak/gdpr/consent/status | get_gdpr_consent_status | stub | none | Hardcoded mock status. |
| POST | /api/v1/regional-compliance/slovak/gdpr/consent/withdraw | withdraw_gdpr_consent | stub | none | Mock: Uuid::new_v4(), no persistence. |
| POST | /api/v1/regional-compliance/czech/svj/config | configure_czech_svj | stub | none | Mock: Uuid::new_v4(), no DB. |
| GET | /api/v1/regional-compliance/czech/svj/config/{building_id} | get_czech_svj_config | stub | none | Hardcoded mock config. |
| POST | /api/v1/regional-compliance/czech/svj/validate | validate_czech_vote | stub | none | Hardcoded participation/approval. |
| GET | /api/v1/regional-compliance/czech/svj/usneseni/{vote_id} | get_czech_usneseni | stub | none | Fully hardcoded usneseni. |
| GET | /api/v1/regional-compliance/status | get_compliance_status | stub | none | Hardcoded mock status. |

## aml_dsa/  (mount: /api/v1/aml-dsa)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/aml-dsa/aml/assess | create_aml_assessment | partial | aml_dsa_authz_pap60_tests.rs | Real (edd_repo.create_aml_assessment). Authz-only test (403); no success path. |
| GET | /api/v1/aml-dsa/aml/assessments | list_aml_assessments | done | compliance_wave1b_tests.rs | 403 for non-compliance role + 200 with assessments/total shape. |
| GET | /api/v1/aml-dsa/aml/assessments/{id} | get_aml_assessment | done | compliance_wave1b_tests.rs | 403 role gate + 404 unknown + 200 with full body shape. |
| POST | /api/v1/aml-dsa/aml/assessments/{id}/review | review_aml_assessment | done | aml_dsa_audit_logging_tests.rs | Real; HTTP test asserts 200 + audit row. |
| GET | /api/v1/aml-dsa/aml/country-risks | get_country_risks | done | compliance_wave1b_tests.rs | 403 role gate + 200 returns array. |
| GET | /api/v1/aml-dsa/aml/thresholds | get_aml_thresholds | done | compliance_wave1b_tests.rs | 403 role gate + 200 with threshold values (statutory €10k). |
| POST | /api/v1/aml-dsa/edd | initiate_edd | done | compliance_wave1b_tests.rs | 403 role gate + 200 creates record with correct body shape. |
| GET | /api/v1/aml-dsa/edd/{id} | get_edd_record | done | compliance_wave1b_tests.rs | 403 role gate + 404 unknown + 200 with documents_received/compliance_notes arrays. |
| POST | /api/v1/aml-dsa/edd/{id}/documents | upload_edd_document | done | compliance_wave1b_tests.rs | 403 role gate + 200 with doc shape (type/verification_status). |
| POST | /api/v1/aml-dsa/edd/{id}/documents/{doc_id}/verify | verify_edd_document | done | compliance_wave1b_tests.rs | 403 role gate + 200 transitions status to verified. |
| POST | /api/v1/aml-dsa/edd/{id}/notes | add_edd_note | done | compliance_wave1b_tests.rs | 403 role gate + 200 + DB side-effect (JSONB array persisted). |
| POST | /api/v1/aml-dsa/edd/{id}/complete | complete_edd | done | compliance_wave1b_tests.rs | 403 role gate + 200 with status=completed + DB completed_at set. |
| GET | /api/v1/aml-dsa/edd/pending | list_pending_edd | done | compliance_wave1b_tests.rs | 403 role gate + 200 with org-scoped array (cross-org leak check). |
| GET | /api/v1/aml-dsa/dsa/reports | list_dsa_reports | done | compliance_wave1b_tests.rs | Non-platform users get 403 (principal_kind guard). |
| POST | /api/v1/aml-dsa/dsa/reports | generate_dsa_report | done | compliance_wave1b_tests.rs | Non-platform users get 403 (principal_kind guard). |
| GET | /api/v1/aml-dsa/dsa/reports/{id} | get_dsa_report | done | compliance_wave1b_tests.rs | Non-platform users get 403 (principal_kind guard). |
| POST | /api/v1/aml-dsa/dsa/reports/{id}/publish | publish_dsa_report | done | compliance_wave1b_tests.rs | Non-platform users get 403 (principal_kind guard). |
| GET | /api/v1/aml-dsa/dsa/reports/{id}/download | download_dsa_report | partial | dsa_report_download_tests.rs | Real; HTTP tests drive 401/403/404 + presigner-reached 503, but no 200 (no storage in CI). |
| GET | /api/v1/aml-dsa/dsa/metrics | get_dsa_metrics | done | compliance_wave1b_tests.rs | Non-platform users get 403 (principal_kind guard). |
| GET | /api/v1/aml-dsa/moderation/queue | get_moderation_queue | done | compliance_wave1b_tests.rs | 403 non-moderator + 200 with cases array (manager role). |
| GET | /api/v1/aml-dsa/moderation/queue/stats | get_moderation_stats | done | compliance_wave1b_tests.rs | 403 non-moderator + 200 with pending_count/under_review_count. |
| GET | /api/v1/aml-dsa/moderation/cases/{id} | get_moderation_case | done | compliance_wave1b_tests.rs | 403 non-moderator + 200 with id/status/priority shape. |
| POST | /api/v1/aml-dsa/moderation/cases/{id}/assign | assign_moderation_case | done | compliance_wave1b_tests.rs | 403 non-moderator role gate confirmed. |
| POST | /api/v1/aml-dsa/moderation/cases/{id}/action | take_moderation_action | done | aml_dsa_audit_logging_tests.rs | Real; HTTP test asserts 200 + audit row. |
| POST | /api/v1/aml-dsa/moderation/cases/{id}/appeal | file_appeal | partial | aml_dsa_authz_pap60_tests.rs | Real (ownership-scoped). Only cross-tenant 404 HTTP test; no 200. |
| POST | /api/v1/aml-dsa/moderation/cases/{id}/appeal/decide | decide_appeal | done | compliance_wave1b_tests.rs | 403 non-moderator role gate confirmed. |
| POST | /api/v1/aml-dsa/moderation/report | report_content | partial | aml_dsa_authz_pap60_tests.rs | Real (resolve_content_owner + create_moderation_case). Only repo-level helper tested; no handler success test. |
| GET | /api/v1/aml-dsa/moderation/templates | get_action_templates | done | compliance_wave1b_tests.rs | 403 non-moderator + 200 with template array. |

## enhanced_tenant_screening/  (mount: /api/v1/tenant-screening)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/tenant-screening/models | list_risk_models | partial | none | Real (RLS conn + repo). No HTTP test. |
| POST | /api/v1/tenant-screening/models | create_risk_model | partial | enhanced_screening_cross_org_idor_tests.rs | Real. Repo tested directly, not via handler. |
| GET | /api/v1/tenant-screening/models/{id} | get_risk_model | partial | enhanced_screening_cross_org_idor_tests.rs | Real. Repo tested directly (pos+neg), not via handler. |
| DELETE | /api/v1/tenant-screening/models/{id} | delete_risk_model | partial | none | Real (204/404). No test. |
| POST | /api/v1/tenant-screening/models/{id}/activate | activate_risk_model | partial | none | Real. No test. |
| GET | /api/v1/tenant-screening/providers | list_provider_configs | partial | none | Real. No test. |
| POST | /api/v1/tenant-screening/providers | create_provider_config | partial | enhanced_screening_cross_org_idor_tests.rs | Real. Repo tested directly, not via handler. |
| GET | /api/v1/tenant-screening/providers/{id} | get_provider_config | partial | enhanced_screening_cross_org_idor_tests.rs | Real. Repo tested directly (pos+neg), not via handler. |
| DELETE | /api/v1/tenant-screening/providers/{id} | delete_provider_config | partial | none | Real (204/404). No test. |
| PUT | /api/v1/tenant-screening/providers/{id}/status | update_provider_status | partial | none | Real (typed body). No test. |
| GET | /api/v1/tenant-screening/results | list_ai_results | partial | none | Real (paginated). No test. |
| GET | /api/v1/tenant-screening/results/{screening_id} | get_ai_result | partial | enhanced_screening_cross_org_idor_tests.rs | Real. Repo tested directly, not via handler. |
| GET | /api/v1/tenant-screening/results/{screening_id}/factors | get_risk_factors | partial | none | Real (2-step). No test. |
| GET | /api/v1/tenant-screening/results/{screening_id}/complete | get_complete_screening_data | partial | enhanced_screening_cross_org_idor_tests.rs | Real. Repo tested directly (pos+neg), not via handler. |
| POST | /api/v1/tenant-screening/score | run_ai_scoring | partial | none | Real scoring + persists AI result (rental/income/employment/reference subscores None pending integrations). No test. |
| GET | /api/v1/tenant-screening/credit/{screening_id} | get_credit_result | partial | none | Real. No test. |
| POST | /api/v1/tenant-screening/credit | create_credit_result | partial | none | Real. No test. |
| GET | /api/v1/tenant-screening/background/{screening_id} | get_background_result | partial | none | Real. No test. |
| POST | /api/v1/tenant-screening/background | create_background_result | partial | none | Real. No test. |
| GET | /api/v1/tenant-screening/eviction/{screening_id} | get_eviction_result | partial | none | Real. No test. |
| POST | /api/v1/tenant-screening/eviction | create_eviction_result | partial | none | Real. No test. |
| GET | /api/v1/tenant-screening/queue | get_pending_queue | partial | none | Real (limit 50). No test. |
| POST | /api/v1/tenant-screening/queue | create_queue_item | partial | none | Real. No test. |
| PUT | /api/v1/tenant-screening/queue/{id}/status | update_queue_status | partial | none | Real. No test. |
| GET | /api/v1/tenant-screening/reports/{screening_id} | get_reports | partial | none | Real. No test. |
| POST | /api/v1/tenant-screening/reports | create_report | partial | none | Real. No test. |
| GET | /api/v1/tenant-screening/statistics | get_statistics | partial | none | Real. No test (faults get_statistics is unrelated). |
| GET | /api/v1/tenant-screening/distribution | get_risk_distribution | partial | none | Real. No test. |

## Summary
- done: 28 | partial: 37 | stub: 20 | missing: 0 | total: 85
