# Faults & Maintenance

_Server: api-server. Modules: faults.rs, work_orders.rs, facilities.rs, predictive_maintenance.rs, meters.rs, outages.rs, energy.rs, insurance.rs, vendor_portal.rs, emergency/, iot/, vendors/._

Mount prefixes (from `backend/servers/api-server/src/lib.rs`):
- faults → `/api/v1/faults` · work_orders → `/api/v1/work-orders` · facilities → `/api/v1` · predictive_maintenance → `/api/v1/predictive-maintenance` · meters → `/api/v1/meters` · outages → `/api/v1/outages` · energy → `/api/v1/energy` · insurance → `/api/v1/insurance` · emergency → `/api/v1/emergency` · iot::sensor_router → `/api/v1/iot/sensors` · vendors → `/api/v1/vendors`
- **vendor_portal → UNMOUNTED** (ROADMAP(PAP-24), lib.rs:329). Module is a pure 501 stub.

> Coverage note: nearly all tests in this group are cross-org/IDOR or auth-only suites. Per spec, a test that only asserts 4xx/`assert_ne!(OK)` does NOT prove the success path. Only endpoints with an explicit same-org `assert_eq!(StatusCode::OK|CREATED)` happy-path are marked `done`.

## faults.rs  (mount: /api/v1/faults)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/faults | create_fault | partial | faults_tests.rs | only auth-reject test; has idempotency middleware layer |
| GET | /api/v1/faults | list_faults | partial | faults_tests.rs | auth-reject only |
| GET | /api/v1/faults/my | list_my_faults | partial | faults_tests.rs | auth-reject only |
| GET | /api/v1/faults/{id} | get_fault | partial | — | no test (axum 0.7 `{id}` path-param tests skipped per file note) |
| PUT | /api/v1/faults/{id} | update_fault | partial | — | |
| POST | /api/v1/faults/{id}/triage | triage_fault | partial | — | |
| POST | /api/v1/faults/{id}/assign | assign_fault | partial | — | |
| PUT | /api/v1/faults/{id}/status | update_status | partial | — | |
| POST | /api/v1/faults/{id}/resolve | resolve_fault | partial | — | |
| POST | /api/v1/faults/{id}/confirm | confirm_fault | partial | — | |
| POST | /api/v1/faults/{id}/reopen | reopen_fault | partial | — | |
| GET | /api/v1/faults/{id}/comments | list_comments | partial | — | |
| POST | /api/v1/faults/{id}/comments | add_comment | partial | — | |
| POST | /api/v1/faults/{id}/work-notes | add_work_note | partial | — | |
| GET | /api/v1/faults/{id}/attachments | list_attachments | partial | — | |
| POST | /api/v1/faults/{id}/attachments | add_attachment | partial | — | |
| DELETE | /api/v1/faults/{id}/attachments/{attachment_id} | delete_attachment | partial | — | |
| POST | /api/v1/faults/{id}/suggest | get_ai_suggestion | partial | — | |
| GET | /api/v1/faults/statistics | get_statistics | partial | faults_tests.rs | auth-reject only |

