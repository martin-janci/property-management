# AI & Automation

_Server: api-server. Modules: registry.rs, automation.rs, ai/sessions.rs, ai/equipment.rs, ai/workflows.rs, ai/llm.rs, ai/ocr.rs (ai/voice.rs handlers mounted by llm_router; ai/mod.rs has no routes)._

> Coverage reality: every test file touching this group is **auth-only / cross-tenant-IDOR** (asserts 401/403/404 rejection, never a 2xx success path). Per spec, an authz-only test does NOT prove the success path, so no real handler here qualifies as `done`. All real handlers are therefore `partial`; the two OCR handlers are `stub`.

## registry.rs  (mount: /api/v1/registry)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/registry/pets | create_pet_registration | done | registry_backfill_tests.rs | success-path 201 |
| GET | /api/v1/registry/pets | list_pet_registrations | done | registry_backfill_tests.rs | success-path 200 |
| GET | /api/v1/registry/pets/{id} | get_pet_registration | done | registry_backfill_tests.rs | success-path 200 |
| PUT | /api/v1/registry/pets/{id} | update_pet_registration | done | registry_backfill_tests.rs | success-path 200 |
| DELETE | /api/v1/registry/pets/{id} | delete_pet_registration | done | registry_backfill_tests.rs | success-path 204 |
| POST | /api/v1/registry/pets/{id}/review | review_pet_registration | done | registry_backfill_tests.rs | success-path 200 |
| POST | /api/v1/registry/vehicles | create_vehicle_registration | done | registry_backfill_tests.rs | success-path 201 |
| GET | /api/v1/registry/vehicles | list_vehicle_registrations | done | registry_backfill_tests.rs | success-path 200 |
| GET | /api/v1/registry/vehicles/{id} | get_vehicle_registration | done | registry_backfill_tests.rs | success-path 200 |
| PUT | /api/v1/registry/vehicles/{id} | update_vehicle_registration | done | registry_backfill_tests.rs | success-path 200 |
| DELETE | /api/v1/registry/vehicles/{id} | delete_vehicle_registration | done | registry_backfill_tests.rs | success-path 204 |
| POST | /api/v1/registry/vehicles/{id}/review | review_vehicle_registration | done | registry_backfill_tests.rs | success-path 200 |
| POST | /api/v1/registry/parking-spots | create_parking_spot | done | registry_backfill_tests.rs | success-path 201 |
| GET | /api/v1/registry/parking-spots | list_parking_spots | done | registry_backfill_tests.rs | success-path 200 |
| GET | /api/v1/registry/parking-spots/{id} | get_parking_spot | done | registry_backfill_tests.rs | success-path 200 |
| PUT | /api/v1/registry/parking-spots/{id} | update_parking_spot | done | registry_backfill_tests.rs | success-path 200 |
| DELETE | /api/v1/registry/parking-spots/{id} | delete_parking_spot | done | registry_backfill_tests.rs | success-path 204 |
| GET | /api/v1/registry/buildings/{building_id}/rules | get_registry_rules | done | registry_backfill_tests.rs | success-path 200 |
| PUT | /api/v1/registry/buildings/{building_id}/rules | update_registry_rules | done | registry_backfill_tests.rs | success-path 200 |
| GET | /api/v1/registry/buildings/{building_id}/statistics | get_registry_statistics | done | registry_backfill_tests.rs | success-path 200 |

