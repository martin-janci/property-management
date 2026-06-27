# Documents & Forms

_Server: api-server. Modules: signatures.rs, templates.rs, legal.rs, lease_abstraction.rs, documents/ (core, versions, folders, shares, intelligence), forms/ (crud, fields, submissions)._

## documents/core.rs  (mount: /api/v1/documents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/documents/upload | upload_document | done | document_upload_tests.rs | CREATED happy path; 50 MiB body limit sub-router |
| POST | /api/v1/documents | create_document | partial | — | no test hits POST / |
| GET | /api/v1/documents | list_documents | partial | — | no test hits GET / |
| GET | /api/v1/documents/{id} | get_document | done | document_upload_tests.rs | GET-back after upload asserts 200 |
| PUT | /api/v1/documents/{id} | update_document | partial | — | no happy-path test |
| DELETE | /api/v1/documents/{id} | delete_document | partial | — | no happy-path test |
| POST | /api/v1/documents/{id}/move | move_document | done | document_folder_tests.rs | OK happy path (line 865) |
| PUT | /api/v1/documents/{id}/access | update_document_access | partial | — | no happy-path test |
| GET | /api/v1/documents/{id}/download | get_download_url | done | document_download_preview_tests.rs | OK happy path (line 870) + auth/IDOR |
| GET | /api/v1/documents/{id}/preview | get_preview_url | done | document_download_preview_tests.rs | OK happy path (line 887) + auth/IDOR |

## documents/versions.rs  (mount: /api/v1/documents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/documents/{id}/versions | get_version_history | partial | — | no test |
| POST | /api/v1/documents/{id}/versions | create_version | partial | — | no test |
| GET | /api/v1/documents/{id}/versions/{version_id} | get_version | partial | — | no test |
| POST | /api/v1/documents/{id}/versions/{version_id}/restore | restore_version | partial | — | no test |

## documents/folders.rs  (mount: /api/v1/documents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/documents/folders | list_folders | partial | document_folder_tests.rs | only auth (401) assertion, no happy path |
| POST | /api/v1/documents/folders | create_folder | done | document_folder_tests.rs | CREATED happy path (test_create_folder_manager_succeeds) |
| GET | /api/v1/documents/folders/tree | get_folder_tree | partial | document_folder_tests.rs | only auth (401) assertion |
| GET | /api/v1/documents/folders/{id} | get_folder | partial | document_folder_tests.rs | only auth/IDOR (401/404) |
| PUT | /api/v1/documents/folders/{id} | update_folder | done | document_folder_tests.rs | OK happy path (lines 1039/1086) |
| DELETE | /api/v1/documents/folders/{id} | delete_folder | partial | document_folder_tests.rs | only auth/IDOR (401/404) |

## documents/shares.rs  (mount: /api/v1/documents authenticated_router + public_router merged at root via documents::public_router())
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/documents/{id}/shares | list_shares | partial | — | no test |
| POST | /api/v1/documents/{id}/shares | create_share | partial | — | no test |
| DELETE | /api/v1/documents/{id}/shares/{share_id} | revoke_share | partial | — | no test |
| GET | /shared/{token} | access_shared_document | done | document_share_access_tests.rs | OK happy path; public (no-auth), merged at root not under /api/v1 |
| POST | /shared/{token}/access | access_protected_share | done | document_share_access_tests.rs | OK happy path; public_router merged in lib.rs |

## documents/intelligence.rs  (mount: /api/v1/documents)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/documents/{id}/ocr/reprocess | reprocess_ocr | partial | — | no test |
| POST | /api/v1/documents/search | search_documents | partial | — | no test |
| GET | /api/v1/documents/{id}/classification | get_classification | partial | — | no test |
| POST | /api/v1/documents/{id}/classification/feedback | submit_classification_feedback | partial | — | no test |
| GET | /api/v1/documents/{id}/classification/history | get_classification_history | partial | — | no test |
| POST | /api/v1/documents/{id}/summarize | request_summarization | partial | — | no test |
| POST | /api/v1/documents/{id}/ai-summarize | ai_summarize_document | partial | — | no test |
| GET | /api/v1/documents/intelligence/stats | get_intelligence_stats | partial | — | no test |