## work_orders.rs  (mount: /api/v1/work-orders)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/work-orders | create_work_order | partial | work_order_cross_org_idor_tests.rs | seeded via repo helper, no API happy-path |
| GET | /api/v1/work-orders | list_work_orders | partial | work_order_cross_org_idor_tests.rs | IDOR only |
| GET | /api/v1/work-orders/with-details | list_work_orders_with_details | partial | — | |
| GET | /api/v1/work-orders/statistics | get_statistics | partial | — | |
| GET | /api/v1/work-orders/overdue | list_overdue | partial | — | |
| GET | /api/v1/work-orders/{id} | get_work_order | done | work_order_cross_org_idor_tests.rs | same-org 200 (get_work_order_same_org_succeeds) |
| PATCH | /api/v1/work-orders/{id} | update_work_order | partial | work_order_cross_org_idor_tests.rs | IDOR-reject only |
| DELETE | /api/v1/work-orders/{id} | delete_work_order | partial | work_order_cross_org_idor_tests.rs | IDOR-reject only |
| POST | /api/v1/work-orders/{id}/assign | assign_work_order | partial | — | |
| POST | /api/v1/work-orders/{id}/start | start_work | partial | — | |
| POST | /api/v1/work-orders/{id}/complete | complete_work_order | partial | — | |
| POST | /api/v1/work-orders/{id}/hold | put_on_hold | partial | — | |
| POST | /api/v1/work-orders/{id}/comments | add_comment | partial | — | |
| GET | /api/v1/work-orders/{id}/comments | list_comments | partial | — | |
| POST | /api/v1/work-orders/schedules | create_schedule | partial | — | |
| GET | /api/v1/work-orders/schedules | list_schedules | partial | — | |
| GET | /api/v1/work-orders/schedules/upcoming | get_upcoming_schedules | partial | — | |
| POST | /api/v1/work-orders/schedules/process-due | process_due_schedules | partial | — | |
| GET | /api/v1/work-orders/schedules/{id} | get_schedule | partial | — | |
| PATCH | /api/v1/work-orders/schedules/{id} | update_schedule | partial | — | |
| DELETE | /api/v1/work-orders/schedules/{id} | delete_schedule | partial | — | |
| POST | /api/v1/work-orders/schedules/{id}/activate | activate_schedule | partial | — | |
| POST | /api/v1/work-orders/schedules/{id}/deactivate | deactivate_schedule | partial | — | |
| POST | /api/v1/work-orders/schedules/{id}/skip | skip_schedule | partial | — | |
| GET | /api/v1/work-orders/schedules/{id}/executions | list_executions | partial | — | |
| GET | /api/v1/work-orders/equipment/{equipment_id}/service-history | get_equipment_service_history | done | service_history_cross_org_idor_tests.rs, work_order_cross_org_idor_tests.rs | same-org 200 |
| GET | /api/v1/work-orders/buildings/{building_id}/service-history | get_building_service_history | done | service_history_cross_org_idor_tests.rs | same-org 200 |
| GET | /api/v1/work-orders/cost-summary | get_cost_summary | partial | — | |

## facilities.rs  (mount: /api/v1)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/buildings/{building_id}/facilities | list_facilities | partial | — | |
| POST | /api/v1/buildings/{building_id}/facilities | create_facility | partial | — | |
| GET | /api/v1/buildings/{building_id}/facilities/{id} | get_facility | partial | — | |
| PUT | /api/v1/buildings/{building_id}/facilities/{id} | update_facility | partial | — | |
| DELETE | /api/v1/buildings/{building_id}/facilities/{id} | delete_facility | partial | — | |
| GET | /api/v1/buildings/{building_id}/facilities/{facility_id}/bookings | list_facility_bookings | partial | — | |
| POST | /api/v1/buildings/{building_id}/facilities/{facility_id}/bookings | create_booking | partial | — | |
| GET | /api/v1/buildings/{building_id}/facilities/{facility_id}/availability | check_availability | partial | — | |
| GET | /api/v1/bookings/my | list_my_bookings | partial | — | |
| GET | /api/v1/bookings/{id} | get_booking | done | booking_cross_user_idor_tests.rs | owner same-org 200 (get_booking_as_owner_is_allowed) |
| PUT | /api/v1/bookings/{id} | update_booking | partial | — | |
| POST | /api/v1/bookings/{id}/cancel | cancel_booking | partial | — | |
| GET | /api/v1/buildings/{building_id}/bookings/pending | list_pending_bookings | partial | — | |
| POST | /api/v1/bookings/{id}/approve | approve_booking | partial | — | |
| POST | /api/v1/bookings/{id}/reject | reject_booking | partial | — | |

