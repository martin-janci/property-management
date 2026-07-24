# Documents & Forms

_Server: api-server. Modules: signatures.rs, templates.rs, legal.rs, lease_abstraction.rs, documents/ (core, versions, folders, shares, intelligence), forms/ (crud, fields, submissions)._

## documents/core.rs  (mount: /api/v1/documents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/documents/upload | upload_document | done | document_upload_tests.rs | CREATED happy path; 50 MiB body limit sub-router |
| POST | /api/v1/documents | create_document | done | documents_core_crud_tests.rs | CREATED happy path (create_document_succeeds) |
| GET | /api/v1/documents | list_documents | done | documents_core_crud_tests.rs | OK happy path (list_documents_succeeds) |
| GET | /api/v1/documents/{id} | get_document | done | document_upload_tests.rs | GET-back after upload asserts 200 |
| PUT | /api/v1/documents/{id} | update_document | done | documents_core_crud_tests.rs | OK happy path (update_document_succeeds) |
| DELETE | /api/v1/documents/{id} | delete_document | done | documents_core_crud_tests.rs | NO_CONTENT + soft-delete verified (delete_document_succeeds) |
| POST | /api/v1/documents/{id}/move | move_document | done | document_folder_tests.rs | OK happy path (line 865) |
| PUT | /api/v1/documents/{id}/access | update_document_access | done | documents_core_crud_tests.rs | OK happy path (update_document_access_succeeds) |
| GET | /api/v1/documents/{id}/download | get_download_url | done | document_download_preview_tests.rs | OK happy path (line 870) + auth/IDOR |
| GET | /api/v1/documents/{id}/preview | get_preview_url | done | document_download_preview_tests.rs | OK happy path (line 887) + auth/IDOR |

## documents/versions.rs  (mount: /api/v1/documents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/documents/{id}/versions | get_version_history | done | documents_core_crud_tests.rs | OK happy path (get_version_history_succeeds) |
| POST | /api/v1/documents/{id}/versions | create_version | done | documents_core_crud_tests.rs | CREATED happy path (create_version_succeeds) |
| GET | /api/v1/documents/{id}/versions/{version_id} | get_version | done | documents_core_crud_tests.rs | OK happy path (get_version_succeeds) |
| POST | /api/v1/documents/{id}/versions/{version_id}/restore | restore_version | done | documents_core_crud_tests.rs | CREATED happy path (restore_version_succeeds) |

## documents/folders.rs  (mount: /api/v1/documents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/documents/folders | list_folders | done | document_folder_tests.rs | OK happy path (list_folders_manager_succeeds) |
| POST | /api/v1/documents/folders | create_folder | done | document_folder_tests.rs | CREATED happy path (test_create_folder_manager_succeeds) |
| GET | /api/v1/documents/folders/tree | get_folder_tree | done | document_folder_tests.rs | OK happy path (get_folder_tree_manager_succeeds) |
| GET | /api/v1/documents/folders/{id} | get_folder | done | document_folder_tests.rs | OK happy path (get_folder_manager_succeeds) |
| PUT | /api/v1/documents/folders/{id} | update_folder | done | document_folder_tests.rs | OK happy path (lines 1039/1086) |
| DELETE | /api/v1/documents/folders/{id} | delete_folder | done | document_folder_tests.rs | NO_CONTENT + soft-delete verified (delete_folder_manager_succeeds) |

## documents/shares.rs  (mount: /api/v1/documents authenticated_router + public_router merged at root via documents::public_router())
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/documents/{id}/shares | list_shares | done | documents_core_crud_tests.rs | OK happy path (list_shares_succeeds) |
| POST | /api/v1/documents/{id}/shares | create_share | done | documents_core_crud_tests.rs | CREATED happy path (create_share_succeeds) |
| DELETE | /api/v1/documents/{id}/shares/{share_id} | revoke_share | done | documents_core_crud_tests.rs | NO_CONTENT + revoked_at verified (revoke_share_succeeds) |
| GET | /shared/{token} | access_shared_document | done | document_share_access_tests.rs | OK happy path; public (no-auth), merged at root not under /api/v1 |
| POST | /shared/{token}/access | access_protected_share | done | document_share_access_tests.rs | OK happy path; public_router merged in lib.rs |

