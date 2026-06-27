# Documents, Forms & Signatures endpoints

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `POST /api/v1/documents` | `documents/core.rs:create_document` | partial | — | real impl, no test |
| `GET /api/v1/documents` | `documents/core.rs:list_documents` | done | `document_access_rls_tests.rs` | RLS read-path |
| `GET /api/v1/documents/{id}` | `documents/core.rs:get_document` | done | `document_upload_tests.rs` | read-back after upload |
| `PUT /api/v1/documents/{id}` | `documents/core.rs:update_document` | partial | — | real impl, no test |
| `DELETE /api/v1/documents/{id}` | `documents/core.rs:delete_document` | partial | — | real impl, no test |
| `POST /api/v1/documents/{id}/move` | `documents/core.rs:move_document` | done | `document_folder_tests.rs` | move into folder |
| `PUT /api/v1/documents/{id}/access` | `documents/core.rs:update_document_access` | partial | — | real impl, no test |
| `GET /api/v1/documents/{id}/download` | `documents/core.rs:get_download_url` | done | `document_download_preview_tests.rs` | |
| `GET /api/v1/documents/{id}/preview` | `documents/core.rs:get_preview_url` | done | `document_download_preview_tests.rs` | |
| `POST /api/v1/documents/upload` | `documents/core.rs:upload_document` | done | `document_upload_tests.rs` | 50 MiB limit |
| `GET /api/v1/documents/{id}/versions` | `documents/versions.rs:get_version_history` | partial | — | no test |
| `POST /api/v1/documents/{id}/versions` | `documents/versions.rs:create_version` | partial | — | no test |
| `GET /api/v1/documents/{id}/versions/{version_id}` | `documents/versions.rs:get_version` | partial | — | no test |
| `POST /api/v1/documents/{id}/versions/{version_id}/restore` | `documents/versions.rs:restore_version` | partial | — | no test |
| `GET /api/v1/documents/folders` | `documents/folders.rs:list_folders` | done | `document_folder_tests.rs` | |
| `POST /api/v1/documents/folders` | `documents/folders.rs:create_folder` | done | `document_folder_tests.rs` | |
| `GET /api/v1/documents/folders/tree` | `documents/folders.rs:get_folder_tree` | done | `document_folder_tests.rs` | |
| `GET /api/v1/documents/folders/{id}` | `documents/folders.rs:get_folder` | done | `document_folder_tests.rs` | |
| `PUT /api/v1/documents/folders/{id}` | `documents/folders.rs:update_folder` | done | `document_folder_tests.rs` | |
| `DELETE /api/v1/documents/folders/{id}` | `documents/folders.rs:delete_folder` | done | `document_folder_tests.rs` | |
| `GET /api/v1/documents/{id}/shares` | `documents/shares.rs:list_shares` | partial | — | no test |
| `POST /api/v1/documents/{id}/shares` | `documents/shares.rs:create_share` | partial | — | no test |
| `DELETE /api/v1/documents/{id}/shares/{share_id}` | `documents/shares.rs:revoke_share` | partial | — | no test |
| `GET /shared/{token}` | `documents/shares.rs:access_shared_document` | done | `document_share_access_tests.rs` | public, root mount |
| `POST /shared/{token}/access` | `documents/shares.rs:access_protected_share` | done | `document_share_access_tests.rs` | password gate |
| `POST /api/v1/documents/{id}/ocr/reprocess` | `documents/intelligence.rs:reprocess_ocr` | partial | — | no test |
| `POST /api/v1/documents/search` | `documents/intelligence.rs:search_documents` | partial | — | no test |
| `GET /api/v1/documents/{id}/classification` | `documents/intelligence.rs:get_classification` | partial | — | no test |
| `POST /api/v1/documents/{id}/classification/feedback` | `documents/intelligence.rs:submit_classification_feedback` | partial | — | no test |
| `GET /api/v1/documents/{id}/classification/history` | `documents/intelligence.rs:get_classification_history` | partial | — | no test |
| `POST /api/v1/documents/{id}/summarize` | `documents/intelligence.rs:request_summarization` | partial | — | no test |
| `POST /api/v1/documents/{id}/ai-summarize` | `documents/intelligence.rs:ai_summarize_document` | partial | — | no test |
| `GET /api/v1/documents/intelligence/stats` | `documents/intelligence.rs:get_intelligence_stats` | partial | — | no test |
| `POST /api/v1/forms` | `forms/crud.rs:create_form` | partial | — | no test |
| `GET /api/v1/forms` | `forms/crud.rs:list_forms` | partial | — | no test |
| `GET /api/v1/forms/available` | `forms/crud.rs:list_available_forms` | partial | — | no test |
| `GET /api/v1/forms/statistics` | `forms/crud.rs:get_statistics` | partial | — | no test |
| `GET /api/v1/forms/{id}` | `forms/crud.rs:get_form` | partial | — | no test |
| `PUT /api/v1/forms/{id}` | `forms/crud.rs:update_form` | partial | — | no test |
| `DELETE /api/v1/forms/{id}` | `forms/crud.rs:delete_form` | partial | — | no test |
| `POST /api/v1/forms/{id}/publish` | `forms/crud.rs:publish_form` | partial | — | no test |
| `POST /api/v1/forms/{id}/archive` | `forms/crud.rs:archive_form` | partial | — | no test |
| `GET /api/v1/forms/{id}/fields` | `forms/fields.rs:list_fields` | partial | — | no test |
| `POST /api/v1/forms/{id}/fields` | `forms/fields.rs:add_field` | partial | — | no test |
| `POST /api/v1/forms/{id}/fields/reorder` | `forms/fields.rs:reorder_fields` | partial | — | no test |
| `PUT /api/v1/forms/{id}/fields/{field_id}` | `forms/fields.rs:update_field` | partial | — | no test |
| `DELETE /api/v1/forms/{id}/fields/{field_id}` | `forms/fields.rs:delete_field` | partial | — | no test |
| `POST /api/v1/forms/{id}/submit` | `forms/submissions.rs:submit_form` | done | `form_cross_org_idor_tests.rs` | cross-org IDOR |
| `GET /api/v1/forms/{id}/submissions` | `forms/submissions.rs:list_submissions` | partial | — | no test |
| `GET /api/v1/forms/{id}/submissions/{submission_id}` | `forms/submissions.rs:get_submission` | partial | — | no test |
| `POST /api/v1/forms/{id}/submissions/{submission_id}/review` | `forms/submissions.rs:review_submission` | partial | — | no test |
| `POST /api/v1/forms/{id}/download` | `forms/submissions.rs:record_download` | done | `form_cross_org_idor_tests.rs` | cross-org IDOR |
| `POST /api/v1/templates` | `templates.rs:create_template` | partial | — | no test |
| `GET /api/v1/templates` | `templates.rs:list_templates` | partial | — | no test |
| `GET /api/v1/templates/{id}` | `templates.rs:get_template` | partial | — | no test |
| `PUT /api/v1/templates/{id}` | `templates.rs:update_template` | partial | — | no test |
| `DELETE /api/v1/templates/{id}` | `templates.rs:delete_template` | partial | — | no test |
| `POST /api/v1/templates/{id}/generate` | `templates.rs:generate_document` | partial | — | no test |
| `GET /api/v1/signature-requests` | `signatures.rs:list_signature_requests` | partial | — | no test |
| `POST /api/v1/signature-requests` | `signatures.rs:create_signature_request` | partial | — | no test |
| `GET /api/v1/signature-requests/{id}` | `signatures.rs:get_signature_request` | done | `esignature_email_status_tracking_tests.rs` | status read-back |
| `POST /api/v1/signature-requests/{id}/remind` | `signatures.rs:send_reminder` | partial | — | no test |
| `POST /api/v1/signature-requests/{id}/cancel` | `signatures.rs:cancel_signature_request` | partial | — | no test |
| `POST /api/v1/signature-requests/webhook/{provider}` | `signatures.rs:handle_webhook` | done | `esignature_webhook_idempotency_tests.rs` | webhook dedup |
| `GET /api/v1/signatures/sign` | `signatures.rs:get_sign_context` | done | `esignature_sign_consumer_tests.rs` | public signer ctx |
| `POST /api/v1/signatures/sign` | `signatures.rs:submit_signature` | done | `esignature_sign_consumer_tests.rs` | public signer submit |

## Tally
done: 19  partial: 42  stub: 0  missing: 0  total: 61