## predictive_maintenance.rs  (mount: /api/v1/predictive-maintenance)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/predictive-maintenance/equipment | create_equipment | partial | — | |
| GET | /api/v1/predictive-maintenance/equipment | list_equipment | partial | — | |
| GET | /api/v1/predictive-maintenance/equipment/{id} | get_equipment | partial | — | |
| PUT | /api/v1/predictive-maintenance/equipment/{id} | update_equipment | partial | — | |
| DELETE | /api/v1/predictive-maintenance/equipment/{id} | delete_equipment | partial | — | |
| POST | /api/v1/predictive-maintenance/equipment/{id}/documents | add_equipment_document | partial | — | |
| GET | /api/v1/predictive-maintenance/equipment/{id}/documents | list_equipment_documents | partial | — | |
| POST | /api/v1/predictive-maintenance/maintenance-logs | create_maintenance_log | partial | — | |
| GET | /api/v1/predictive-maintenance/maintenance-logs/{id} | get_maintenance_log | partial | — | |
| PUT | /api/v1/predictive-maintenance/maintenance-logs/{id} | update_maintenance_log | partial | — | |
| GET | /api/v1/predictive-maintenance/equipment/{id}/maintenance-logs | list_equipment_maintenance_logs | partial | — | |
| POST | /api/v1/predictive-maintenance/maintenance-logs/{id}/photos | add_maintenance_photo | partial | predictive_maintenance_photos_idor_tests.rs | IDOR-reject only for POST |
| GET | /api/v1/predictive-maintenance/maintenance-logs/{id}/photos | list_maintenance_photos | done | predictive_maintenance_photos_idor_tests.rs | same-org 200 (list_photos_same_org_succeeds) |
| POST | /api/v1/predictive-maintenance/predictions/run | run_prediction | partial | — | |
| POST | /api/v1/predictive-maintenance/predictions/batch | run_batch_predictions | partial | — | |
| GET | /api/v1/predictive-maintenance/equipment/{id}/predictions | get_equipment_predictions | partial | — | |
| GET | /api/v1/predictive-maintenance/alerts | list_alerts | partial | — | |
| POST | /api/v1/predictive-maintenance/alerts/{id}/acknowledge | acknowledge_alert | partial | — | |
| POST | /api/v1/predictive-maintenance/alerts/{id}/resolve | resolve_alert | partial | — | |
| POST | /api/v1/predictive-maintenance/alerts/{id}/dismiss | dismiss_alert | partial | — | |
| GET | /api/v1/predictive-maintenance/thresholds | list_health_thresholds | partial | — | |
| POST | /api/v1/predictive-maintenance/thresholds | set_health_threshold | partial | — | |
| GET | /api/v1/predictive-maintenance/dashboard | get_dashboard | partial | — | |
| GET | /api/v1/predictive-maintenance/equipment/by-health | get_equipment_by_health | partial | — | |

## meters.rs  (mount: /api/v1/meters)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/meters | register_meter | partial | — | |
| GET | /api/v1/meters/buildings/{building_id} | list_meters | partial | meters_energy_cross_org_idor_tests.rs | auth-reject only |
| GET | /api/v1/meters/{id} | get_meter | done | meters_energy_cross_org_idor_tests.rs | same-org 200 (own meter readable) |
| POST | /api/v1/meters/{id}/replace | replace_meter | partial | — | |
| GET | /api/v1/meters/units/{unit_id} | list_unit_meters | partial | — | |
| POST | /api/v1/meters/readings | submit_reading | partial | — | |
| GET | /api/v1/meters/readings/{id} | get_reading | partial | — | |
| GET | /api/v1/meters/{meter_id}/readings | list_readings | partial | — | |
| POST | /api/v1/meters/submission-windows | create_submission_window | partial | — | |
| GET | /api/v1/meters/submission-windows/open/{building_id} | get_open_window | partial | — | |
| PUT | /api/v1/meters/readings/{id}/validate | validate_reading | partial | — | |
| GET | /api/v1/meters/readings/pending | get_pending_readings | partial | — | |
| GET | /api/v1/meters/validation-rules | get_validation_rules | partial | — | |
| POST | /api/v1/meters/utility-bills | create_utility_bill | partial | — | |
| GET | /api/v1/meters/utility-bills/{id} | get_utility_bill | partial | — | |
| POST | /api/v1/meters/utility-bills/{id}/distribute | distribute_bill | partial | — | |
| GET | /api/v1/meters/{meter_id}/consumption | get_consumption_history | partial | — | |
| GET | /api/v1/meters/{meter_id}/aggregates | get_consumption_aggregates | partial | — | |
| GET | /api/v1/meters/providers | list_providers | partial | — | |
| GET | /api/v1/meters/providers/{id} | get_provider | partial | — | |
| POST | /api/v1/meters/ingest | ingest_smart_reading | partial | — | |
| GET | /api/v1/meters/alerts | list_missing_alerts | partial | — | |
| POST | /api/v1/meters/alerts/{id}/resolve | resolve_alert | partial | — | |

