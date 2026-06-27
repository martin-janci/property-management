# Compliance & Screening

_Server: api-server. Modules: compliance.rs, regional_compliance.rs, aml_dsa/, enhanced_tenant_screening/._

## compliance.rs  (mount: /api/v1/compliance)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/compliance/audit-logs | get_audit_logs | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/audit-logs/summary | get_audit_summary | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/audit-logs/user/{user_id} | get_user_audit_logs | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/audit-logs/integrity | verify_audit_integrity | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/gdpr/data-exports | get_data_export_report | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/gdpr/deletion-requests | get_deletion_requests_report | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/gdpr/privacy-report | get_privacy_settings_report | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/security/login-activity | get_login_activity_report | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/security/mfa-status | get_mfa_status_report | done | gdpr_compliance_tests.rs | |
| GET | /api/v1/compliance/security/failed-logins | get_failed_logins_report | done | gdpr_compliance_tests.rs | |

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
| GET | /api/v1/aml-dsa/aml/assessments | list_aml_assessments | partial | none | Real (edd_repo.list_aml_assessments). No test. |
| GET | /api/v1/aml-dsa/aml/assessments/{id} | get_aml_assessment | partial | aml_dsa_cross_org_idor_tests.rs | Real. IDOR test hits repo directly, not handler. |
| POST | /api/v1/aml-dsa/aml/assessments/{id}/review | review_aml_assessment | done | aml_dsa_audit_logging_tests.rs | Real; HTTP test asserts 200 + audit row. |
| GET | /api/v1/aml-dsa/aml/country-risks | get_country_risks | partial | none | Real (edd_repo.list_country_risks). No test. |
| GET | /api/v1/aml-dsa/aml/thresholds | get_aml_thresholds | partial | none | Real handler; returns hardcoded constant thresholds (by design, not TODO). No test. |
| POST | /api/v1/aml-dsa/edd | initiate_edd | partial | none | Real (edd_repo.create_edd + audit). No test. |
| GET | /api/v1/aml-dsa/edd/{id} | get_edd_record | partial | aml_dsa_cross_org_idor_tests.rs | Real. IDOR test is repo-only. |
| POST | /api/v1/aml-dsa/edd/{id}/documents | upload_edd_document | partial | none | Real (validation + edd_repo.upload_edd_document). Handler path untested. |
| POST | /api/v1/aml-dsa/edd/{id}/documents/{doc_id}/verify | verify_edd_document | partial | none | Real (edd_repo.verify_edd_document + audit). No test. |
| POST | /api/v1/aml-dsa/edd/{id}/notes | add_edd_note | partial | none | Real (edd_repo.add_compliance_note). No test. |
| POST | /api/v1/aml-dsa/edd/{id}/complete | complete_edd | partial | none | Real (edd_repo.complete_edd + audit). No test. |
| GET | /api/v1/aml-dsa/edd/pending | list_pending_edd | partial | none | Real (edd_repo.list_pending_edd). No test. |
| GET | /api/v1/aml-dsa/dsa/reports | list_dsa_reports | partial | none | Real (compliance_repo.list_dsa_reports); breakdowns hardcoded empty. No test. |
| POST | /api/v1/aml-dsa/dsa/reports | generate_dsa_report | partial | none | Real (validate period + compliance_repo.create_dsa_report). No test. |
| GET | /api/v1/aml-dsa/dsa/reports/{id} | get_dsa_report | partial | none | Real (compliance_repo.get_dsa_report). No test. |
| POST | /api/v1/aml-dsa/dsa/reports/{id}/publish | publish_dsa_report | partial | none | Real (compliance_repo.publish_dsa_report + audit). No test. |
| GET | /api/v1/aml-dsa/dsa/reports/{id}/download | download_dsa_report | partial | dsa_report_download_tests.rs | Real; HTTP tests drive 401/403/404 + presigner-reached 503, but no 200 (no storage in CI). |
| GET | /api/v1/aml-dsa/dsa/metrics | get_dsa_metrics | partial | none | Real (compliance_repo.get_platform_moderation_queue_stats). No test. |
| GET | /api/v1/aml-dsa/moderation/queue | get_moderation_queue | partial | aml_dsa_cross_org_idor_tests.rs | Real; case data DB-backed (owner display fields placeholder). IDOR test is repo-only. |
| GET | /api/v1/aml-dsa/moderation/queue/stats | get_moderation_stats | partial | none | Real (compliance_repo.get_moderation_queue_stats). No test. |
| GET | /api/v1/aml-dsa/moderation/cases/{id} | get_moderation_case | partial | aml_dsa_cross_org_idor_tests.rs | Real. IDOR test is repo-only. |
| POST | /api/v1/aml-dsa/moderation/cases/{id}/assign | assign_moderation_case | partial | aml_dsa_cross_org_idor_tests.rs | Real (org-member check + assign). IDOR test is repo-only. |
| POST | /api/v1/aml-dsa/moderation/cases/{id}/action | take_moderation_action | done | aml_dsa_audit_logging_tests.rs | Real; HTTP test asserts 200 + audit row. |
| POST | /api/v1/aml-dsa/moderation/cases/{id}/appeal | file_appeal | partial | aml_dsa_authz_pap60_tests.rs | Real (ownership-scoped). Only cross-tenant 404 HTTP test; no 200. |
| POST | /api/v1/aml-dsa/moderation/cases/{id}/appeal/decide | decide_appeal | partial | aml_dsa_cross_org_idor_tests.rs | Real (compliance_repo.decide_appeal + audit). IDOR test is repo-only. |
| POST | /api/v1/aml-dsa/moderation/report | report_content | partial | aml_dsa_authz_pap60_tests.rs | Real (resolve_content_owner + create_moderation_case). Only repo-level helper tested; no handler success test. |
| GET | /api/v1/aml-dsa/moderation/templates | get_action_templates | partial | none | Real (compliance_repo.list_action_templates). No test. |

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
- done: 12 | partial: 54 | stub: 19 | missing: 0 | total: 85