## templates.rs  (mount: /api/v1/templates)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/templates | create_template | partial | — | no test (templates hits in grep are unrelated automation/ai/iot routes) |
| GET | /api/v1/templates | list_templates | partial | — | no test |
| GET | /api/v1/templates/{id} | get_template | partial | — | no test |
| PUT | /api/v1/templates/{id} | update_template | partial | — | no test |
| DELETE | /api/v1/templates/{id} | delete_template | partial | — | no test |
| POST | /api/v1/templates/{id}/generate | generate_document | partial | — | no test |

## signatures.rs  (mount: /api/v1/signature-requests router; /api/v1/signatures public_sign_router)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/signature-requests | list_signature_requests | partial | — | handlers take Path<document_id> but route has no {id} param → 422; no test |
| POST | /api/v1/signature-requests | create_signature_request | partial | — | same broken route; requests seeded via SQL |
| GET | /api/v1/signature-requests/{id} | get_signature_request | done | esignature_email_status_tracking_tests.rs, signatures_legal_docs_tests.rs | OK happy path asserting signer_counts |
| POST | /api/v1/signature-requests/{id}/remind | send_reminder | done | signatures_legal_docs_tests.rs | OK or 400 (email service unavailable in test env) |
| POST | /api/v1/signature-requests/{id}/cancel | cancel_signature_request | done | signatures_legal_docs_tests.rs | OK happy path |
| POST | /api/v1/signature-requests/webhook/{provider} | handle_webhook | done | esignature_webhook_idempotency_tests.rs, esignature_email_status_tracking_tests.rs | OK happy path + idempotency |
| GET | /api/v1/signatures/sign | get_sign_context | done | esignature_sign_consumer_tests.rs | OK happy path (render context) + tamper/replay |
| POST | /api/v1/signatures/sign | submit_signature | done | esignature_sign_consumer_tests.rs | OK happy path (record signature) + CONFLICT/GONE |