## outages.rs  (mount: /api/v1/outages)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/outages | create_outage | partial | endpoints_smoke_tests.rs | auth-reject only |
| GET | /api/v1/outages | list_outages | partial | endpoints_smoke_tests.rs | auth-reject only |
| GET | /api/v1/outages/active | list_active_outages | partial | endpoints_smoke_tests.rs | auth-reject only |
| GET | /api/v1/outages/{id} | get_outage | partial | — | |
| PUT | /api/v1/outages/{id} | update_outage | partial | — | |
| DELETE | /api/v1/outages/{id} | delete_outage | partial | — | |
| POST | /api/v1/outages/{id}/start | start_outage | partial | — | |
| POST | /api/v1/outages/{id}/resolve | resolve_outage | partial | — | |
| POST | /api/v1/outages/{id}/cancel | cancel_outage | partial | — | |
| POST | /api/v1/outages/{id}/read | mark_read | partial | — | |
| GET | /api/v1/outages/statistics | get_statistics | partial | — | |
| GET | /api/v1/outages/dashboard | get_dashboard | partial | endpoints_smoke_tests.rs | auth-reject only |
| GET | /api/v1/outages/unread-count | get_unread_count | partial | endpoints_smoke_tests.rs | auth-reject only |

## energy.rs  (mount: /api/v1/energy)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/energy/units/{unit_id}/epc | get_unit_epc | partial | meters_energy_cross_org_idor_tests.rs | IDOR-reject only |
| POST | /api/v1/energy/units/{unit_id}/epc | create_unit_epc | partial | — | |
| PUT | /api/v1/energy/units/{unit_id}/epc | update_unit_epc | partial | — | |
| GET | /api/v1/energy/buildings/{building_id}/epcs | list_building_epcs | partial | — | |
| GET | /api/v1/energy/epc/{id} | get_epc | partial | — | |
| DELETE | /api/v1/energy/epc/{id} | delete_epc | partial | — | |
| GET | /api/v1/energy/buildings/{building_id}/carbon | get_carbon_dashboard | partial | — | |
| POST | /api/v1/energy/buildings/{building_id}/emissions | record_emission | partial | — | |
| GET | /api/v1/energy/buildings/{building_id}/emissions | list_emissions | partial | — | |
| POST | /api/v1/energy/buildings/{building_id}/carbon/target | set_carbon_target | partial | — | |
| GET | /api/v1/energy/buildings/{building_id}/carbon/export | export_carbon_report | partial | — | |
| GET | /api/v1/energy/listings/{listing_id}/sustainability | get_sustainability_score | partial | — | |
| POST | /api/v1/energy/listings/{listing_id}/sustainability | create_sustainability_score | partial | — | |
| PUT | /api/v1/energy/listings/{listing_id}/sustainability | update_sustainability_score | partial | — | |
| GET | /api/v1/energy/listings/sustainability/search | search_sustainable_listings | partial | — | |
| GET | /api/v1/energy/buildings/{building_id}/benchmark | get_benchmark_dashboard | partial | meters_energy_cross_org_idor_tests.rs | IDOR-reject only |
| POST | /api/v1/energy/buildings/{building_id}/benchmark/calculate | calculate_benchmark | partial | — | |
| GET | /api/v1/energy/buildings/{building_id}/benchmark/alerts | list_benchmark_alerts | partial | — | |
| POST | /api/v1/energy/benchmark/alerts/{id}/resolve | resolve_benchmark_alert | partial | — | |

