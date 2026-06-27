# Integrations & API Ecosystem endpoints

Mount prefixes (from `src/lib.rs`): integrations→`/api/v1/integrations`, government_portal→`/api/v1/government-portal`, migration→`/api/v1/migration`, registry→`/api/v1/registry`, portal_webhooks→`/api/v1/webhooks/portals`, ecosystem→`/api/v1/ecosystem`. `public_api` is **unmounted** (ROADMAP PAP-24) → all `stub`.

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `GET /api/v1/integrations/organizations/{org_id}/airbnb/connections` | `airbnb_connections.rs:list_airbnb_connections` | done | `airbnb_connections_routes_tests.rs` | |
| `GET /api/v1/integrations/organizations/{org_id}/airbnb/status` | `install.rs:get_airbnb_status` | done | `integrations_cross_org_idor_tests.rs` | |
| `POST /api/v1/integrations/organizations/{org_id}/airbnb/connect` | `install.rs:connect_airbnb` | done | `integrations_cross_org_idor_tests.rs` | |
| `POST /api/v1/integrations/organizations/{org_id}/airbnb/sync` | `install.rs:sync_airbnb` | done | `integrations_cross_org_idor_tests.rs` | |
| `DELETE /api/v1/integrations/organizations/{org_id}/airbnb` | `install.rs:disconnect_airbnb` | done | `integrations_cross_org_idor_tests.rs` | |
| `POST /api/v1/integrations/organizations/{org_id}/airbnb/direct-connect` | `install.rs:direct_connect_airbnb` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/airbnb/availability-sync` | `install.rs:enqueue_airbnb_availability_sync` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/booking/status` | `install.rs:get_booking_status` | done | `integrations_cross_org_idor_tests.rs` | |
| `POST /api/v1/integrations/organizations/{org_id}/booking/connect` | `install.rs:connect_booking` | done | `booking_connect_encryption_tests.rs` | Also IDOR test |
| `POST /api/v1/integrations/organizations/{org_id}/booking/sync` | `install.rs:sync_booking` | done | `integrations_cross_org_idor_tests.rs` | |
| `POST /api/v1/integrations/organizations/{org_id}/booking/push-availability` | `install.rs:push_booking_availability` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/booking/push-rates` | `install.rs:push_booking_rates` | partial | — | Real handler, no test |
| `DELETE /api/v1/integrations/organizations/{org_id}/booking` | `install.rs:disconnect_booking` | done | `integrations_cross_org_idor_tests.rs` | |
| `GET /api/v1/integrations/organizations/{org_id}/portals` | `install.rs:list_portal_connections` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/portals` | `install.rs:create_portal_connection` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/portals/{id}` | `install.rs:get_portal_connection` | partial | — | Real handler, no test |
| `DELETE /api/v1/integrations/portals/{id}` | `install.rs:delete_portal_connection` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/portal-inquiries` | `install.rs:list_portal_inquiries` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/portal-inquiries/{id}` | `install.rs:get_portal_inquiry` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/portal-inquiries/{id}/read` | `install.rs:mark_inquiry_read` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/portal-inquiries/{id}/archive` | `install.rs:archive_inquiry` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/airbnb/callback` | `oauth.rs:airbnb_oauth_callback` | done | `booking_oauth_csrf_tests.rs` | |
| `POST /api/v1/integrations/organizations/{org_id}/airbnb/token/exchange` | `oauth.rs:airbnb_token_exchange` | done | `airbnb_oauth_routes_tests.rs` | |
| `GET /api/v1/integrations/organizations/{org_id}/airbnb/listings` | `oauth.rs:list_airbnb_listings` | done | `airbnb_oauth_routes_tests.rs` | |
| `GET /api/v1/integrations/organizations/{org_id}/airbnb/reservations` | `oauth.rs:list_airbnb_reservations` | done | `airbnb_oauth_routes_tests.rs` | |
| `POST /api/v1/integrations/organizations/{org_id}/booking/token/exchange` | `oauth.rs:booking_token_exchange` | done | `booking_oauth_routes_tests.rs` | |
| `POST /api/v1/integrations/booking/push` | `webhook.rs:booking_push_notification` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/webhooks/portal/{connection_id}` | `webhook.rs:handle_portal_webhook` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/airbnb/webhook` | `webhook.rs:handle_airbnb_webhook` | done | `airbnb_webhook_routes_tests.rs` | Dedup+recon tested |
| `POST /api/v1/integrations/organizations/{org_id}/booking/listing-push` | `booking_channel.rs:push_booking_listing` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/booking/conflicts` | `booking_channel.rs:get_booking_conflicts` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/airbnb/token/revoke` | `token_rotation.rs:revoke_airbnb_token` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/stats` | `sync.rs:get_integration_stats` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/calendars` | `sync.rs:list_calendar_connections` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/calendars` | `sync.rs:create_calendar_connection` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/calendars/{id}` | `sync.rs:get_calendar_connection` | partial | — | Real handler, no test |
| `PUT /api/v1/integrations/calendars/{id}` | `sync.rs:update_calendar_connection` | partial | — | Real handler, no test |
| `DELETE /api/v1/integrations/calendars/{id}` | `sync.rs:delete_calendar_connection` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/calendars/{id}/sync` | `sync.rs:sync_calendar` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/calendars/{id}/events` | `sync.rs:list_calendar_events` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/calendars/{id}/events` | `sync.rs:create_calendar_event` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/accounting/exports` | `sync.rs:list_accounting_exports` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/accounting/exports` | `sync.rs:create_accounting_export` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/accounting/exports/{id}` | `sync.rs:get_accounting_export` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/accounting/exports/{id}/download` | `sync.rs:download_accounting_export` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/accounting/settings/{system}` | `sync.rs:get_accounting_settings` | partial | — | Real handler, no test |
| `PUT /api/v1/integrations/organizations/{org_id}/accounting/settings/{system}` | `sync.rs:update_accounting_settings` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/esignatures` | `sync.rs:list_esignature_workflows` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/esignatures` | `sync.rs:create_esignature_workflow` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/esignatures/{id}` | `sync.rs:get_esignature_workflow` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/esignatures/{id}/send` | `sync.rs:send_esignature_workflow` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/esignatures/{id}/void` | `sync.rs:void_esignature_workflow` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/esignatures/{id}/remind` | `sync.rs:send_esignature_reminder` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/video/connections` | `sync.rs:list_video_connections` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/video/connections` | `sync.rs:create_video_connection` | partial | — | Real handler, no test |
| `DELETE /api/v1/integrations/video/connections/{id}` | `sync.rs:delete_video_connection` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/organizations/{org_id}/video/meetings` | `sync.rs:list_video_meetings` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/organizations/{org_id}/video/meetings` | `sync.rs:create_video_meeting` | partial | — | Real handler, no test |
| `GET /api/v1/integrations/video/meetings/{id}` | `sync.rs:get_video_meeting` | partial | — | Real handler, no test |
| `PUT /api/v1/integrations/video/meetings/{id}` | `sync.rs:update_video_meeting` | partial | — | Real handler, no test |
| `DELETE /api/v1/integrations/video/meetings/{id}` | `sync.rs:delete_video_meeting` | partial | — | Real handler, no test |
| `POST /api/v1/integrations/video/meetings/{id}/start` | `sync.rs:start_video_meeting` | partial | — | Real handler, no test |
| `POST /api/v1/webhooks/portals/reality-portal/views` | `portal_webhooks.rs:reality_portal_views_webhook` | done | `portal_webhook_signature_tests.rs` | |
| `POST /api/v1/webhooks/portals/sreality/views` | `portal_webhooks.rs:sreality_views_webhook` | partial | — | Real handler, no test |
| `POST /api/v1/webhooks/portals/sreality/inquiries` | `portal_webhooks.rs:sreality_inquiry_webhook` | partial | — | Real handler, no test |
| `POST /api/v1/webhooks/portals/bezrealitky/views` | `portal_webhooks.rs:bezrealitky_views_webhook` | partial | — | Real handler, no test |
| `POST /api/v1/webhooks/portals/bezrealitky/inquiries` | `portal_webhooks.rs:bezrealitky_inquiry_webhook` | partial | — | Real handler, no test |
| `POST /api/v1/webhooks/portals/nehnutelnosti/views` | `portal_webhooks.rs:nehnutelnosti_views_webhook` | partial | — | Real handler, no test |
| `POST /api/v1/webhooks/portals/nehnutelnosti/inquiries` | `portal_webhooks.rs:nehnutelnosti_inquiry_webhook` | partial | — | Real handler, no test |
| `POST /api/v1/webhooks/portals/{portal}/events` | `portal_webhooks.rs:generic_portal_webhook` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/marketplace` | `api_ecosystem.rs:list_marketplace_integrations` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/marketplace` | `api_ecosystem.rs:create_marketplace_integration` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/marketplace/{id}` | `api_ecosystem.rs:get_marketplace_integration` | partial | — | Real handler, no test |
| `PUT /api/v1/ecosystem/marketplace/{id}` | `api_ecosystem.rs:update_marketplace_integration` | partial | — | Real handler, no test |
| `DELETE /api/v1/ecosystem/marketplace/{id}` | `api_ecosystem.rs:delete_marketplace_integration` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/marketplace/categories` | `api_ecosystem.rs:list_integration_categories` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/marketplace/{id}/ratings` | `api_ecosystem.rs:list_integration_ratings` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/marketplace/{id}/ratings` | `api_ecosystem.rs:create_integration_rating` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/integrations` | `api_ecosystem.rs:list_organization_integrations` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/organizations/{org_id}/integrations` | `api_ecosystem.rs:install_integration` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/integrations/{id}` | `api_ecosystem.rs:get_organization_integration` | partial | — | Real handler, no test |
| `PUT /api/v1/ecosystem/organizations/{org_id}/integrations/{id}` | `api_ecosystem.rs:update_organization_integration` | partial | — | Real handler, no test |
| `DELETE /api/v1/ecosystem/organizations/{org_id}/integrations/{id}` | `api_ecosystem.rs:uninstall_integration` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/organizations/{org_id}/integrations/{id}/sync` | `api_ecosystem.rs:sync_integration` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/connectors` | `api_ecosystem.rs:list_connectors` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/connectors` | `api_ecosystem.rs:create_connector` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/connectors/{id}` | `api_ecosystem.rs:get_connector` | partial | — | Real handler, no test |
| `PUT /api/v1/ecosystem/connectors/{id}` | `api_ecosystem.rs:update_connector` | partial | — | Real handler, no test |
| `DELETE /api/v1/ecosystem/connectors/{id}` | `api_ecosystem.rs:delete_connector` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/connectors/{id}/actions` | `api_ecosystem.rs:list_connector_actions` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/connectors/{id}/actions` | `api_ecosystem.rs:create_connector_action` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/connector-logs` | `api_ecosystem.rs:list_connector_logs` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/webhooks` | `api_ecosystem.rs:list_enhanced_webhooks` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/organizations/{org_id}/webhooks` | `api_ecosystem.rs:create_enhanced_webhook` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/webhooks/{id}` | `api_ecosystem.rs:get_enhanced_webhook` | partial | — | Real handler, no test |
| `PUT /api/v1/ecosystem/webhooks/{id}` | `api_ecosystem.rs:update_enhanced_webhook` | partial | — | Real handler, no test |
| `DELETE /api/v1/ecosystem/webhooks/{id}` | `api_ecosystem.rs:delete_enhanced_webhook` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/webhooks/{id}/test` | `api_ecosystem.rs:test_enhanced_webhook` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/webhooks/{id}/logs` | `api_ecosystem.rs:list_webhook_delivery_logs` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/webhooks/{id}/stats` | `api_ecosystem.rs:get_enhanced_webhook_stats` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/webhooks/events` | `api_ecosystem.rs:list_webhook_event_types` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/prebuilt` | `api_ecosystem.rs:list_prebuilt_connections` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/organizations/{org_id}/prebuilt` | `api_ecosystem.rs:create_prebuilt_connection` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}` | `api_ecosystem.rs:get_prebuilt_connection` | partial | — | Real handler, no test |
| `PUT /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}` | `api_ecosystem.rs:update_prebuilt_connection` | partial | — | Real handler, no test |
| `DELETE /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}` | `api_ecosystem.rs:delete_prebuilt_connection` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}/sync` | `api_ecosystem.rs:sync_prebuilt_connection` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}/oauth` | `api_ecosystem.rs:get_prebuilt_oauth_url` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/organizations/{org_id}/prebuilt/{integration_type}/oauth/callback` | `api_ecosystem.rs:handle_prebuilt_oauth_callback` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/developers/register` | `api_ecosystem.rs:register_developer` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/developers/{id}` | `api_ecosystem.rs:get_developer_registration` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/developers/{id}/review` | `api_ecosystem.rs:review_developer_registration` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/developers/{id}/keys` | `api_ecosystem.rs:list_developer_api_keys` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/developers/{id}/keys` | `api_ecosystem.rs:create_developer_api_key` | partial | — | Real handler, no test |
| `DELETE /api/v1/ecosystem/developers/{id}/keys/{key_id}` | `api_ecosystem.rs:revoke_developer_api_key` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/developers/{id}/keys/{key_id}/rotate` | `api_ecosystem.rs:rotate_developer_api_key` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/developers/{id}/usage` | `api_ecosystem.rs:get_developer_usage_stats` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/developers/{id}/sandbox` | `api_ecosystem.rs:create_sandbox_environment` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/developers/{id}/sandbox` | `api_ecosystem.rs:get_sandbox_environment` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/developers/{id}/sandbox/test` | `api_ecosystem.rs:test_sandbox_request` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/docs` | `api_ecosystem.rs:list_api_documentation` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/docs` | `api_ecosystem.rs:create_api_documentation` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/docs/{slug}` | `api_ecosystem.rs:get_api_documentation` | partial | — | Real handler, no test |
| `PUT /api/v1/ecosystem/docs/{slug}` | `api_ecosystem.rs:update_api_documentation` | partial | — | Real handler, no test |
| `DELETE /api/v1/ecosystem/docs/{slug}` | `api_ecosystem.rs:delete_api_documentation` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/docs/{slug}/code-samples` | `api_ecosystem.rs:list_code_samples` | partial | — | Real handler, no test |
| `POST /api/v1/ecosystem/docs/{slug}/code-samples` | `api_ecosystem.rs:create_code_sample` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/portal/stats` | `api_ecosystem.rs:get_developer_portal_stats` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/dashboard` | `api_ecosystem.rs:get_ecosystem_dashboard` | partial | — | Real handler, no test |
| `GET /api/v1/ecosystem/organizations/{org_id}/stats` | `api_ecosystem.rs:get_ecosystem_statistics` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/connections` | `government_portal.rs:list_connections` | partial | — | Real handler, no test |
| `POST /api/v1/government-portal/connections` | `government_portal.rs:create_connection` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/connections/{id}` | `government_portal.rs:get_connection` | partial | — | Real handler, no test |
| `PUT /api/v1/government-portal/connections/{id}` | `government_portal.rs:update_connection` | partial | — | Real handler, no test |
| `DELETE /api/v1/government-portal/connections/{id}` | `government_portal.rs:delete_connection` | partial | — | Real handler, no test |
| `POST /api/v1/government-portal/connections/{id}/test` | `government_portal.rs:test_connection` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/templates` | `government_portal.rs:list_templates` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/templates/{id}` | `government_portal.rs:get_template` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/submissions` | `government_portal.rs:list_submissions` | partial | — | Real handler, no test |
| `POST /api/v1/government-portal/submissions` | `government_portal.rs:create_submission` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/submissions/{id}` | `government_portal.rs:get_submission` | partial | — | Real handler, no test |
| `PUT /api/v1/government-portal/submissions/{id}` | `government_portal.rs:update_submission` | partial | — | Real handler, no test |
| `POST /api/v1/government-portal/submissions/{id}/validate` | `government_portal.rs:validate_submission` | partial | — | Real handler, no test |
| `POST /api/v1/government-portal/submissions/{id}/submit` | `government_portal.rs:submit_submission` | partial | — | Real handler, no test |
| `POST /api/v1/government-portal/submissions/{id}/cancel` | `government_portal.rs:cancel_submission` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/submissions/{id}/audit` | `government_portal.rs:get_submission_audit` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/submissions/{id}/attachments` | `government_portal.rs:list_attachments` | partial | — | Real handler, no test |
| `POST /api/v1/government-portal/submissions/{id}/attachments` | `government_portal.rs:add_attachment` | partial | — | Real handler, no test |
| `DELETE /api/v1/government-portal/submissions/{submission_id}/attachments/{attachment_id}` | `government_portal.rs:delete_attachment` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/schedules` | `government_portal.rs:list_schedules` | partial | — | Real handler, no test |
| `POST /api/v1/government-portal/schedules` | `government_portal.rs:create_schedule` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/schedules/{id}` | `government_portal.rs:get_schedule` | partial | — | Real handler, no test |
| `PUT /api/v1/government-portal/schedules/{id}` | `government_portal.rs:update_schedule` | partial | — | Real handler, no test |
| `DELETE /api/v1/government-portal/schedules/{id}` | `government_portal.rs:delete_schedule` | partial | — | Real handler, no test |
| `GET /api/v1/government-portal/stats` | `government_portal.rs:get_stats` | partial | — | Real handler, no test |
| `POST /api/v1/registry/pets` | `registry.rs:create_pet_registration` | partial | — | Real handler, no test |
| `GET /api/v1/registry/pets` | `registry.rs:list_pet_registrations` | partial | — | Real handler, no test |
| `GET /api/v1/registry/pets/{id}` | `registry.rs:get_pet_registration` | partial | — | Real handler, no test |
| `PUT /api/v1/registry/pets/{id}` | `registry.rs:update_pet_registration` | partial | — | Real handler, no test |
| `DELETE /api/v1/registry/pets/{id}` | `registry.rs:delete_pet_registration` | partial | — | Real handler, no test |
| `POST /api/v1/registry/pets/{id}/review` | `registry.rs:review_pet_registration` | partial | — | Real handler, no test |
| `POST /api/v1/registry/vehicles` | `registry.rs:create_vehicle_registration` | partial | — | Real handler, no test |
| `GET /api/v1/registry/vehicles` | `registry.rs:list_vehicle_registrations` | partial | — | Real handler, no test |
| `GET /api/v1/registry/vehicles/{id}` | `registry.rs:get_vehicle_registration` | partial | — | Real handler, no test |
| `PUT /api/v1/registry/vehicles/{id}` | `registry.rs:update_vehicle_registration` | partial | — | Real handler, no test |
| `DELETE /api/v1/registry/vehicles/{id}` | `registry.rs:delete_vehicle_registration` | partial | — | Real handler, no test |
| `POST /api/v1/registry/vehicles/{id}/review` | `registry.rs:review_vehicle_registration` | partial | — | Real handler, no test |
| `POST /api/v1/registry/parking-spots` | `registry.rs:create_parking_spot` | partial | — | Real handler, no test |
| `GET /api/v1/registry/parking-spots` | `registry.rs:list_parking_spots` | partial | — | Real handler, no test |
| `GET /api/v1/registry/parking-spots/{id}` | `registry.rs:get_parking_spot` | partial | — | Real handler, no test |
| `PUT /api/v1/registry/parking-spots/{id}` | `registry.rs:update_parking_spot` | partial | — | Real handler, no test |
| `DELETE /api/v1/registry/parking-spots/{id}` | `registry.rs:delete_parking_spot` | partial | — | Real handler, no test |
| `GET /api/v1/registry/buildings/{building_id}/rules` | `registry.rs:get_registry_rules` | partial | — | Real handler, no test |
| `PUT /api/v1/registry/buildings/{building_id}/rules` | `registry.rs:update_registry_rules` | partial | — | Real handler, no test |
| `GET /api/v1/registry/buildings/{building_id}/statistics` | `registry.rs:get_registry_statistics` | partial | — | Real handler, no test |
| `GET /api/v1/migration/templates` | `migration.rs:list_templates` | stub | — | Mock data, state unused |
| `POST /api/v1/migration/templates` | `migration.rs:create_template` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/templates/system` | `migration.rs:list_system_templates` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/templates/{template_id}` | `migration.rs:get_template` | stub | — | Mock data, state unused |
| `PUT /api/v1/migration/templates/{template_id}` | `migration.rs:update_template` | stub | — | Mock data, state unused |
| `DELETE /api/v1/migration/templates/{template_id}` | `migration.rs:delete_template` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/templates/{template_id}/download` | `migration.rs:download_template` | stub | — | Mock data, state unused |
| `POST /api/v1/migration/templates/{template_id}/duplicate` | `migration.rs:duplicate_template` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/categories/import` | `migration.rs:get_import_categories` | stub | — | Mock data, state unused |
| `POST /api/v1/migration/import/upload` | `migration.rs:upload_import_file` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/import/jobs` | `migration.rs:list_import_jobs` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/import/jobs/{job_id}` | `migration.rs:get_import_job_status` | stub | — | Mock data, state unused |
| `POST /api/v1/migration/import/jobs/{job_id}/cancel` | `migration.rs:cancel_import_job` | stub | — | Mock data, state unused |
| `POST /api/v1/migration/import/jobs/{job_id}/retry` | `migration.rs:retry_import_job` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/import/jobs/{job_id}/errors` | `migration.rs:get_import_job_errors` | stub | — | Mock data, state unused |
| `POST /api/v1/migration/export` | `migration.rs:request_migration_export` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/export/{export_id}` | `migration.rs:get_export_status` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/export/{export_id}/download` | `migration.rs:download_export` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/export/history` | `migration.rs:get_export_history` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/categories/export` | `migration.rs:get_export_categories` | stub | — | Mock data, state unused |
| `GET /api/v1/migration/import/jobs/{job_id}/preview` | `migration.rs:get_import_preview` | stub | — | Mock data, state unused |
| `POST /api/v1/migration/import/jobs/{job_id}/approve` | `migration.rs:approve_import` | stub | — | Mock data, state unused |
| `POST /api/v1/migration/import/jobs/{job_id}/validate` | `migration.rs:validate_import` | stub | — | Mock data, state unused |
| `POST /api/v1/developer/accounts` | `public_api.rs:create_developer_account` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/accounts/me` | `public_api.rs:get_my_developer_account` | stub | — | unmounted ROADMAP |
| `PATCH /api/v1/developer/accounts/me` | `public_api.rs:update_my_developer_account` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/accounts/me/usage` | `public_api.rs:get_my_usage_summary` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/keys` | `public_api.rs:create_api_key` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/keys` | `public_api.rs:list_api_keys` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/keys/{id}` | `public_api.rs:get_api_key` | stub | — | unmounted ROADMAP |
| `PATCH /api/v1/developer/keys/{id}` | `public_api.rs:update_api_key` | stub | — | unmounted ROADMAP |
| `DELETE /api/v1/developer/keys/{id}` | `public_api.rs:revoke_api_key` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/keys/{id}/rotate` | `public_api.rs:rotate_api_key` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/keys/{id}/usage` | `public_api.rs:get_api_key_usage` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/docs/endpoints` | `public_api.rs:list_api_endpoints` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/docs/endpoints/{id}` | `public_api.rs:get_api_endpoint` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/docs/changelog` | `public_api.rs:list_api_changelog` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/docs/openapi` | `public_api.rs:get_openapi_spec` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/sandbox` | `public_api.rs:create_sandbox` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/sandbox/test` | `public_api.rs:test_sandbox_request` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/sandbox/{id}` | `public_api.rs:get_sandbox` | stub | — | unmounted ROADMAP |
| `DELETE /api/v1/developer/sandbox/{id}` | `public_api.rs:delete_sandbox` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/webhooks` | `public_api.rs:create_webhook` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/webhooks` | `public_api.rs:list_webhooks` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/webhooks/{id}` | `public_api.rs:get_webhook` | stub | — | unmounted ROADMAP |
| `PATCH /api/v1/developer/webhooks/{id}` | `public_api.rs:update_webhook` | stub | — | unmounted ROADMAP |
| `DELETE /api/v1/developer/webhooks/{id}` | `public_api.rs:delete_webhook` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/webhooks/{id}/test` | `public_api.rs:test_webhook` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/webhooks/{id}/rotate-secret` | `public_api.rs:rotate_webhook_secret` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/webhooks/{id}/deliveries` | `public_api.rs:list_webhook_deliveries` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/webhooks/events` | `public_api.rs:list_webhook_event_types` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/rate-limits/status` | `public_api.rs:get_rate_limit_status` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/rate-limits/tiers` | `public_api.rs:list_rate_limit_tiers` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/sdks` | `public_api.rs:list_sdk_languages` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/sdks/{language}` | `public_api.rs:get_sdk_info` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/sdks/{language}/download` | `public_api.rs:download_sdk` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/sdks/{language}/versions` | `public_api.rs:list_sdk_versions` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/admin/developers` | `public_api.rs:list_developers` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/admin/developers/{id}` | `public_api.rs:get_developer` | stub | — | unmounted ROADMAP |
| `PATCH /api/v1/developer/admin/developers/{id}` | `public_api.rs:update_developer` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/admin/developers/{id}/verify` | `public_api.rs:verify_developer` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/admin/developers/{id}/suspend` | `public_api.rs:suspend_developer` | stub | — | unmounted ROADMAP |
| `POST /api/v1/developer/admin/rate-limits` | `public_api.rs:create_rate_limit_config` | stub | — | unmounted ROADMAP |
| `PATCH /api/v1/developer/admin/rate-limits/{id}` | `public_api.rs:update_rate_limit_config` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/admin/stats` | `public_api.rs:get_portal_stats` | stub | — | unmounted ROADMAP |
| `GET /api/v1/developer/admin/request-logs` | `public_api.rs:list_request_logs` | stub | — | unmounted ROADMAP |

## Tally
done: 16  partial: 159  stub: 66  missing: 0  total: 241
