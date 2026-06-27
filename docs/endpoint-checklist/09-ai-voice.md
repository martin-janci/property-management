# AI & Voice endpoints

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `POST /api/v1/ai/chat/sessions` | `ai/sessions.rs:create_session` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/chat/sessions` | `ai/sessions.rs:list_sessions` | done | `ai_auth_tests.rs` | Auth path asserted |
| `GET /api/v1/ai/chat/sessions/{session_id}` | `ai/sessions.rs:get_session` | partial | — | Repo-level IDOR test only |
| `DELETE /api/v1/ai/chat/sessions/{session_id}` | `ai/sessions.rs:delete_session` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/chat/sessions/{session_id}/messages` | `ai/sessions.rs:list_messages` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/chat/sessions/{session_id}/messages` | `ai/sessions.rs:send_message` | done | `ai_auth_tests.rs` | Auth path asserted |
| `POST /api/v1/ai/chat/messages/{message_id}/feedback` | `ai/sessions.rs:provide_feedback` | partial | — | Repo-level IDOR test only |
| `GET /api/v1/ai/chat/escalated` | `ai/sessions.rs:list_escalated` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/sentiment/trends` | `ai/sessions.rs:get_trends` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/sentiment/alerts` | `ai/sessions.rs:list_alerts` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/sentiment/alerts/{alert_id}/acknowledge` | `ai/sessions.rs:acknowledge_alert` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/sentiment/thresholds` | `ai/sessions.rs:get_thresholds` | partial | — | Real DB, no direct test |
| `PUT /api/v1/ai/sentiment/thresholds` | `ai/sessions.rs:update_thresholds` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/sentiment/dashboard` | `ai/sessions.rs:get_dashboard` | done | `ai_auth_tests.rs` | Auth path asserted |
| `POST /api/v1/ai/equipment/` | `ai/equipment.rs:create_equipment` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/equipment/` | `ai/equipment.rs:list_equipment` | done | `ai_auth_tests.rs` | Auth path asserted |
| `GET /api/v1/ai/equipment/{id}` | `ai/equipment.rs:get_equipment` | done | `equipment_cross_tenant_idor_tests.rs` | IDOR 404 asserted |
| `PUT /api/v1/ai/equipment/{id}` | `ai/equipment.rs:update_equipment` | done | `equipment_cross_tenant_idor_tests.rs` | IDOR asserted |
| `DELETE /api/v1/ai/equipment/{id}` | `ai/equipment.rs:delete_equipment` | done | `equipment_cross_tenant_idor_tests.rs` | IDOR asserted |
| `GET /api/v1/ai/equipment/{id}/maintenance` | `ai/equipment.rs:list_maintenance` | done | `equipment_cross_tenant_idor_tests.rs` | IDOR asserted |
| `POST /api/v1/ai/equipment/{id}/maintenance` | `ai/equipment.rs:create_maintenance` | done | `equipment_cross_tenant_idor_tests.rs` | IDOR asserted |
| `PUT /api/v1/ai/equipment/maintenance/{id}` | `ai/equipment.rs:update_maintenance` | done | `equipment_cross_tenant_idor_tests.rs` | IDOR asserted |
| `GET /api/v1/ai/equipment/predictions` | `ai/equipment.rs:list_predictions` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/equipment/predictions/{id}/acknowledge` | `ai/equipment.rs:acknowledge_prediction` | done | `equipment_cross_tenant_idor_tests.rs` | IDOR asserted |
| `GET /api/v1/ai/equipment/needing-maintenance` | `ai/equipment.rs:list_needing_maintenance` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/workflows/` | `ai/workflows.rs:create_workflow` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/workflows/` | `ai/workflows.rs:list_workflows` | done | `ai_auth_tests.rs` | Auth path asserted |
| `GET /api/v1/ai/workflows/{id}` | `ai/workflows.rs:get_workflow` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `PUT /api/v1/ai/workflows/{id}` | `ai/workflows.rs:update_workflow` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `DELETE /api/v1/ai/workflows/{id}` | `ai/workflows.rs:delete_workflow` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `GET /api/v1/ai/workflows/{id}/actions` | `ai/workflows.rs:list_actions` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `POST /api/v1/ai/workflows/{id}/actions` | `ai/workflows.rs:add_action` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `DELETE /api/v1/ai/workflows/actions/{action_id}` | `ai/workflows.rs:delete_action` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `POST /api/v1/ai/workflows/{id}/trigger` | `ai/workflows.rs:trigger_workflow` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `GET /api/v1/ai/workflows/executions` | `ai/workflows.rs:list_executions` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/workflows/executions/{id}` | `ai/workflows.rs:get_execution` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `GET /api/v1/ai/workflows/executions/{id}/steps` | `ai/workflows.rs:list_execution_steps` | done | `workflow_cross_tenant_idor_tests.rs` | IDOR asserted |
| `POST /api/v1/ai/workflows/events` | `ai/workflows.rs:handle_workflow_event` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/workflows/templates` | `ai/workflows.rs:list_workflow_templates` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/workflows/templates/builtin` | `ai/workflows.rs:list_builtin_templates` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/workflows/templates/{id}` | `ai/workflows.rs:get_workflow_template` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/workflows/templates/{id}/import` | `ai/workflows.rs:import_workflow_template` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/llm/lease/generate` | `ai/llm.rs:generate_lease` | done | `ai_auth_tests.rs` | Auth path asserted |
| `GET /api/v1/ai/llm/lease/templates` | `ai/llm.rs:list_lease_templates` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/llm/lease/templates/{id}` | `ai/llm.rs:get_lease_template` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/llm/listing/description` | `ai/llm.rs:generate_listing_description` | partial | — | LLM + placeholder fallback |
| `GET /api/v1/ai/llm/listing/descriptions/{listing_id}` | `ai/llm.rs:list_listing_descriptions` | partial | — | Repo-level IDOR test only |
| `POST /api/v1/ai/llm/listing/descriptions/{id}/publish` | `ai/llm.rs:publish_description` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/llm/chat/enhanced` | `ai/llm.rs:enhanced_chat` | partial | — | Real LLM, no direct test |
| `GET /api/v1/ai/llm/chat/escalation-config` | `ai/llm.rs:get_escalation_config` | partial | — | Real DB, no direct test |
| `PUT /api/v1/ai/llm/chat/escalation-config` | `ai/llm.rs:update_escalation_config` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/llm/photos/enhance` | `ai/llm.rs:enhance_photo` | partial | — | Real logic, no direct test |
| `POST /api/v1/ai/llm/photos/enhance/batch` | `ai/llm.rs:batch_enhance_photos` | partial | — | Real logic, no direct test |
| `GET /api/v1/ai/llm/photos/{id}` | `ai/llm.rs:get_photo_enhancement` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/llm/voice/devices` | `ai/llm.rs:list_voice_devices` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/llm/voice/devices` | `ai/llm.rs:link_voice_device` | partial | — | Real DB, no direct test |
| `DELETE /api/v1/ai/llm/voice/devices/{id}` | `ai/llm.rs:unlink_voice_device` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/llm/voice/commands/{device_id}` | `ai/llm.rs:list_voice_commands` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/llm/statistics` | `ai/llm.rs:get_ai_statistics` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/llm/requests` | `ai/llm.rs:list_generation_requests` | partial | — | Real DB, no direct test |
| `GET /api/v1/ai/llm/requests/{id}` | `ai/llm.rs:get_generation_request` | partial | — | Real DB, no direct test |
| `POST /api/v1/ai/ocr/meter-reading` | `ai/ocr.rs:process_meter_reading` | stub | — | Returns 501 NOT_IMPLEMENTED |
| `POST /api/v1/ai/ocr/correction` | `ai/ocr.rs:submit_correction` | stub | — | Accepts and discards payload |
| `GET /internal/caddy-ask` | `caddy_ask.rs:caddy_ask` | done | `caddy_ask_tests.rs` | Domain auth asserted |
| `POST /api/v1/webhooks/voice/alexa` | `voice_webhooks.rs:alexa_webhook` | partial | — | Real handler, no direct test |
| `POST /api/v1/webhooks/voice/alexa/health` | `voice_webhooks.rs:alexa_health_check` | partial | — | Trivial 200 health |
| `POST /api/v1/webhooks/voice/google` | `voice_webhooks.rs:google_actions_webhook` | partial | — | Real handler, no direct test |
| `POST /api/v1/webhooks/voice/oauth/exchange` | `voice_webhooks.rs:oauth_token_exchange` | done | `voice_oauth_exchange_auth_tests.rs` | Auth asserted |
| `POST /api/v1/webhooks/voice/oauth/refresh` | `voice_webhooks.rs:oauth_token_refresh` | partial | — | Real handler, no direct test |
| `POST /api/v1/webhooks/voice/verify` | `voice_webhooks.rs:verify_webhook_signature` | partial | — | Real handler, no direct test |
| `GET /api/v1/lease-abstraction/documents` | `lease_abstraction.rs:list_documents` | partial | — | Real DB, no direct test |
| `POST /api/v1/lease-abstraction/documents` | `lease_abstraction.rs:upload_document` | partial | — | Real DB, no direct test |
| `GET /api/v1/lease-abstraction/documents/{id}` | `lease_abstraction.rs:get_document` | partial | — | Real DB, no direct test |
| `DELETE /api/v1/lease-abstraction/documents/{id}` | `lease_abstraction.rs:delete_document` | partial | — | Real DB, no direct test |
| `POST /api/v1/lease-abstraction/documents/{id}/process` | `lease_abstraction.rs:process_document` | partial | — | Persists placeholder extraction |
| `GET /api/v1/lease-abstraction/documents/{id}/extractions` | `lease_abstraction.rs:list_extractions` | partial | — | Real DB, no direct test |
| `GET /api/v1/lease-abstraction/extractions/{id}` | `lease_abstraction.rs:get_extraction` | partial | — | Real DB, no direct test |
| `GET /api/v1/lease-abstraction/extractions/{id}/fields` | `lease_abstraction.rs:get_extraction_fields` | partial | — | Real DB, no direct test |
| `POST /api/v1/lease-abstraction/extractions/{id}/approve` | `lease_abstraction.rs:approve_extraction` | partial | — | Real DB, no direct test |
| `POST /api/v1/lease-abstraction/extractions/{id}/reject` | `lease_abstraction.rs:reject_extraction` | partial | — | Real DB, no direct test |
| `GET /api/v1/lease-abstraction/extractions/{id}/corrections` | `lease_abstraction.rs:list_corrections` | partial | — | Real DB, no direct test |
| `POST /api/v1/lease-abstraction/extractions/{id}/corrections` | `lease_abstraction.rs:add_correction` | partial | — | Real DB, no direct test |
| `POST /api/v1/lease-abstraction/extractions/{id}/validate` | `lease_abstraction.rs:validate_import` | partial | — | Real DB, no direct test |
| `POST /api/v1/lease-abstraction/extractions/{id}/import` | `lease_abstraction.rs:import_to_lease` | partial | — | Real DB, no direct test |
| `GET /api/v1/lease-abstraction/imports` | `lease_abstraction.rs:list_imports` | partial | — | Real DB, no direct test |
| `GET /api/v1/lease-abstraction/imports/{id}` | `lease_abstraction.rs:get_import` | partial | — | Real DB, no direct test |

## Tally
done: 24  partial: 60  stub: 2  missing: 0  total: 86