## insurance.rs  (mount: /api/v1/insurance)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/insurance/policies | list_policies | done | insurance_cross_tenant_idor_tests.rs | tenant-resolved 200, returns own policy |
| POST | /api/v1/insurance/policies | create_policy | partial | — | seeded via repo, no API happy-path |
| GET | /api/v1/insurance/policies/{policy_id} | get_policy | partial | insurance_cross_tenant_idor_tests.rs | IDOR-reject only |
| PUT | /api/v1/insurance/policies/{policy_id} | update_policy | partial | — | |
| DELETE | /api/v1/insurance/policies/{policy_id} | delete_policy | partial | insurance_cross_tenant_idor_tests.rs | IDOR-reject only |
| GET | /api/v1/insurance/policies/expiring | get_expiring_policies | partial | — | |
| GET | /api/v1/insurance/policies/{policy_id}/documents | list_policy_documents | partial | — | |
| POST | /api/v1/insurance/policies/{policy_id}/documents | add_policy_document | partial | — | |
| DELETE | /api/v1/insurance/policies/{policy_id}/documents/{document_id} | remove_policy_document | partial | — | |
| GET | /api/v1/insurance/policies/{policy_id}/reminders | list_reminders | partial | — | |
| POST | /api/v1/insurance/policies/{policy_id}/reminders | create_reminder | partial | — | |
| PUT | /api/v1/insurance/reminders/{reminder_id} | update_reminder | partial | — | |
| DELETE | /api/v1/insurance/reminders/{reminder_id} | delete_reminder | partial | — | |
| GET | /api/v1/insurance/claims | list_claims | partial | insurance_cross_tenant_idor_tests.rs | auth/IDOR only |
| POST | /api/v1/insurance/claims | create_claim | partial | — | |
| GET | /api/v1/insurance/claims/{claim_id} | get_claim | partial | — | |
| PUT | /api/v1/insurance/claims/{claim_id} | update_claim | partial | — | |
| DELETE | /api/v1/insurance/claims/{claim_id} | delete_claim | partial | — | |
| POST | /api/v1/insurance/claims/{claim_id}/submit | submit_claim | partial | — | |
| POST | /api/v1/insurance/claims/{claim_id}/review | review_claim | partial | — | |
| POST | /api/v1/insurance/claims/{claim_id}/payment | record_claim_payment | partial | — | |
| GET | /api/v1/insurance/claims/{claim_id}/history | get_claim_history | partial | — | |
| GET | /api/v1/insurance/claims/{claim_id}/documents | list_claim_documents | partial | — | |
| POST | /api/v1/insurance/claims/{claim_id}/documents | add_claim_document | partial | — | |
| DELETE | /api/v1/insurance/claims/{claim_id}/documents/{document_id} | remove_claim_document | partial | — | |
| GET | /api/v1/insurance/statistics | get_statistics | partial | — | |