## automation.rs  (mount: /api/v1/automation)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/automation/organizations/{org_id}/rules | list_rules | partial | automation_auth_tests.rs | real (automation_repo); test asserts 401/403 only |
| POST | /api/v1/automation/organizations/{org_id}/rules | create_rule | partial | automation_auth_tests.rs | auth-only test |
| GET | /api/v1/automation/rules/{id} | get_rule | partial | automation_auth_tests.rs | auth-only test |
| PUT | /api/v1/automation/rules/{id} | update_rule | partial | automation_auth_tests.rs | auth-only test |
| DELETE | /api/v1/automation/rules/{id} | delete_rule | partial | automation_auth_tests.rs | auth-only test |
| POST | /api/v1/automation/rules/{id}/toggle | toggle_rule | partial | automation_auth_tests.rs | auth-only test |
| GET | /api/v1/automation/rules/{id}/logs | get_rule_logs | partial | automation_auth_tests.rs | auth-only test |
| GET | /api/v1/automation/templates | list_templates | partial | automation_auth_tests.rs | auth-only test |
| GET | /api/v1/automation/templates/{id} | get_template | partial | automation_auth_tests.rs | auth-only test |
| POST | /api/v1/automation/organizations/{org_id}/rules/from-template | create_from_template | partial | — | real; no test reference |

## ai/sessions.rs — ai_chat_router  (mount: /api/v1/ai/chat)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/chat/sessions | create_session | partial | ai_auth_tests.rs | real; auth-only (401/403) |
| GET | /api/v1/ai/chat/sessions | list_sessions | partial | ai_auth_tests.rs | auth-only |
| GET | /api/v1/ai/chat/sessions/{session_id} | get_session | partial | — | real; no test |
| DELETE | /api/v1/ai/chat/sessions/{session_id} | delete_session | partial | — | real; no test |
| GET | /api/v1/ai/chat/sessions/{session_id}/messages | list_messages | partial | — | real; no test |
| POST | /api/v1/ai/chat/sessions/{session_id}/messages | send_message | partial | ai_auth_tests.rs | auth-only |
| POST | /api/v1/ai/chat/messages/{message_id}/feedback | provide_feedback | partial | — | real; no test |
| GET | /api/v1/ai/chat/escalated | list_escalated | partial | — | real; no test |

## ai/sessions.rs — sentiment_router  (mount: /api/v1/ai/sentiment)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/ai/sentiment/trends | get_trends | partial | — | real; no test |
| GET | /api/v1/ai/sentiment/alerts | list_alerts | partial | — | real; no test |
| POST | /api/v1/ai/sentiment/alerts/{alert_id}/acknowledge | acknowledge_alert | partial | — | real; no test |
| GET | /api/v1/ai/sentiment/thresholds | get_thresholds | partial | — | real; no test |
| PUT | /api/v1/ai/sentiment/thresholds | update_thresholds | partial | — | real; no test |
| GET | /api/v1/ai/sentiment/dashboard | get_dashboard | partial | ai_auth_tests.rs | auth-only |

## ai/equipment.rs — equipment_router  (mount: /api/v1/ai/equipment)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/equipment/ | create_equipment | partial | — | real; no test |
| GET | /api/v1/ai/equipment/ | list_equipment | partial | ai_auth_tests.rs | auth-only |
| GET | /api/v1/ai/equipment/{id} | get_equipment | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only (cross-tenant reject) |
| PUT | /api/v1/ai/equipment/{id} | update_equipment | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| DELETE | /api/v1/ai/equipment/{id} | delete_equipment | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/equipment/{id}/maintenance | list_maintenance | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| POST | /api/v1/ai/equipment/{id}/maintenance | create_maintenance | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| PUT | /api/v1/ai/equipment/maintenance/{id} | update_maintenance | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/equipment/predictions | list_predictions | partial | — | real; no test |
| POST | /api/v1/ai/equipment/predictions/{id}/acknowledge | acknowledge_prediction | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/equipment/needing-maintenance | list_needing_maintenance | partial | — | real; no test |