## documents/intelligence.rs  (mount: /api/v1/documents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/documents/{id}/ocr/reprocess | reprocess_ocr | done | documents_intelligence_templates_tests.rs | OK happy path (enqueues OCR) |
| POST | /api/v1/documents/search | search_documents | done | documents_intelligence_templates_tests.rs | OK happy path asserting results array |
| GET | /api/v1/documents/{id}/classification | get_classification | done | documents_intelligence_templates_tests.rs | OK happy path |
| POST | /api/v1/documents/{id}/classification/feedback | submit_classification_feedback | done | documents_intelligence_templates_tests.rs | OK happy path (accepted=true) |
| GET | /api/v1/documents/{id}/classification/history | get_classification_history | done | documents_intelligence_templates_tests.rs | OK happy path asserting history array |
| POST | /api/v1/documents/{id}/summarize | request_summarization | done | documents_intelligence_templates_tests.rs | OK happy path (enqueues summarization) |
| POST | /api/v1/documents/{id}/ai-summarize | ai_summarize_document | partial | — | not unit-testable: live LLM inference + S3 text extraction, needs external creds |
| GET | /api/v1/documents/intelligence/stats | get_intelligence_stats | done | documents_intelligence_templates_tests.rs | OK happy path asserting stats array |

## templates.rs  (mount: /api/v1/templates)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/templates | create_template | done | documents_intelligence_templates_tests.rs | CREATED happy path asserting id |
| GET | /api/v1/templates | list_templates | done | documents_intelligence_templates_tests.rs | OK happy path asserting templates array |
| GET | /api/v1/templates/{id} | get_template | done | documents_intelligence_templates_tests.rs | OK happy path asserting template.id |
| PUT | /api/v1/templates/{id} | update_template | done | documents_intelligence_templates_tests.rs | OK happy path asserting updated name |
| DELETE | /api/v1/templates/{id} | delete_template | done | documents_intelligence_templates_tests.rs | NO_CONTENT + follow-up GET returns 404 |
| POST | /api/v1/templates/{id}/generate | generate_document | done | documents_intelligence_templates_tests.rs | CREATED happy path asserting document_id |

## signatures.rs  (mount: /api/v1/signature-requests router; /api/v1/documents/{id}/signature-requests document_signature_router; /api/v1/signatures public_sign_router)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/documents/{id}/signature-requests | list_signature_requests | done | signature_request_happy_path_tests.rs | BIT-313: re-mounted as a document sub-resource (was unreachable at bare "/"); OK happy path asserts total + document_id |
| POST | /api/v1/documents/{id}/signature-requests | create_signature_request | done | signature_request_happy_path_tests.rs | BIT-313: re-mounted as a document sub-resource (was unreachable at bare "/"); CREATED happy path asserts signature_request.id + document_id |
| GET | /api/v1/signature-requests/{id} | get_signature_request | done | esignature_email_status_tracking_tests.rs | OK happy path asserting signer_counts |
| POST | /api/v1/signature-requests/{id}/remind | send_reminder | partial | — | no test |
| POST | /api/v1/signature-requests/{id}/cancel | cancel_signature_request | partial | — | no test |
| POST | /api/v1/signature-requests/webhook/{provider} | handle_webhook | done | esignature_webhook_idempotency_tests.rs, esignature_email_status_tracking_tests.rs | OK happy path + idempotency |
| GET | /api/v1/signatures/sign | get_sign_context | done | esignature_sign_consumer_tests.rs | OK happy path (render context) + tamper/replay |
| POST | /api/v1/signatures/sign | submit_signature | done | esignature_sign_consumer_tests.rs | OK happy path (record signature) + CONFLICT/GONE |