## vendor_portal.rs  (mount: UNMOUNTED — ROADMAP(PAP-24))
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/vendor-portal/dashboard/stats | get_dashboard_stats | stub | vendor_portal_stub_removal_tests.rs | router unmounted; handler returns 501; test asserts 404/not-200 |
| GET | /api/v1/vendor-portal/jobs | list_jobs | stub | vendor_portal_stub_removal_tests.rs | 501 / unmounted |
| GET | /api/v1/vendor-portal/jobs/{job_id} | get_job_details | stub | vendor_portal_stub_removal_tests.rs | 501 / unmounted |
| POST | /api/v1/vendor-portal/jobs/{job_id}/accept | accept_job | stub | — | 501 / unmounted |
| POST | /api/v1/vendor-portal/jobs/{job_id}/decline | decline_job | stub | — | 501 / unmounted |
| POST | /api/v1/vendor-portal/jobs/{job_id}/propose-time | propose_alternative_time | stub | — | 501 / unmounted |
| GET | /api/v1/vendor-portal/jobs/{job_id}/access | get_access_info | stub | — | 501 / unmounted |
| POST | /api/v1/vendor-portal/jobs/{job_id}/access/generate-code | generate_access_code | stub | — | 501 / unmounted |
| POST | /api/v1/vendor-portal/jobs/{job_id}/complete | submit_work_completion | stub | — | 501 / unmounted |
| GET | /api/v1/vendor-portal/jobs/{job_id}/completion | get_work_completion | stub | — | 501 / unmounted |
| GET | /api/v1/vendor-portal/invoices | list_invoices | stub | vendor_portal_stub_removal_tests.rs | 501 / unmounted |
| GET | /api/v1/vendor-portal/profile | get_profile | stub | vendor_portal_stub_removal_tests.rs | 501 / unmounted |
| GET | /api/v1/vendor-portal/feedback | list_feedback | stub | — | 501 / unmounted |
| GET | /api/v1/vendor-portal/earnings | get_earnings_summary | stub | — | 501 / unmounted |

## emergency/  (mount: /api/v1/emergency)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/emergency/protocols | create_protocol | partial | emergency_cross_org_idor_tests.rs | auth/IDOR only |
| GET | /api/v1/emergency/protocols | list_protocols | partial | emergency_cross_org_idor_tests.rs | auth/IDOR only |
| GET | /api/v1/emergency/protocols/{id} | get_protocol | done | emergency_cross_org_idor_tests.rs | same-org 200 (get_protocol_for_own_org_succeeds) |
| PUT | /api/v1/emergency/protocols/{id} | update_protocol | partial | emergency_cross_org_idor_tests.rs | IDOR-reject only |
| DELETE | /api/v1/emergency/protocols/{id} | delete_protocol | partial | — | |
| POST | /api/v1/emergency/contacts | create_contact | partial | — | |
| GET | /api/v1/emergency/contacts | list_contacts | partial | — | |
| GET | /api/v1/emergency/contacts/{id} | get_contact | partial | — | |
| PUT | /api/v1/emergency/contacts/{id} | update_contact | partial | — | |
| DELETE | /api/v1/emergency/contacts/{id} | delete_contact | partial | — | |
| POST | /api/v1/emergency/incidents | create_incident | partial | — | |
| GET | /api/v1/emergency/incidents | list_incidents | partial | — | |
| GET | /api/v1/emergency/incidents/active | get_active_incidents | partial | — | |
| GET | /api/v1/emergency/incidents/{id} | get_incident | partial | — | |
| PUT | /api/v1/emergency/incidents/{id} | update_incident | partial | — | |
| POST | /api/v1/emergency/incidents/{id}/acknowledge | acknowledge_incident | partial | — | |
| POST | /api/v1/emergency/incidents/{id}/resolve | resolve_incident | partial | — | |
| POST | /api/v1/emergency/incidents/{id}/close | close_incident | partial | — | |
| POST | /api/v1/emergency/incidents/{id}/attachments | add_incident_attachment | partial | — | |
| GET | /api/v1/emergency/incidents/{id}/attachments | list_incident_attachments | partial | — | |
| POST | /api/v1/emergency/incidents/{id}/updates | add_incident_update | partial | — | |
| GET | /api/v1/emergency/incidents/{id}/updates | list_incident_updates | partial | — | |
| POST | /api/v1/emergency/broadcasts | create_broadcast | done | emergency_cross_org_idor_tests.rs | manager same-org 201 (create_broadcast_as_manager_succeeds) |
| GET | /api/v1/emergency/broadcasts | list_broadcasts | partial | — | |
| GET | /api/v1/emergency/broadcasts/{id} | get_broadcast | partial | — | |
| POST | /api/v1/emergency/broadcasts/{id}/deactivate | deactivate_broadcast | partial | — | |
| POST | /api/v1/emergency/broadcasts/{id}/acknowledge | acknowledge_broadcast | partial | — | |
| GET | /api/v1/emergency/broadcasts/{id}/acknowledgments | list_broadcast_acknowledgments | partial | — | |
| POST | /api/v1/emergency/drills | create_drill | partial | — | |
| GET | /api/v1/emergency/drills | list_drills | partial | — | |
| GET | /api/v1/emergency/drills/upcoming | get_upcoming_drills | partial | — | |
| GET | /api/v1/emergency/drills/{id} | get_drill | partial | — | |
| PUT | /api/v1/emergency/drills/{id} | update_drill | partial | — | |
| POST | /api/v1/emergency/drills/{id}/start | start_drill | partial | — | |
| POST | /api/v1/emergency/drills/{id}/complete | complete_drill | partial | — | |
| POST | /api/v1/emergency/drills/{id}/cancel | cancel_drill | partial | — | |
| DELETE | /api/v1/emergency/drills/{id} | delete_drill | partial | — | |
| GET | /api/v1/emergency/statistics | get_statistics | partial | — | |
| GET | /api/v1/emergency/statistics/incidents/by-type | get_incidents_by_type | partial | — | |
| GET | /api/v1/emergency/statistics/incidents/by-severity | get_incidents_by_severity | partial | — | |

