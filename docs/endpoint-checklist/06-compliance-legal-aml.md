# Compliance, Legal, AML/DSA & Insurance endpoints

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `POST /api/v1/gdpr/export/request` | `gdpr.rs:request_data_export` | partial | — | repo-backed, no test |
| `GET /api/v1/gdpr/export/status/{request_id}` | `gdpr.rs:get_export_status` | partial | — | repo-backed, no test |
| `GET /api/v1/gdpr/export/download/{token}` | `gdpr.rs:download_export` | partial | — | repo-backed, no test |
| `GET /api/v1/gdpr/export/categories` | `gdpr.rs:get_export_categories` | partial | — | static catalog response |
| `GET /api/v1/gdpr/export/history` | `gdpr.rs:get_export_history` | partial | — | repo-backed, no test |
| `POST /api/v1/gdpr/deletion/request` | `gdpr.rs:request_data_deletion` | partial | — | repo-backed, no test |
| `GET /api/v1/gdpr/deletion/status` | `gdpr.rs:get_deletion_status` | partial | — | repo-backed, no test |
| `POST /api/v1/gdpr/deletion/cancel` | `gdpr.rs:cancel_deletion_request` | partial | — | repo-backed, no test |
| `GET /api/v1/gdpr/privacy` | `gdpr.rs:get_privacy_settings` | partial | — | repo-backed, no test |
| `POST /api/v1/gdpr/privacy` | `gdpr.rs:update_privacy_settings` | partial | — | repo-backed, no test |
| `GET /api/v1/compliance/audit-logs` | `compliance.rs:get_audit_logs` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/audit-logs/summary` | `compliance.rs:get_audit_summary` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/audit-logs/user/{user_id}` | `compliance.rs:get_user_audit_logs` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/audit-logs/integrity` | `compliance.rs:verify_audit_integrity` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/gdpr/data-exports` | `compliance.rs:get_data_export_report` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/gdpr/deletion-requests` | `compliance.rs:get_deletion_requests_report` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/gdpr/privacy-report` | `compliance.rs:get_privacy_settings_report` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/security/login-activity` | `compliance.rs:get_login_activity_report` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/security/mfa-status` | `compliance.rs:get_mfa_status_report` | partial | — | sqlx-backed, no test |
| `GET /api/v1/compliance/security/failed-logins` | `compliance.rs:get_failed_logins_report` | partial | — | sqlx-backed, no test |
| `GET /api/v1/regional-compliance/jurisdiction` | `regional_compliance.rs:get_jurisdiction` | stub | — | hardcoded, ignores state |
| `PUT /api/v1/regional-compliance/jurisdiction` | `regional_compliance.rs:set_jurisdiction` | stub | — | echoes input, no persist |
| `POST /api/v1/regional-compliance/slovak/voting/config` | `regional_compliance.rs:configure_slovak_voting` | stub | — | fabricated response |
| `GET /api/v1/regional-compliance/slovak/voting/config/{building_id}` | `regional_compliance.rs:get_slovak_voting_config` | stub | — | hardcoded response |
| `POST /api/v1/regional-compliance/slovak/voting/validate` | `regional_compliance.rs:validate_slovak_vote` | stub | — | hardcoded participation values |
| `GET /api/v1/regional-compliance/slovak/voting/minutes/{vote_id}` | `regional_compliance.rs:get_slovak_vote_minutes` | stub | — | hardcoded minutes |
| `POST /api/v1/regional-compliance/slovak/accounting/config` | `regional_compliance.rs:configure_slovak_accounting` | stub | — | fabricated response |
| `GET /api/v1/regional-compliance/slovak/accounting/config` | `regional_compliance.rs:get_slovak_accounting_config` | stub | — | hardcoded response |
| `POST /api/v1/regional-compliance/slovak/accounting/export` | `regional_compliance.rs:export_slovak_accounting` | stub | — | fabricated response |
| `POST /api/v1/regional-compliance/slovak/gdpr/config` | `regional_compliance.rs:configure_slovak_gdpr` | stub | — | fabricated response |
| `GET /api/v1/regional-compliance/slovak/gdpr/config` | `regional_compliance.rs:get_slovak_gdpr_config` | stub | — | hardcoded response |
| `POST /api/v1/regional-compliance/slovak/gdpr/consent` | `regional_compliance.rs:record_gdpr_consent` | stub | — | fabricated response |
| `GET /api/v1/regional-compliance/slovak/gdpr/consent/status` | `regional_compliance.rs:get_gdpr_consent_status` | stub | — | hardcoded response |
| `POST /api/v1/regional-compliance/slovak/gdpr/consent/withdraw` | `regional_compliance.rs:withdraw_gdpr_consent` | stub | — | fabricated response |
| `POST /api/v1/regional-compliance/czech/svj/config` | `regional_compliance.rs:configure_czech_svj` | stub | — | fabricated response |
| `GET /api/v1/regional-compliance/czech/svj/config/{building_id}` | `regional_compliance.rs:get_czech_svj_config` | stub | — | hardcoded response |
| `POST /api/v1/regional-compliance/czech/svj/validate` | `regional_compliance.rs:validate_czech_vote` | stub | — | hardcoded validation |
| `GET /api/v1/regional-compliance/czech/svj/usneseni/{vote_id}` | `regional_compliance.rs:get_czech_usneseni` | stub | — | hardcoded response |
| `GET /api/v1/regional-compliance/status` | `regional_compliance.rs:get_compliance_status` | stub | — | hardcoded response |
| `GET /api/v1/data-residency/config` | `data_residency.rs:get_residency_config` | done | `data_residency_tests.rs` | DB-backed config |
| `POST /api/v1/data-residency/config` | `data_residency.rs:configure_residency` | done | `data_residency_tests.rs` | DB-backed config |
| `PUT /api/v1/data-residency/config` | `data_residency.rs:update_residency_config` | done | `data_residency_tests.rs` | DB-backed config |
| `GET /api/v1/data-residency/regions` | `data_residency.rs:list_available_regions` | done | `data_residency_tests.rs` | DB-backed regions |
| `GET /api/v1/data-residency/routing/status` | `data_residency.rs:get_routing_status` | done | `data_residency_tests.rs` | DB-backed routing status |
| `POST /api/v1/data-residency/routing/log-access` | `data_residency.rs:log_cross_region_access` | done | `data_residency_tests.rs` | DB-backed log access |
| `GET /api/v1/data-residency/routing/access-logs` | `data_residency.rs:list_access_logs` | done | `data_residency_tests.rs` | DB-backed access logs |
| `POST /api/v1/data-residency/compliance/verify` | `data_residency.rs:run_compliance_verification` | done | `data_residency_tests.rs` | DB-backed compliance verification |
| `GET /api/v1/data-residency/compliance/verification/{id}` | `data_residency.rs:get_verification_result` | done | `data_residency_tests.rs` | DB-backed verification results |
| `GET /api/v1/data-residency/compliance/export` | `data_residency.rs:export_compliance_report` | done | `data_residency_tests.rs` | DB-backed compliance report export |
| `GET /api/v1/data-residency/audit` | `data_residency.rs:list_audit_logs` | done | `data_residency_tests.rs` | DB-backed audit logs |
| `GET /api/v1/data-residency/audit/{id}` | `data_residency.rs:get_audit_entry` | done | `data_residency_tests.rs` | DB-backed audit logs |
| `POST /api/v1/data-residency/audit/verify-chain` | `data_residency.rs:verify_audit_chain` | done | `data_residency_tests.rs` | DB-backed tamper-evident chain verification |
| `GET /api/v1/data-residency/dashboard` | `data_residency.rs:get_residency_dashboard` | done | `data_residency_tests.rs` | DB-backed dashboard |
| `POST /api/v1/aml-dsa/aml/assess` | `aml_dsa/aml.rs:create_aml_assessment` | done | `aml_dsa_authz_pap60_tests.rs` | |
| `GET /api/v1/aml-dsa/aml/assessments` | `aml_dsa/aml.rs:list_aml_assessments` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/aml/assessments/{id}` | `aml_dsa/aml.rs:get_aml_assessment` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/aml/assessments/{id}/review` | `aml_dsa/aml.rs:review_aml_assessment` | done | `aml_dsa_audit_logging_tests.rs` | |
| `GET /api/v1/aml-dsa/aml/country-risks` | `aml_dsa/aml.rs:get_country_risks` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/aml/thresholds` | `aml_dsa/aml.rs:get_aml_thresholds` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/edd` | `aml_dsa/edd.rs:initiate_edd` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/edd/{id}` | `aml_dsa/edd.rs:get_edd_record` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/edd/{id}/documents` | `aml_dsa/edd.rs:upload_edd_document` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/edd/{id}/documents/{doc_id}/verify` | `aml_dsa/edd.rs:verify_edd_document` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/edd/{id}/notes` | `aml_dsa/edd.rs:add_edd_note` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/edd/{id}/complete` | `aml_dsa/edd.rs:complete_edd` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/edd/pending` | `aml_dsa/edd.rs:list_pending_edd` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/dsa/reports` | `aml_dsa/dsa.rs:list_dsa_reports` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/dsa/reports` | `aml_dsa/dsa.rs:generate_dsa_report` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/dsa/reports/{id}` | `aml_dsa/dsa.rs:get_dsa_report` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/dsa/reports/{id}/publish` | `aml_dsa/dsa.rs:publish_dsa_report` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/dsa/reports/{id}/download` | `aml_dsa/dsa.rs:download_dsa_report` | done | `dsa_report_download_tests.rs` | |
| `GET /api/v1/aml-dsa/dsa/metrics` | `aml_dsa/dsa.rs:get_dsa_metrics` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/moderation/queue` | `aml_dsa/moderation.rs:get_moderation_queue` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/moderation/queue/stats` | `aml_dsa/moderation.rs:get_moderation_stats` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/moderation/cases/{id}` | `aml_dsa/moderation.rs:get_moderation_case` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/moderation/cases/{id}/assign` | `aml_dsa/moderation.rs:assign_moderation_case` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/moderation/cases/{id}/action` | `aml_dsa/moderation.rs:take_moderation_action` | done | `aml_dsa_audit_logging_tests.rs` | |
| `POST /api/v1/aml-dsa/moderation/cases/{id}/appeal` | `aml_dsa/moderation.rs:file_appeal` | done | `aml_dsa_authz_pap60_tests.rs` | |
| `POST /api/v1/aml-dsa/moderation/cases/{id}/appeal/decide` | `aml_dsa/moderation.rs:decide_appeal` | partial | — | repo-backed, no test |
| `POST /api/v1/aml-dsa/moderation/report` | `aml_dsa/moderation.rs:report_content` | partial | — | repo-backed, no test |
| `GET /api/v1/aml-dsa/moderation/templates` | `aml_dsa/moderation.rs:get_action_templates` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/documents` | `legal.rs:create_document` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/documents` | `legal.rs:list_documents` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/documents/summary` | `legal.rs:list_documents_summary` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/documents/{id}` | `legal.rs:get_document` | done | `legal_cross_org_idor_tests.rs` | |
| `PATCH /api/v1/legal/documents/{id}` | `legal.rs:update_document` | done | `legal_cross_org_idor_tests.rs` | |
| `DELETE /api/v1/legal/documents/{id}` | `legal.rs:delete_document` | done | `legal_cross_org_idor_tests.rs` | |
| `POST /api/v1/legal/documents/{id}/versions` | `legal.rs:add_version` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/documents/{id}/versions` | `legal.rs:list_versions` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/documents/{id}/versions/{version}` | `legal.rs:get_version` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/requirements` | `legal.rs:create_requirement` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/requirements` | `legal.rs:list_requirements` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/requirements/with-details` | `legal.rs:list_requirements_with_details` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/requirements/statistics` | `legal.rs:get_compliance_statistics` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/requirements/{id}` | `legal.rs:get_requirement` | partial | — | repo-backed, no test |
| `PATCH /api/v1/legal/requirements/{id}` | `legal.rs:update_requirement` | partial | — | repo-backed, no test |
| `DELETE /api/v1/legal/requirements/{id}` | `legal.rs:delete_requirement` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/requirements/{id}/verify` | `legal.rs:create_verification` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/requirements/{id}/verifications` | `legal.rs:list_verifications` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/notices` | `legal.rs:create_notice` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/notices` | `legal.rs:list_notices` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/notices/with-recipients` | `legal.rs:list_notices_with_recipients` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/notices/statistics` | `legal.rs:get_notice_statistics` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/notices/{id}` | `legal.rs:get_notice` | partial | — | repo-backed, no test |
| `PATCH /api/v1/legal/notices/{id}` | `legal.rs:update_notice` | partial | — | repo-backed, no test |
| `DELETE /api/v1/legal/notices/{id}` | `legal.rs:delete_notice` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/notices/{id}/send` | `legal.rs:send_notice` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/notices/{id}/recipients` | `legal.rs:list_recipients` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/notices/{notice_id}/acknowledge/{recipient_id}` | `legal.rs:acknowledge_notice` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/templates` | `legal.rs:create_template` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/templates` | `legal.rs:list_templates` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/templates/{id}` | `legal.rs:get_template` | partial | — | repo-backed, no test |
| `PATCH /api/v1/legal/templates/{id}` | `legal.rs:update_template` | partial | — | repo-backed, no test |
| `DELETE /api/v1/legal/templates/{id}` | `legal.rs:delete_template` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/templates/apply` | `legal.rs:apply_template` | partial | — | repo-backed, no test |
| `GET /api/v1/legal/audit-trail` | `legal.rs:list_audit_trail` | partial | — | repo-backed, no test |
| `POST /api/v1/legal/audit-trail` | `legal.rs:create_audit_entry` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/policies` | `insurance.rs:list_policies` | done | `insurance_cross_tenant_idor_tests.rs` | |
| `POST /api/v1/insurance/policies` | `insurance.rs:create_policy` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/policies/{policy_id}` | `insurance.rs:get_policy` | done | `insurance_cross_tenant_idor_tests.rs` | |
| `PUT /api/v1/insurance/policies/{policy_id}` | `insurance.rs:update_policy` | partial | — | repo-backed, no test |
| `DELETE /api/v1/insurance/policies/{policy_id}` | `insurance.rs:delete_policy` | done | `insurance_cross_tenant_idor_tests.rs` | |
| `GET /api/v1/insurance/policies/expiring` | `insurance.rs:get_expiring_policies` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/policies/{policy_id}/documents` | `insurance.rs:list_policy_documents` | partial | — | repo-backed, no test |
| `POST /api/v1/insurance/policies/{policy_id}/documents` | `insurance.rs:add_policy_document` | partial | — | repo-backed, no test |
| `DELETE /api/v1/insurance/policies/{policy_id}/documents/{document_id}` | `insurance.rs:remove_policy_document` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/policies/{policy_id}/reminders` | `insurance.rs:list_reminders` | partial | — | repo-backed, no test |
| `POST /api/v1/insurance/policies/{policy_id}/reminders` | `insurance.rs:create_reminder` | partial | — | repo-backed, no test |
| `PUT /api/v1/insurance/reminders/{reminder_id}` | `insurance.rs:update_reminder` | partial | — | repo-backed, no test |
| `DELETE /api/v1/insurance/reminders/{reminder_id}` | `insurance.rs:delete_reminder` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/claims` | `insurance.rs:list_claims` | done | `insurance_cross_tenant_idor_tests.rs` | |
| `POST /api/v1/insurance/claims` | `insurance.rs:create_claim` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/claims/{claim_id}` | `insurance.rs:get_claim` | partial | — | repo-backed, no test |
| `PUT /api/v1/insurance/claims/{claim_id}` | `insurance.rs:update_claim` | partial | — | repo-backed, no test |
| `DELETE /api/v1/insurance/claims/{claim_id}` | `insurance.rs:delete_claim` | partial | — | repo-backed, no test |
| `POST /api/v1/insurance/claims/{claim_id}/submit` | `insurance.rs:submit_claim` | partial | — | repo-backed, no test |
| `POST /api/v1/insurance/claims/{claim_id}/review` | `insurance.rs:review_claim` | partial | — | repo-backed, no test |
| `POST /api/v1/insurance/claims/{claim_id}/payment` | `insurance.rs:record_claim_payment` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/claims/{claim_id}/history` | `insurance.rs:get_claim_history` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/claims/{claim_id}/documents` | `insurance.rs:list_claim_documents` | partial | — | repo-backed, no test |
| `POST /api/v1/insurance/claims/{claim_id}/documents` | `insurance.rs:add_claim_document` | partial | — | repo-backed, no test |
| `DELETE /api/v1/insurance/claims/{claim_id}/documents/{document_id}` | `insurance.rs:remove_claim_document` | partial | — | repo-backed, no test |
| `GET /api/v1/insurance/statistics` | `insurance.rs:get_statistics` | partial | — | repo-backed, no test |

## Tally
done: 26  partial: 98  stub: 19  missing: 0  total: 143