## legal.rs  (mount: /api/v1/legal)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/legal/documents | create_document | done | legal_insurance_wave1b_tests.rs | 201 with id/title/document_type |
| GET | /api/v1/legal/documents | list_documents | done | legal_insurance_wave1b_tests.rs | 200 JSON array |
| GET | /api/v1/legal/documents/summary | list_documents_summary | partial | — | no test |
| GET | /api/v1/legal/documents/{id} | get_document | done | legal_cross_org_idor_tests.rs, legal_insurance_wave1b_tests.rs | same-org 200 + cross-org 404 |
| PATCH | /api/v1/legal/documents/{id} | update_document | done | legal_insurance_wave1b_tests.rs | 200 title changed |
| DELETE | /api/v1/legal/documents/{id} | delete_document | done | legal_insurance_wave1b_tests.rs | 200 success=true + 404 re-read |
| POST | /api/v1/legal/documents/{id}/versions | add_version | done | legal_insurance_wave1b_tests.rs | 201 with document_id/version_number |
| GET | /api/v1/legal/documents/{id}/versions | list_versions | done | legal_insurance_wave1b_tests.rs | 200 JSON array |
| GET | /api/v1/legal/documents/{id}/versions/{version} | get_version | partial | — | no test |
| POST | /api/v1/legal/requirements | create_requirement | done | legal_insurance_wave1b_tests.rs | 201 with id/requirement_type |
| GET | /api/v1/legal/requirements | list_requirements | done | legal_insurance_wave1b_tests.rs | 200 JSON array |
| GET | /api/v1/legal/requirements/with-details | list_requirements_with_details | partial | — | no test |
| GET | /api/v1/legal/requirements/statistics | get_compliance_statistics | partial | — | no test |
| GET | /api/v1/legal/requirements/{id} | get_requirement | done | legal_insurance_wave1b_tests.rs | 200 same-org + 404 unknown |
| PATCH | /api/v1/legal/requirements/{id} | update_requirement | done | legal_insurance_wave1b_tests.rs | 200 title changed |
| DELETE | /api/v1/legal/requirements/{id} | delete_requirement | done | legal_insurance_wave1b_tests.rs | 200 success=true |
| POST | /api/v1/legal/requirements/{id}/verify | create_verification | done | legal_insurance_wave1b_tests.rs | 201 with id/requirement_id |
| GET | /api/v1/legal/requirements/{id}/verifications | list_verifications | done | legal_insurance_wave1b_tests.rs | 200 JSON array |
| POST | /api/v1/legal/notices | create_notice | partial | — | no test |
| GET | /api/v1/legal/notices | list_notices | partial | — | no test |
| GET | /api/v1/legal/notices/with-recipients | list_notices_with_recipients | partial | — | no test |
| GET | /api/v1/legal/notices/statistics | get_notice_statistics | partial | — | no test |
| GET | /api/v1/legal/notices/{id} | get_notice | partial | — | no test |
| PATCH | /api/v1/legal/notices/{id} | update_notice | partial | — | no test |
| DELETE | /api/v1/legal/notices/{id} | delete_notice | partial | — | no test |
| POST | /api/v1/legal/notices/{id}/send | send_notice | partial | — | no test |
| GET | /api/v1/legal/notices/{id}/recipients | list_recipients | partial | — | no test |
| POST | /api/v1/legal/notices/{notice_id}/acknowledge/{recipient_id} | acknowledge_notice | partial | — | no test |
| POST | /api/v1/legal/templates | create_template | partial | — | no test |
| GET | /api/v1/legal/templates | list_templates | partial | — | no test |
| GET | /api/v1/legal/templates/{id} | get_template | partial | — | no test |
| PATCH | /api/v1/legal/templates/{id} | update_template | partial | — | no test |
| DELETE | /api/v1/legal/templates/{id} | delete_template | partial | — | no test |
| POST | /api/v1/legal/templates/apply | apply_template | partial | — | no test |
| GET | /api/v1/legal/audit-trail | list_audit_trail | partial | — | no test |
| POST | /api/v1/legal/audit-trail | create_audit_entry | partial | — | no test |