## iot/  (mount: /api/v1/iot/sensors — only sensor_router is mounted)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/iot/sensors | sensors::create_sensor | partial | iot_auth_tests.rs | auth-reject (401) only |
| GET | /api/v1/iot/sensors | sensors::list_sensors | partial | — | |
| GET | /api/v1/iot/sensors/{id} | sensors::get_sensor | partial | iot_auth_tests.rs | auth-reject only |
| PUT | /api/v1/iot/sensors/{id} | sensors::update_sensor | partial | iot_auth_tests.rs | auth-reject only |
| DELETE | /api/v1/iot/sensors/{id} | sensors::delete_sensor | partial | iot_auth_tests.rs | auth-reject only |
| GET | /api/v1/iot/sensors/{id}/readings | readings::list_readings | partial | iot_auth_tests.rs | auth-reject only |
| POST | /api/v1/iot/sensors/{id}/readings | readings::add_reading | partial | iot_auth_tests.rs | auth-reject only |
| POST | /api/v1/iot/sensors/{id}/readings/batch | readings::add_batch_readings | partial | iot_auth_tests.rs | auth-reject only |
| GET | /api/v1/iot/sensors/{id}/readings/aggregated | readings::get_aggregated_readings | partial | iot_auth_tests.rs | auth-reject only |
| GET | /api/v1/iot/sensors/{id}/thresholds | thresholds::list_thresholds | partial | iot_auth_tests.rs | auth-reject only |
| POST | /api/v1/iot/sensors/{id}/thresholds | thresholds::create_threshold | partial | iot_auth_tests.rs | auth-reject only |
| PUT | /api/v1/iot/sensors/thresholds/{threshold_id} | thresholds::update_threshold | partial | iot_auth_tests.rs | auth-reject only |
| DELETE | /api/v1/iot/sensors/thresholds/{threshold_id} | thresholds::delete_threshold | partial | iot_auth_tests.rs | auth-reject only |
| GET | /api/v1/iot/sensors/{id}/alerts | alerts::list_sensor_alerts | partial | — | |
| POST | /api/v1/iot/sensors/alerts/{alert_id}/acknowledge | alerts::acknowledge_alert | partial | — | |
| POST | /api/v1/iot/sensors/alerts/{alert_id}/resolve | alerts::resolve_alert | partial | iot_auth_tests.rs | auth-reject only |
| GET | /api/v1/iot/sensors/{id}/correlations | correlations::list_correlations | partial | iot_auth_tests.rs | auth-reject only |
| POST | /api/v1/iot/sensors/{id}/correlations | correlations::create_correlation | partial | — | |
| DELETE | /api/v1/iot/sensors/correlations/{correlation_id} | correlations::delete_correlation | partial | iot_auth_tests.rs | auth-reject only |
| GET | /api/v1/iot/sensors/templates | thresholds::list_threshold_templates | partial | — | |
| POST | /api/v1/iot/sensors/templates/{template_id}/apply | thresholds::apply_template | partial | iot_auth_tests.rs | auth-reject only |
| GET | /api/v1/iot/sensors/dashboard | dashboard::get_dashboard | partial | — | |
| GET | /api/v1/iot/sensors/ws | realtime::sensor_ws_handler | partial | — | WebSocket upgrade endpoint |