## ai/workflows.rs — workflow_router  (mount: /api/v1/ai/workflows)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/workflows/ | create_workflow | partial | — | real; no test |
| GET | /api/v1/ai/workflows/ | list_workflows | partial | ai_auth_tests.rs | auth-only |
| GET | /api/v1/ai/workflows/{id} | get_workflow | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| PUT | /api/v1/ai/workflows/{id} | update_workflow | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| DELETE | /api/v1/ai/workflows/{id} | delete_workflow | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/workflows/{id}/actions | list_actions | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| POST | /api/v1/ai/workflows/{id}/actions | add_action | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| DELETE | /api/v1/ai/workflows/actions/{action_id} | delete_action | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| POST | /api/v1/ai/workflows/{id}/trigger | trigger_workflow | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/workflows/executions | list_executions | partial | — | real; no test |
| GET | /api/v1/ai/workflows/executions/{id} | get_execution | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/workflows/executions/{id}/steps | list_execution_steps | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| POST | /api/v1/ai/workflows/events | handle_workflow_event | partial | — | real; no test |
| GET | /api/v1/ai/workflows/templates | list_workflow_templates | partial | — | real; no test |
| GET | /api/v1/ai/workflows/templates/builtin | list_builtin_templates | partial | — | real; no test |
| GET | /api/v1/ai/workflows/templates/{id} | get_workflow_template | partial | — | real; no test |
| POST | /api/v1/ai/workflows/templates/{id}/import | import_workflow_template | partial | — | real; no test |

## ai/llm.rs — llm_router  (mount: /api/v1/ai/llm)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/llm/lease/generate | generate_lease | partial | ai_auth_tests.rs | real (llm provider); auth-only test. IDOR test exercises repo layer only, not the endpoint |
| GET | /api/v1/ai/llm/lease/templates | list_lease_templates | partial | — | real; no test |
| GET | /api/v1/ai/llm/lease/templates/{id} | get_lease_template | partial | — | real; no endpoint test (llm IDOR test hits repo fn directly) |
| POST | /api/v1/ai/llm/listing/description | generate_listing_description | partial | — | real; no test |
| GET | /api/v1/ai/llm/listing/descriptions/{listing_id} | list_listing_descriptions | partial | — | real; no endpoint test (llm IDOR test hits repo fn directly) |
| POST | /api/v1/ai/llm/listing/descriptions/{id}/publish | publish_description | partial | — | real; no test |
| POST | /api/v1/ai/llm/chat/enhanced | enhanced_chat | partial | — | real; no test |
| GET | /api/v1/ai/llm/chat/escalation-config | get_escalation_config | partial | — | real; no test |
| PUT | /api/v1/ai/llm/chat/escalation-config | update_escalation_config | partial | — | real; no test |
| POST | /api/v1/ai/llm/photos/enhance | enhance_photo | partial | — | real; no test |
| POST | /api/v1/ai/llm/photos/enhance/batch | batch_enhance_photos | partial | — | real; no test |
| GET | /api/v1/ai/llm/photos/{id} | get_photo_enhancement | partial | — | real; no test |
| GET | /api/v1/ai/llm/voice/devices | list_voice_devices | partial | — | voice handler (ai/voice.rs); real; no test |
| POST | /api/v1/ai/llm/voice/devices | link_voice_device | partial | — | voice handler; real; no test |
| DELETE | /api/v1/ai/llm/voice/devices/{id} | unlink_voice_device | partial | — | voice handler; real; no test |
| GET | /api/v1/ai/llm/voice/commands/{device_id} | list_voice_commands | partial | — | voice handler; real; no test |
| GET | /api/v1/ai/llm/statistics | get_ai_statistics | partial | — | real; no test |
| GET | /api/v1/ai/llm/requests | list_generation_requests | partial | — | real; no test |
| GET | /api/v1/ai/llm/requests/{id} | get_generation_request | partial | — | real; no test |

## ai/ocr.rs — ocr_router  (mount: /api/v1/ai/ocr)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/ocr/meter-reading | process_meter_reading | stub | — | always returns 501 NOT_IMPLEMENTED (OCR backend not configured) |
| POST | /api/v1/ai/ocr/correction | submit_correction | stub | — | accepts payload and discards it (no training sink); returns 200 but does no work |

## Summary
- done: 20 | partial: 71 | stub: 2 | missing: 0 | total: 93