## legal.rs  (mount: /api/v1/legal)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/legal/documents | create_document | done | signatures_legal_docs_tests.rs | CREATED happy path |
| GET | /api/v1/legal/documents | list_documents | done | signatures_legal_docs_tests.rs | OK happy path |
| GET | /api/v1/legal/documents/summary | list_documents_summary | done | signatures_legal_docs_tests.rs | OK happy path |
| GET | /api/v1/legal/documents/{id} | get_document | done | legal_cross_org_idor_tests.rs, signatures_legal_docs_tests.rs | same-org read asserts OK + body id |
| PATCH | /api/v1/legal/documents/{id} | update_document | done | signatures_legal_docs_tests.rs | OK happy path |
| DELETE | /api/v1/legal/documents/{id} | delete_document | done | signatures_legal_docs_tests.rs | 204 happy path |
| POST | /api/v1/legal/documents/{id}/versions | add_version | done | signatures_legal_docs_tests.rs | CREATED happy path |
| GET | /api/v1/legal/documents/{id}/versions | list_versions | done | signatures_legal_docs_tests.rs | OK happy path |
| GET | /api/v1/legal/documents/{id}/versions/{version} | get_version | done | signatures_legal_docs_tests.rs | OK happy path |
| POST | /api/v1/legal/requirements | create_requirement | done | signatures_legal_docs_tests.rs | CREATED happy path |
| GET | /api/v1/legal/requirements | list_requirements | done | signatures_legal_docs_tests.rs | OK happy path |
| GET | /api/v1/legal/requirements/with-details | list_requirements_with_details | done | signatures_legal_docs_tests.rs | OK happy path |
| GET | /api/v1/legal/requirements/statistics | get_compliance_statistics | done | signatures_legal_docs_tests.rs | OK happy path |
| GET | /api/v1/legal/requirements/{id} | get_requirement | done | signatures_legal_docs_tests.rs | OK happy path |
| PATCH | /api/v1/legal/requirements/{id} | update_requirement | done | signatures_legal_docs_tests.rs | OK happy path |
| DELETE | /api/v1/legal/requirements/{id} | delete_requirement | done | signatures_legal_docs_tests.rs | 204 happy path |
| POST | /api/v1/legal/requirements/{id}/verify | create_verification | done | signatures_legal_docs_tests.rs | CREATED happy path |
| GET | /api/v1/legal/requirements/{id}/verifications | list_verifications | done | signatures_legal_docs_tests.rs | OK happy path |
| POST | /api/v1/legal/notices | create_notice | done | legal_notices_audit_tests.rs | CREATED happy path |
| GET | /api/v1/legal/notices | list_notices | done | legal_notices_audit_tests.rs | OK happy path |
| GET | /api/v1/legal/notices/with-recipients | list_notices_with_recipients | done | legal_notices_audit_tests.rs | OK happy path |
| GET | /api/v1/legal/notices/statistics | get_notice_statistics | done | legal_notices_audit_tests.rs | OK happy path |
| GET | /api/v1/legal/notices/{id} | get_notice | done | legal_notices_audit_tests.rs | OK happy path |
| PATCH | /api/v1/legal/notices/{id} | update_notice | done | legal_notices_audit_tests.rs | OK happy path |
| DELETE | /api/v1/legal/notices/{id} | delete_notice | done | legal_notices_audit_tests.rs | 204 happy path |
| POST | /api/v1/legal/notices/{id}/send | send_notice | done | legal_notices_audit_tests.rs | OK or 400 (email service unavailable in test env) |
| GET | /api/v1/legal/notices/{id}/recipients | list_recipients | done | legal_notices_audit_tests.rs | OK happy path |
| POST | /api/v1/legal/notices/{notice_id}/acknowledge/{recipient_id} | acknowledge_notice | done | legal_notices_audit_tests.rs | OK happy path |
| POST | /api/v1/legal/templates | create_template | done | legal_notices_audit_tests.rs | CREATED happy path |
| GET | /api/v1/legal/templates | list_templates | done | legal_notices_audit_tests.rs | OK happy path |
| GET | /api/v1/legal/templates/{id} | get_template | done | legal_notices_audit_tests.rs | OK happy path |
| PATCH | /api/v1/legal/templates/{id} | update_template | done | legal_notices_audit_tests.rs | OK happy path |
| DELETE | /api/v1/legal/templates/{id} | delete_template | done | legal_notices_audit_tests.rs | 204 happy path |
| POST | /api/v1/legal/templates/apply | apply_template | done | legal_notices_audit_tests.rs | CREATED happy path |
| GET | /api/v1/legal/audit-trail | list_audit_trail | done | legal_notices_audit_tests.rs | OK happy path |
| POST | /api/v1/legal/audit-trail | create_audit_entry | done | legal_notices_audit_tests.rs | CREATED happy path |

