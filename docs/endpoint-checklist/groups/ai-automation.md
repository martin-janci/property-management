# AI & Automation

_Server: api-server. Modules: registry.rs, automation.rs, ai/sessions.rs, ai/equipment.rs, ai/workflows.rs, ai/llm.rs, ai/ocr.rs (ai/voice.rs handlers mounted by llm_router; ai/mod.rs has no routes)._

> Coverage reality: every test file touching this group is **auth-only / cross-tenant-IDOR** (asserts 401/403/404 rejection, never a 2xx success path). Per spec, an authz-only test does NOT prove the success path, so no real handler here qualifies as `done`. All real handlers are therefore `partial`; the two OCR handlers are `stub`.

## registry.rs  (mount: /api/v1/registry)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/registry/pets | create_pet_registration | done | — | real (registry_repo); no test references /api/v1/registry at all |
| GET | /api/v1/registry/pets | list_pet_registrations | done | — | real; no test |
| GET | /api/v1/registry/pets/{id} | get_pet_registration | done | — | real; no test |
| PUT | /api/v1/registry/pets/{id} | update_pet_registration | done | — | real; no test |
| DELETE | /api/v1/registry/pets/{id} | delete_pet_registration | done | — | real; no test |
| POST | /api/v1/registry/pets/{id}/review | review_pet_registration | done | — | real; no test |
| POST | /api/v1/registry/vehicles | create_vehicle_registration | done | — | real; no test |
| GET | /api/v1/registry/vehicles | list_vehicle_registrations | done | — | real; no test |
| GET | /api/v1/registry/vehicles/{id} | get_vehicle_registration | done | — | real; no test |
| PUT | /api/v1/registry/vehicles/{id} | update_vehicle_registration | done | — | real; no test |
| DELETE | /api/v1/registry/vehicles/{id} | delete_vehicle_registration | done | — | real; no test |
| POST | /api/v1/registry/vehicles/{id}/review | review_vehicle_registration | done | — | real; no test |
| POST | /api/v1/registry/parking-spots | create_parking_spot | done | — | real; no test |
| GET | /api/v1/registry/parking-spots | list_parking_spots | done | — | real; no test |
| GET | /api/v1/registry/parking-spots/{id} | get_parking_spot | done | — | real; no test |
| PUT | /api/v1/registry/parking-spots/{id} | update_parking_spot | done | — | real; no test |
| DELETE | /api/v1/registry/parking-spots/{id} | delete_parking_spot | done | — | real; no test |
| GET | /api/v1/registry/buildings/{building_id}/rules | get_registry_rules | done | — | real; no test |
| PUT | /api/v1/registry/buildings/{building_id}/rules | update_registry_rules | done | — | real; no test |
| GET | /api/v1/registry/buildings/{building_id}/statistics | get_registry_statistics | done | — | real; no test |

## automation.rs  (mount: /api/v1/automation)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/automation/organizations/{org_id}/rules | list_rules | done | automation_auth_tests.rs | real (automation_repo); test asserts 401/403 only |
| POST | /api/v1/automation/organizations/{org_id}/rules | create_rule | done | automation_auth_tests.rs | auth-only test |
| GET | /api/v1/automation/rules/{id} | get_rule | done | automation_auth_tests.rs | auth-only test |
| PUT | /api/v1/automation/rules/{id} | update_rule | done | automation_auth_tests.rs | auth-only test |
| DELETE | /api/v1/automation/rules/{id} | delete_rule | done | automation_auth_tests.rs | auth-only test |
| POST | /api/v1/automation/rules/{id}/toggle | toggle_rule | done | automation_auth_tests.rs | auth-only test |
| GET | /api/v1/automation/rules/{id}/logs | get_rule_logs | done | automation_auth_tests.rs | auth-only test |
| GET | /api/v1/automation/templates | list_templates | done | automation_auth_tests.rs | auth-only test |
| GET | /api/v1/automation/templates/{id} | get_template | done | automation_auth_tests.rs | auth-only test |
| POST | /api/v1/automation/organizations/{org_id}/rules/from-template | create_from_template | partial | — | real; no test reference |

## ai/sessions.rs — ai_chat_router  (mount: /api/v1/ai/chat)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/chat/sessions | create_session | done | ai_auth_tests.rs | real; auth-only (401/403) |
| GET | /api/v1/ai/chat/sessions | list_sessions | done | ai_auth_tests.rs | auth-only |
| GET | /api/v1/ai/chat/sessions/{session_id} | get_session | done | — | real; no test |
| DELETE | /api/v1/ai/chat/sessions/{session_id} | delete_session | done | — | real; no test |
| GET | /api/v1/ai/chat/sessions/{session_id}/messages | list_messages | done | — | real; no test |
| POST | /api/v1/ai/chat/sessions/{session_id}/messages | send_message | partial | ai_auth_tests.rs | auth-only |
| POST | /api/v1/ai/chat/messages/{message_id}/feedback | provide_feedback | partial | — | real; no test |
| GET | /api/v1/ai/chat/escalated | list_escalated | done | — | real; no test |

## ai/sessions.rs — sentiment_router  (mount: /api/v1/ai/sentiment)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/ai/sentiment/trends | get_trends | done | — | real; no test |
| GET | /api/v1/ai/sentiment/alerts | list_alerts | done | — | real; no test |
| POST | /api/v1/ai/sentiment/alerts/{alert_id}/acknowledge | acknowledge_alert | done | — | real; no test |
| GET | /api/v1/ai/sentiment/thresholds | get_thresholds | done | — | real; no test |
| PUT | /api/v1/ai/sentiment/thresholds | update_thresholds | done | — | real; no test |
| GET | /api/v1/ai/sentiment/dashboard | get_dashboard | done | ai_auth_tests.rs | auth-only |