## vendors/  (mount: /api/v1/vendors)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/vendors | core::create_vendor | partial | — | seeded via repo helper |
| GET | /api/v1/vendors | core::list_vendors | partial | vendor_cross_org_idor_tests.rs | auth/IDOR only |
| GET | /api/v1/vendors/with-details | core::list_vendors_with_details | partial | — | |
| GET | /api/v1/vendors/statistics | core::get_statistics | partial | — | |
| GET | /api/v1/vendors/{id} | core::get_vendor | done | vendor_cross_org_idor_tests.rs | same-org 200 (read own vendor) |
| PATCH | /api/v1/vendors/{id} | core::update_vendor | partial | vendor_cross_org_idor_tests.rs | IDOR-reject only |
| DELETE | /api/v1/vendors/{id} | core::delete_vendor | partial | vendor_cross_org_idor_tests.rs | IDOR-reject only |
| POST | /api/v1/vendors/{id}/preferred | core::set_preferred | partial | — | |
| POST | /api/v1/vendors/{id}/contacts | contacts::add_contact | partial | — | |
| GET | /api/v1/vendors/{id}/contacts | contacts::list_contacts | partial | — | |
| DELETE | /api/v1/vendors/contacts/{contact_id} | contacts::delete_contact | partial | — | |
| POST | /api/v1/vendors/{id}/ratings | contacts::add_rating | partial | — | |
| GET | /api/v1/vendors/{id}/ratings | contacts::list_ratings | partial | — | |
| POST | /api/v1/vendors/contracts | contracts::create_contract | partial | — | |
| GET | /api/v1/vendors/contracts | contracts::list_contracts | partial | — | |
| GET | /api/v1/vendors/contracts/expiring | contracts::get_expiring_contracts | partial | — | |
| GET | /api/v1/vendors/contracts/{id} | contracts::get_contract | partial | — | |
| PATCH | /api/v1/vendors/contracts/{id} | contracts::update_contract | partial | — | |
| DELETE | /api/v1/vendors/contracts/{id} | contracts::delete_contract | partial | — | |
| POST | /api/v1/vendors/invoices | invoices::create_invoice | partial | — | |
| GET | /api/v1/vendors/invoices | invoices::list_invoices | partial | — | |
| GET | /api/v1/vendors/invoices/overdue | invoices::get_overdue_invoices | partial | — | |
| GET | /api/v1/vendors/invoices/summary | invoices::get_invoice_summary | partial | — | |
| GET | /api/v1/vendors/invoices/{id} | invoices::get_invoice | partial | — | |
| PATCH | /api/v1/vendors/invoices/{id} | invoices::update_invoice | partial | — | |
| DELETE | /api/v1/vendors/invoices/{id} | invoices::delete_invoice | partial | — | |
| POST | /api/v1/vendors/invoices/{id}/approve | invoices::approve_invoice | partial | — | |
| POST | /api/v1/vendors/invoices/{id}/reject | invoices::reject_invoice | partial | — | |
| POST | /api/v1/vendors/invoices/{id}/payment | invoices::record_payment | partial | — | |

## Summary
- done: 10 | partial: 249 | stub: 14 | missing: 0 | total: 273