## lease_abstraction.rs  (mount: /api/v1/lease-abstraction)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/lease-abstraction/documents | list_documents | partial | — | no test for this prefix |
| POST | /api/v1/lease-abstraction/documents | upload_document | partial | — | no test |
| GET | /api/v1/lease-abstraction/documents/{id} | get_document | partial | — | no test |
| DELETE | /api/v1/lease-abstraction/documents/{id} | delete_document | partial | — | no test |
| POST | /api/v1/lease-abstraction/documents/{id}/process | process_document | partial | — | no test |
| GET | /api/v1/lease-abstraction/documents/{id}/extractions | list_extractions | partial | — | no test |
| GET | /api/v1/lease-abstraction/extractions/{id} | get_extraction | partial | — | no test |
| GET | /api/v1/lease-abstraction/extractions/{id}/fields | get_extraction_fields | partial | — | no test |
| POST | /api/v1/lease-abstraction/extractions/{id}/approve | approve_extraction | partial | — | no test |
| POST | /api/v1/lease-abstraction/extractions/{id}/reject | reject_extraction | partial | — | no test |
| GET | /api/v1/lease-abstraction/extractions/{id}/corrections | list_corrections | partial | — | no test |
| POST | /api/v1/lease-abstraction/extractions/{id}/corrections | add_correction | partial | — | no test |
| POST | /api/v1/lease-abstraction/extractions/{id}/validate | validate_import | partial | — | no test |
| POST | /api/v1/lease-abstraction/extractions/{id}/import | import_to_lease | partial | — | no test |
| GET | /api/v1/lease-abstraction/imports | list_imports | partial | — | no test |
| GET | /api/v1/lease-abstraction/imports/{id} | get_import | partial | — | no test |

## forms/crud.rs  (mount: /api/v1/forms)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/forms | create_form | partial | — | no test |
| GET | /api/v1/forms | list_forms | partial | — | no test |
| GET | /api/v1/forms/available | list_available_forms | partial | — | no test |
| GET | /api/v1/forms/statistics | get_statistics | partial | — | no test |
| GET | /api/v1/forms/{id} | get_form | partial | — | no test |
| PUT | /api/v1/forms/{id} | update_form | partial | — | no test |
| DELETE | /api/v1/forms/{id} | delete_form | partial | — | no test |
| POST | /api/v1/forms/{id}/publish | publish_form | partial | — | no test |
| POST | /api/v1/forms/{id}/archive | archive_form | partial | — | no test |

## forms/fields.rs  (mount: /api/v1/forms)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/forms/{id}/fields | list_fields | partial | — | no test |
| POST | /api/v1/forms/{id}/fields | add_field | partial | — | no test |
| POST | /api/v1/forms/{id}/fields/reorder | reorder_fields | partial | — | no test |
| PUT | /api/v1/forms/{id}/fields/{field_id} | update_field | partial | — | no test |
| DELETE | /api/v1/forms/{id}/fields/{field_id} | delete_field | partial | — | no test |

## forms/submissions.rs  (mount: /api/v1/forms)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/forms/{id}/submit | submit_form | done | form_cross_org_idor_tests.rs | CREATED happy path |
| GET | /api/v1/forms/{id}/submissions | list_submissions | partial | — | no test |
| GET | /api/v1/forms/{id}/submissions/{submission_id} | get_submission | partial | — | no test |
| POST | /api/v1/forms/{id}/submissions/{submission_id}/review | review_submission | partial | — | no test |
| POST | /api/v1/forms/{id}/download | record_download | partial | form_cross_org_idor_tests.rs | only IDOR (404), no happy path |

## Summary
- done: 31 | partial: 87 | stub: 0 | missing: 0 | total: 118