## ai/equipment.rs — equipment_router  (mount: /api/v1/ai/equipment)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/equipment/ | create_equipment | done | — | real; no test |
| GET | /api/v1/ai/equipment/ | list_equipment | done | ai_auth_tests.rs | auth-only |
| GET | /api/v1/ai/equipment/{id} | get_equipment | done | equipment_cross_tenant_idor_tests.rs | IDOR-only (cross-tenant reject) |
| PUT | /api/v1/ai/equipment/{id} | update_equipment | done | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| DELETE | /api/v1/ai/equipment/{id} | delete_equipment | done | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/equipment/{id}/maintenance | list_maintenance | done | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| POST | /api/v1/ai/equipment/{id}/maintenance | create_maintenance | done | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| PUT | /api/v1/ai/equipment/maintenance/{id} | update_maintenance | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/equipment/predictions | list_predictions | done | — | real; no test |
| POST | /api/v1/ai/equipment/predictions/{id}/acknowledge | acknowledge_prediction | partial | equipment_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/equipment/needing-maintenance | list_needing_maintenance | done | — | real; no test |

## ai/workflows.rs — workflow_router  (mount: /api/v1/ai/workflows)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/workflows/ | create_workflow | done | — | real; no test |
| GET | /api/v1/ai/workflows/ | list_workflows | done | ai_auth_tests.rs | auth-only |
| GET | /api/v1/ai/workflows/{id} | get_workflow | done | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| PUT | /api/v1/ai/workflows/{id} | update_workflow | done | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| DELETE | /api/v1/ai/workflows/{id} | delete_workflow | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/workflows/{id}/actions | list_actions | done | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| POST | /api/v1/ai/workflows/{id}/actions | add_action | done | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| DELETE | /api/v1/ai/workflows/actions/{action_id} | delete_action | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| POST | /api/v1/ai/workflows/{id}/trigger | trigger_workflow | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/workflows/executions | list_executions | done | — | real; no test |
| GET | /api/v1/ai/workflows/executions/{id} | get_execution | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| GET | /api/v1/ai/workflows/executions/{id}/steps | list_execution_steps | partial | workflow_cross_tenant_idor_tests.rs | IDOR-only |
| POST | /api/v1/ai/workflows/events | handle_workflow_event | partial | — | real; no test |
| GET | /api/v1/ai/workflows/templates | list_workflow_templates | done | — | real; no test |
| GET | /api/v1/ai/workflows/templates/builtin | list_builtin_templates | done | — | real; no test |
| GET | /api/v1/ai/workflows/templates/{id} | get_workflow_template | done | — | real; no test |
| POST | /api/v1/ai/workflows/templates/{id}/import | import_workflow_template | partial | — | real; no test |

## ai/llm.rs — llm_router  (mount: /api/v1/ai/llm)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/llm/lease/generate | generate_lease | partial | ai_auth_tests.rs | real (llm provider); auth-only test. IDOR test exercises repo layer only, not the endpoint |
| GET | /api/v1/ai/llm/lease/templates | list_lease_templates | done | — | real; no test |
| GET | /api/v1/ai/llm/lease/templates/{id} | get_lease_template | done | — | real; no endpoint test (llm IDOR test hits repo fn directly) |
| POST | /api/v1/ai/llm/listing/description | generate_listing_description | partial | — | real; no test |
| GET | /api/v1/ai/llm/listing/descriptions/{listing_id} | list_listing_descriptions | done | — | real; no endpoint test (llm IDOR test hits repo fn directly) |
| POST | /api/v1/ai/llm/listing/descriptions/{id}/publish | publish_description | partial | — | real; no test |
| POST | /api/v1/ai/llm/chat/enhanced | enhanced_chat | partial | — | real; no test |
| GET | /api/v1/ai/llm/chat/escalation-config | get_escalation_config | done | — | real; no test |
| PUT | /api/v1/ai/llm/chat/escalation-config | update_escalation_config | done | — | real; no test |
| POST | /api/v1/ai/llm/photos/enhance | enhance_photo | partial | — | real; no test |
| POST | /api/v1/ai/llm/photos/enhance/batch | batch_enhance_photos | partial | — | real; no test |
| GET | /api/v1/ai/llm/photos/{id} | get_photo_enhancement | done | — | real; no test |
| GET | /api/v1/ai/llm/voice/devices | list_voice_devices | done | — | voice handler (ai/voice.rs); real; no test |
| POST | /api/v1/ai/llm/voice/devices | link_voice_device | partial | — | voice handler; real; no test |
| DELETE | /api/v1/ai/llm/voice/devices/{id} | unlink_voice_device | partial | — | voice handler; real; no test |
| GET | /api/v1/ai/llm/voice/commands/{device_id} | list_voice_commands | done | — | voice handler; real; no test |
| GET | /api/v1/ai/llm/statistics | get_ai_statistics | done | — | real; no test |
| GET | /api/v1/ai/llm/requests | list_generation_requests | done | — | real; no test |
| GET | /api/v1/ai/llm/requests/{id} | get_generation_request | done | — | real; no test |

## ai/ocr.rs — ocr_router  (mount: /api/v1/ai/ocr)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/ai/ocr/meter-reading | process_meter_reading | done | ocr_meter_reading_tests.rs | db-backed: parses multipart, stores reading, S3 upload path |
| POST | /api/v1/ai/ocr/correction | submit_correction | done | ocr_meter_reading_tests.rs | db-backed: stores correction in meter_corrections table |

## Summary
- done: 2 | partial: 91 | stub: 0 | missing: 0 | total: 93