## lease_abstraction.rs  (mount: /api/v1/lease-abstraction)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/lease-abstraction/documents | list_documents | done | lease_abstraction_forms_tests.rs | OK happy path |
| POST | /api/v1/lease-abstraction/documents | upload_document | done | lease_abstraction_forms_tests.rs | CREATED happy path |
| GET | /api/v1/lease-abstraction/documents/{id} | get_document | done | lease_abstraction_forms_tests.rs | OK happy path |
| DELETE | /api/v1/lease-abstraction/documents/{id} | delete_document | done | lease_abstraction_forms_tests.rs | 204 happy path |
| POST | /api/v1/lease-abstraction/documents/{id}/process | process_document | done | lease_abstraction_forms_tests.rs | OK or 409 or 503 (LLM unavailable in test env) |
| GET | /api/v1/lease-abstraction/documents/{id}/extractions | list_extractions | done | lease_abstraction_forms_tests.rs | OK happy path |
| GET | /api/v1/lease-abstraction/extractions/{id} | get_extraction | done | lease_abstraction_forms_tests.rs | OK happy path |
| GET | /api/v1/lease-abstraction/extractions/{id}/fields | get_extraction_fields | done | lease_abstraction_forms_tests.rs | OK happy path |
| POST | /api/v1/lease-abstraction/extractions/{id}/approve | approve_extraction | done | lease_abstraction_forms_tests.rs | OK happy path |
| POST | /api/v1/lease-abstraction/extractions/{id}/reject | reject_extraction | done | lease_abstraction_forms_tests.rs | OK happy path |
| GET | /api/v1/lease-abstraction/extractions/{id}/corrections | list_corrections | done | lease_abstraction_forms_tests.rs | OK happy path |
| POST | /api/v1/lease-abstraction/extractions/{id}/corrections | add_correction | done | lease_abstraction_forms_tests.rs | CREATED happy path |
| POST | /api/v1/lease-abstraction/extractions/{id}/validate | validate_import | partial | — | no test |
| POST | /api/v1/lease-abstraction/extractions/{id}/import | import_to_lease | partial | — | no test |
| GET | /api/v1/lease-abstraction/imports | list_imports | done | lease_abstraction_forms_tests.rs | OK happy path |
| GET | /api/v1/lease-abstraction/imports/{id} | get_import | partial | — | no test |

## forms/crud.rs  (mount: /api/v1/forms)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/forms | create_form | done | lease_abstraction_forms_tests.rs | CREATED happy path |
| GET | /api/v1/forms | list_forms | done | lease_abstraction_forms_tests.rs | OK happy path |
| GET | /api/v1/forms/available | list_available_forms | done | lease_abstraction_forms_tests.rs | OK happy path |
| GET | /api/v1/forms/statistics | get_statistics | done | lease_abstraction_forms_tests.rs | OK happy path |
| GET | /api/v1/forms/{id} | get_form | done | lease_abstraction_forms_tests.rs | OK happy path |
| PUT | /api/v1/forms/{id} | update_form | done | lease_abstraction_forms_tests.rs | OK happy path |
| DELETE | /api/v1/forms/{id} | delete_form | done | lease_abstraction_forms_tests.rs | 204 happy path |
| POST | /api/v1/forms/{id}/publish | publish_form | done | lease_abstraction_forms_tests.rs | OK happy path |
| POST | /api/v1/forms/{id}/archive | archive_form | done | lease_abstraction_forms_tests.rs | OK happy path (publish then archive) |

## forms/fields.rs  (mount: /api/v1/forms)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/forms/{id}/fields | list_fields | done | lease_abstraction_forms_tests.rs | OK happy path |
| POST | /api/v1/forms/{id}/fields | add_field | done | lease_abstraction_forms_tests.rs | CREATED happy path |
| POST | /api/v1/forms/{id}/fields/reorder | reorder_fields | done | lease_abstraction_forms_tests.rs | OK happy path |
| PUT | /api/v1/forms/{id}/fields/{field_id} | update_field | done | lease_abstraction_forms_tests.rs | OK happy path |
| DELETE | /api/v1/forms/{id}/fields/{field_id} | delete_field | done | lease_abstraction_forms_tests.rs | 204 happy path |

## forms/submissions.rs  (mount: /api/v1/forms)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/forms/{id}/submit | submit_form | done | form_cross_org_idor_tests.rs | CREATED happy path |
| GET | /api/v1/forms/{id}/submissions | list_submissions | done | lease_abstraction_forms_tests.rs | OK happy path |
| GET | /api/v1/forms/{id}/submissions/{submission_id} | get_submission | done | lease_abstraction_forms_tests.rs | OK happy path |
| POST | /api/v1/forms/{id}/submissions/{submission_id}/review | review_submission | done | lease_abstraction_forms_tests.rs | OK happy path |
| POST | /api/v1/forms/{id}/download | record_download | done | lease_abstraction_forms_tests.rs | OK happy path (publish then download) |

## Summary
- done: 83 | partial: 35 | stub: 0 | missing: 0 | total: 118
