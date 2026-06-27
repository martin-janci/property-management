# Leasing, Listings, Vendors & Buildings endpoints

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `POST /api/v1/listings` | `listings.rs:create_listing` | partial | — | real handler, no test |
| `GET /api/v1/listings` | `listings.rs:list_listings` | partial | — | real handler, no test |
| `POST /api/v1/listings/from-unit` | `listings.rs:create_from_unit` | partial | — | real handler, no test |
| `GET /api/v1/listings/statistics` | `listings.rs:get_statistics` | partial | — | real handler, no test |
| `GET /api/v1/listings/syndication/dashboard` | `listings.rs:get_syndication_dashboard` | partial | — | real handler, no test |
| `GET /api/v1/listings/syndication/stats` | `listings.rs:get_organization_syndication_stats` | partial | — | real handler, no test |
| `GET /api/v1/listings/{id}` | `listings.rs:get_listing` | partial | — | real handler, no test |
| `PUT /api/v1/listings/{id}` | `listings.rs:update_listing` | partial | — | real handler, no test |
| `DELETE /api/v1/listings/{id}` | `listings.rs:delete_listing` | partial | — | real handler, no test |
| `PUT /api/v1/listings/{id}/status` | `listings.rs:update_status` | partial | — | real handler, no test |
| `POST /api/v1/listings/{id}/publish` | `listings.rs:publish_listing` | partial | — | real handler, no test |
| `POST /api/v1/listings/{id}/global-publish` | `listings.rs:global_publish` | done | `listings_global_publish_authz_tests.rs` | authz tested |
| `POST /api/v1/listings/{id}/global-unpublish` | `listings.rs:global_unpublish` | done | `listings_global_publish_authz_tests.rs` | authz tested |
| `GET /api/v1/listings/{id}/photos` | `listings.rs:get_photos` | partial | — | real handler, no test |
| `POST /api/v1/listings/{id}/photos` | `listings.rs:add_photo` | partial | — | real handler, no test |
| `POST /api/v1/listings/{id}/photos/reorder` | `listings.rs:reorder_photos` | partial | — | real handler, no test |
| `DELETE /api/v1/listings/{id}/photos/{photo_id}` | `listings.rs:delete_photo` | partial | — | real handler, no test |
| `GET /api/v1/listings/{id}/syndications` | `listings.rs:get_syndications` | partial | — | real handler, no test |
| `GET /api/v1/listings/{id}/syndication/status` | `listings.rs:get_listing_syndication_status` | partial | — | real handler, no test |
| `GET /api/v1/rentals/statistics` | `rentals.rs:get_statistics` | partial | — | real handler, no test |
| `GET /api/v1/rentals/sync-status` | `rentals.rs:get_sync_status` | partial | — | real handler, no test |
| `GET /api/v1/rentals/connections` | `rentals.rs:list_connections` | partial | — | real handler, no test |
| `POST /api/v1/rentals/connections` | `rentals.rs:create_connection` | partial | — | real handler, no test |
| `GET /api/v1/rentals/connections/{id}` | `rentals.rs:get_connection` | partial | — | real handler, no test |
| `PUT /api/v1/rentals/connections/{id}` | `rentals.rs:update_connection` | partial | — | real handler, no test |
| `DELETE /api/v1/rentals/connections/{id}` | `rentals.rs:delete_connection` | partial | — | real handler, no test |
| `GET /api/v1/rentals/units/{unit_id}/connections` | `rentals.rs:get_unit_connections` | partial | — | real handler, no test |
| `GET /api/v1/rentals/bookings` | `rentals.rs:list_bookings` | partial | — | real handler, no test |
| `POST /api/v1/rentals/bookings` | `rentals.rs:create_booking` | partial | — | real handler, no test |
| `GET /api/v1/rentals/bookings/{id}` | `rentals.rs:get_booking` | partial | — | real handler, no test |
| `PUT /api/v1/rentals/bookings/{id}` | `rentals.rs:update_booking` | partial | — | real handler, no test |
| `PUT /api/v1/rentals/bookings/{id}/status` | `rentals.rs:update_booking_status` | partial | — | real handler, no test |
| `GET /api/v1/rentals/bookings/{id}/guests` | `rentals.rs:get_booking_with_guests` | partial | — | real handler, no test |
| `GET /api/v1/rentals/calendar/{unit_id}` | `rentals.rs:get_calendar` | partial | — | real handler, no test |
| `GET /api/v1/rentals/calendar/{unit_id}/availability` | `rentals.rs:check_availability` | partial | — | real handler, no test |
| `POST /api/v1/rentals/calendar/blocks` | `rentals.rs:create_calendar_block` | partial | — | real handler, no test |
| `DELETE /api/v1/rentals/calendar/blocks/{id}` | `rentals.rs:delete_calendar_block` | partial | — | real handler, no test |
| `POST /api/v1/rentals/guests` | `rentals.rs:create_guest` | partial | — | real handler, no test |
| `GET /api/v1/rentals/guests/{id}` | `rentals.rs:get_guest` | partial | — | real handler, no test |
| `PUT /api/v1/rentals/guests/{id}` | `rentals.rs:update_guest` | partial | — | real handler, no test |
| `DELETE /api/v1/rentals/guests/{id}` | `rentals.rs:delete_guest` | partial | — | real handler, no test |
| `POST /api/v1/rentals/guests/{id}/register` | `rentals.rs:register_guest` | partial | — | real handler, no test |
| `GET /api/v1/rentals/checkin-reminders` | `rentals.rs:get_checkin_reminders` | partial | — | real handler, no test |
| `GET /api/v1/rentals/reports` | `rentals.rs:list_reports` | partial | — | real handler, no test |
| `POST /api/v1/rentals/reports/preview` | `rentals.rs:generate_report_preview` | partial | — | real handler, no test |
| `POST /api/v1/rentals/reports` | `rentals.rs:create_report` | partial | — | real handler, no test |
| `GET /api/v1/rentals/reports/{id}` | `rentals.rs:get_report` | partial | — | real handler, no test |
| `POST /api/v1/rentals/reports/{id}/submit` | `rentals.rs:submit_report` | partial | — | real handler, no test |
| `POST /api/v1/rentals/ical` | `rentals.rs:create_ical_feed` | partial | — | real handler, no test |
| `PUT /api/v1/rentals/ical/{id}` | `rentals.rs:update_ical_feed` | partial | — | real handler, no test |
| `DELETE /api/v1/rentals/ical/{id}` | `rentals.rs:delete_ical_feed` | partial | — | real handler, no test |
| `GET /api/v1/rentals/units/{unit_id}/ical` | `rentals.rs:get_unit_ical_feeds` | partial | — | real handler, no test |
| `POST /api/v1/leases/applications` | `leases.rs:create_application` | partial | — | real handler, no test |
| `GET /api/v1/leases/applications` | `leases.rs:list_applications` | partial | — | real handler, no test |
| `GET /api/v1/leases/applications/{id}` | `leases.rs:get_application` | partial | — | real handler, no test |
| `PUT /api/v1/leases/applications/{id}` | `leases.rs:update_application` | partial | — | real handler, no test |
| `POST /api/v1/leases/applications/{id}/submit` | `leases.rs:submit_application` | partial | — | real handler, no test |
| `POST /api/v1/leases/applications/{id}/review` | `leases.rs:review_application` | done | `lease_review_rbac_tests.rs` | RBAC tested |
| `POST /api/v1/leases/applications/{id}/screening` | `leases.rs:initiate_screening` | partial | — | real handler, no test |
| `POST /api/v1/leases/applications/{id}/screening/consent` | `leases.rs:record_consent` | partial | — | real handler, no test |
| `GET /api/v1/leases/screenings/{id}` | `leases.rs:get_screening` | partial | — | real handler, no test |
| `PATCH /api/v1/leases/screenings/{id}/result` | `leases.rs:update_screening_result` | partial | — | real handler, no test |
| `POST /api/v1/leases/templates` | `leases.rs:create_template` | partial | — | real handler, no test |
| `GET /api/v1/leases/templates` | `leases.rs:list_templates` | partial | — | real handler, no test |
| `GET /api/v1/leases/templates/{id}` | `leases.rs:get_template` | partial | — | real handler, no test |
| `PUT /api/v1/leases/templates/{id}` | `leases.rs:update_template` | partial | — | real handler, no test |
| `POST /api/v1/leases` | `leases.rs:create_lease` | partial | — | real handler, no test |
| `GET /api/v1/leases` | `leases.rs:list_leases` | partial | — | real handler, no test |
| `GET /api/v1/leases/{id}` | `leases.rs:get_lease` | partial | — | real handler, no test |
| `PUT /api/v1/leases/{id}` | `leases.rs:update_lease` | partial | — | real handler, no test |
| `POST /api/v1/leases/{id}/terminate` | `leases.rs:terminate_lease` | partial | — | real handler, no test |
| `POST /api/v1/leases/{id}/renew` | `leases.rs:renew_lease` | partial | — | real handler, no test |
| `POST /api/v1/leases/{id}/send-for-signature` | `leases.rs:send_lease_for_signature` | done | `lease_send_for_signature_rbac_tests.rs` | RBAC tested |
| `POST /api/v1/leases/{id}/amendments` | `leases.rs:create_amendment` | partial | — | real handler, no test |
| `GET /api/v1/leases/{id}/amendments` | `leases.rs:list_amendments` | partial | — | real handler, no test |
| `POST /api/v1/leases/{id}/payments` | `leases.rs:record_payment` | partial | — | real handler, no test |
| `GET /api/v1/leases/{id}/payments` | `leases.rs:list_payments` | partial | — | real handler, no test |
| `GET /api/v1/leases/{id}/payments/summary` | `leases.rs:get_payment_summary` | partial | — | real handler, no test |
| `POST /api/v1/leases/{id}/reminders` | `leases.rs:create_reminder` | partial | — | real handler, no test |
| `GET /api/v1/leases/{id}/reminders` | `leases.rs:list_reminders` | partial | — | real handler, no test |
| `GET /api/v1/leases/expiring` | `leases.rs:get_expiring_leases` | partial | — | real handler, no test |
| `GET /api/v1/leases/statistics` | `leases.rs:get_statistics` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/providers` | `marketplace.rs:create_profile` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/providers` | `marketplace.rs:search_providers` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/providers/me` | `marketplace.rs:get_my_profile` | partial | — | real handler, no test |
| `PATCH /api/v1/marketplace/providers/me` | `marketplace.rs:update_my_profile` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/providers/me/dashboard` | `marketplace.rs:get_provider_dashboard` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/providers/statistics` | `marketplace.rs:get_marketplace_statistics` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/providers/{id}` | `marketplace.rs:get_provider` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/providers/{id}/complete` | `marketplace.rs:get_provider_complete` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/rfqs` | `marketplace.rs:create_rfq` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/rfqs` | `marketplace.rs:list_rfqs` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/rfqs/{id}` | `marketplace.rs:get_rfq` | done | `marketplace_voting_investor_cross_org_idor_tests.rs` | IDOR tested |
| `PATCH /api/v1/marketplace/rfqs/{id}` | `marketplace.rs:update_rfq` | partial | — | real handler, no test |
| `DELETE /api/v1/marketplace/rfqs/{id}` | `marketplace.rs:delete_rfq` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/rfqs/{id}/quotes` | `marketplace.rs:list_rfq_quotes` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/rfqs/{id}/compare` | `marketplace.rs:compare_quotes` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/rfqs/{id}/award` | `marketplace.rs:award_quote` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/rfqs/{id}/cancel` | `marketplace.rs:cancel_rfq` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/quotes` | `marketplace.rs:submit_quote` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/quotes/my` | `marketplace.rs:list_my_quotes` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/quotes/{id}` | `marketplace.rs:get_quote` | partial | — | real handler, no test |
| `PATCH /api/v1/marketplace/quotes/{id}` | `marketplace.rs:update_quote` | partial | — | real handler, no test |
| `DELETE /api/v1/marketplace/quotes/{id}` | `marketplace.rs:withdraw_quote` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/invitations` | `marketplace.rs:list_my_invitations` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/invitations/{id}/view` | `marketplace.rs:mark_invitation_viewed` | done | `marketplace_voting_investor_cross_org_idor_tests.rs` | IDOR tested |
| `POST /api/v1/marketplace/invitations/{id}/decline` | `marketplace.rs:decline_invitation` | done | `marketplace_voting_investor_cross_org_idor_tests.rs` | IDOR tested |
| `POST /api/v1/marketplace/verifications` | `marketplace.rs:submit_verification` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/verifications` | `marketplace.rs:list_verifications` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/verifications/queue` | `marketplace.rs:get_verification_queue` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/verifications/expiring` | `marketplace.rs:get_expiring_verifications` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/verifications/{id}` | `marketplace.rs:get_verification` | done | `marketplace_voting_investor_cross_org_idor_tests.rs` | IDOR tested |
| `POST /api/v1/marketplace/verifications/{id}/review` | `marketplace.rs:review_verification` | done | `marketplace_voting_investor_cross_org_idor_tests.rs` | RBAC tested |
| `GET /api/v1/marketplace/providers/{id}/badges` | `marketplace.rs:list_provider_badges` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/providers/{id}/badges` | `marketplace.rs:award_badge` | partial | — | real handler, no test |
| `DELETE /api/v1/marketplace/badges/{id}` | `marketplace.rs:revoke_badge` | done | `marketplace_voting_investor_cross_org_idor_tests.rs` | RBAC tested |
| `POST /api/v1/marketplace/providers/{id}/reviews` | `marketplace.rs:create_review` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/providers/{id}/reviews` | `marketplace.rs:list_provider_reviews` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/providers/{id}/ratings` | `marketplace.rs:get_rating_breakdown` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/reviews` | `marketplace.rs:list_reviews` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/reviews/{id}` | `marketplace.rs:get_review` | partial | — | real handler, no test |
| `PATCH /api/v1/marketplace/reviews/{id}` | `marketplace.rs:update_review` | partial | — | real handler, no test |
| `DELETE /api/v1/marketplace/reviews/{id}` | `marketplace.rs:delete_review` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/reviews/{id}/respond` | `marketplace.rs:respond_to_review` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/reviews/{id}/moderate` | `marketplace.rs:moderate_review` | partial | — | real handler, no test |
| `POST /api/v1/marketplace/reviews/{id}/helpful` | `marketplace.rs:mark_review_helpful` | partial | — | real handler, no test |
| `GET /api/v1/marketplace/dashboard` | `marketplace.rs:get_manager_dashboard` | partial | — | real handler, no test |
| `POST /api/v1/vendors` | `vendors.rs:create_vendor` | done | `vendor_cross_org_idor_tests.rs` | IDOR tested |
| `GET /api/v1/vendors` | `vendors.rs:list_vendors` | done | `vendor_cross_org_idor_tests.rs` | IDOR tested |
| `GET /api/v1/vendors/with-details` | `vendors.rs:list_vendors_with_details` | partial | — | real handler, no test |
| `GET /api/v1/vendors/statistics` | `vendors.rs:get_statistics` | partial | — | real handler, no test |
| `GET /api/v1/vendors/{id}` | `vendors.rs:get_vendor` | done | `vendor_cross_org_idor_tests.rs` | IDOR tested |
| `PATCH /api/v1/vendors/{id}` | `vendors.rs:update_vendor` | done | `vendor_cross_org_idor_tests.rs` | IDOR tested |
| `DELETE /api/v1/vendors/{id}` | `vendors.rs:delete_vendor` | done | `vendor_cross_org_idor_tests.rs` | IDOR tested |
| `POST /api/v1/vendors/{id}/preferred` | `vendors.rs:set_preferred` | partial | — | real handler, no test |
| `POST /api/v1/vendors/{id}/contacts` | `vendors.rs:add_contact` | partial | — | real handler, no test |
| `GET /api/v1/vendors/{id}/contacts` | `vendors.rs:list_contacts` | partial | — | real handler, no test |
| `DELETE /api/v1/vendors/contacts/{contact_id}` | `vendors.rs:delete_contact` | partial | — | real handler, no test |
| `POST /api/v1/vendors/{id}/ratings` | `vendors.rs:add_rating` | partial | — | real handler, no test |
| `GET /api/v1/vendors/{id}/ratings` | `vendors.rs:list_ratings` | partial | — | real handler, no test |
| `POST /api/v1/vendors/contracts` | `vendors.rs:create_contract` | partial | — | real handler, no test |
| `GET /api/v1/vendors/contracts` | `vendors.rs:list_contracts` | partial | — | real handler, no test |
| `GET /api/v1/vendors/contracts/expiring` | `vendors.rs:get_expiring_contracts` | partial | — | real handler, no test |
| `GET /api/v1/vendors/contracts/{id}` | `vendors.rs:get_contract` | partial | — | real handler, no test |
| `PATCH /api/v1/vendors/contracts/{id}` | `vendors.rs:update_contract` | partial | — | real handler, no test |
| `DELETE /api/v1/vendors/contracts/{id}` | `vendors.rs:delete_contract` | partial | — | real handler, no test |
| `POST /api/v1/vendors/invoices` | `vendors.rs:create_invoice` | partial | — | real handler, no test |
| `GET /api/v1/vendors/invoices` | `vendors.rs:list_invoices` | partial | — | real handler, no test |
| `GET /api/v1/vendors/invoices/overdue` | `vendors.rs:get_overdue_invoices` | partial | — | real handler, no test |
| `GET /api/v1/vendors/invoices/summary` | `vendors.rs:get_invoice_summary` | partial | — | real handler, no test |
| `GET /api/v1/vendors/invoices/{id}` | `vendors.rs:get_invoice` | partial | — | real handler, no test |
| `PATCH /api/v1/vendors/invoices/{id}` | `vendors.rs:update_invoice` | partial | — | real handler, no test |
| `DELETE /api/v1/vendors/invoices/{id}` | `vendors.rs:delete_invoice` | partial | — | real handler, no test |
| `POST /api/v1/vendors/invoices/{id}/approve` | `vendors.rs:approve_invoice` | partial | — | real handler, no test |
| `POST /api/v1/vendors/invoices/{id}/reject` | `vendors.rs:reject_invoice` | partial | — | real handler, no test |
| `POST /api/v1/vendors/invoices/{id}/payment` | `vendors.rs:record_payment` | partial | — | real handler, no test |
| `GET /api/v1/vendor-portal/dashboard/stats` | `vendor_portal.rs:get_dashboard_stats` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/vendor-portal/jobs` | `vendor_portal.rs:list_jobs` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/vendor-portal/jobs/{job_id}` | `vendor_portal.rs:get_job_details` | stub | — | unmounted ROADMAP, 501 |
| `POST /api/v1/vendor-portal/jobs/{job_id}/accept` | `vendor_portal.rs:accept_job` | stub | — | unmounted ROADMAP, 501 |
| `POST /api/v1/vendor-portal/jobs/{job_id}/decline` | `vendor_portal.rs:decline_job` | stub | — | unmounted ROADMAP, 501 |
| `POST /api/v1/vendor-portal/jobs/{job_id}/propose-time` | `vendor_portal.rs:propose_alternative_time` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/vendor-portal/jobs/{job_id}/access` | `vendor_portal.rs:get_access_info` | stub | — | unmounted ROADMAP, 501 |
| `POST /api/v1/vendor-portal/jobs/{job_id}/access/generate-code` | `vendor_portal.rs:generate_access_code` | stub | — | unmounted ROADMAP, 501 |
| `POST /api/v1/vendor-portal/jobs/{job_id}/complete` | `vendor_portal.rs:submit_work_completion` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/vendor-portal/jobs/{job_id}/completion` | `vendor_portal.rs:get_work_completion` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/vendor-portal/invoices` | `vendor_portal.rs:list_invoices` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/vendor-portal/profile` | `vendor_portal.rs:get_profile` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/vendor-portal/feedback` | `vendor_portal.rs:list_feedback` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/vendor-portal/earnings` | `vendor_portal.rs:get_earnings_summary` | stub | — | unmounted ROADMAP, 501 |
| `GET /api/v1/property-valuations/dashboard` | `property_valuation.rs:get_dashboard` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/expiring` | `property_valuation.rs:get_expiring_valuations` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/models` | `property_valuation.rs:list_models` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations/models` | `property_valuation.rs:create_model` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/models/{model_id}` | `property_valuation.rs:get_model` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/models/{model_id}` | `property_valuation.rs:update_model` | partial | — | real handler, no test |
| `DELETE /api/v1/property-valuations/models/{model_id}` | `property_valuation.rs:delete_model` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations` | `property_valuation.rs:list_valuations` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations` | `property_valuation.rs:create_valuation` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/{valuation_id}` | `property_valuation.rs:get_valuation` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/{valuation_id}` | `property_valuation.rs:update_valuation` | partial | — | real handler, no test |
| `DELETE /api/v1/property-valuations/{valuation_id}` | `property_valuation.rs:delete_valuation` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/{valuation_id}/approve` | `property_valuation.rs:approve_valuation` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/{valuation_id}/comparables` | `property_valuation.rs:list_comparables` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations/{valuation_id}/comparables` | `property_valuation.rs:create_comparable` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/comparables/{comparable_id}` | `property_valuation.rs:update_comparable` | partial | — | real handler, no test |
| `DELETE /api/v1/property-valuations/comparables/{comparable_id}` | `property_valuation.rs:delete_comparable` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/comparables/{comparable_id}/adjustments` | `property_valuation.rs:list_adjustments` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations/comparables/{comparable_id}/adjustments` | `property_valuation.rs:create_adjustment` | partial | — | real handler, no test |
| `DELETE /api/v1/property-valuations/adjustments/{adjustment_id}` | `property_valuation.rs:delete_adjustment` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/market-data` | `property_valuation.rs:get_market_data` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations/market-data` | `property_valuation.rs:create_market_data` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/market-data/{market_data_id}` | `property_valuation.rs:update_market_data` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/properties/{property_id}/history` | `property_valuation.rs:get_value_history` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations/properties/{property_id}/history` | `property_valuation.rs:create_value_history` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/requests` | `property_valuation.rs:list_requests` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations/requests` | `property_valuation.rs:create_request` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/requests/{request_id}` | `property_valuation.rs:get_request` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/requests/{request_id}` | `property_valuation.rs:update_request` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/properties/{property_id}/features` | `property_valuation.rs:get_features` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations/properties/{property_id}/features` | `property_valuation.rs:create_features` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/features/{feature_id}` | `property_valuation.rs:update_features` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/{valuation_id}/reports` | `property_valuation.rs:list_reports` | partial | — | real handler, no test |
| `POST /api/v1/property-valuations/{valuation_id}/reports` | `property_valuation.rs:create_report` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/reports/{report_id}` | `property_valuation.rs:update_report` | partial | — | real handler, no test |
| `PUT /api/v1/property-valuations/reports/{report_id}/sign` | `property_valuation.rs:sign_report` | partial | — | real handler, no test |
| `GET /api/v1/property-valuations/{valuation_id}/audit-logs` | `property_valuation.rs:get_audit_logs` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/models` | `enhanced_tenant_screening.rs:list_risk_models` | partial | — | only repo-level IDOR test |
| `POST /api/v1/tenant-screening/models` | `enhanced_tenant_screening.rs:create_risk_model` | partial | — | only repo-level IDOR test |
| `GET /api/v1/tenant-screening/models/{id}` | `enhanced_tenant_screening.rs:get_risk_model` | partial | — | only repo-level IDOR test |
| `DELETE /api/v1/tenant-screening/models/{id}` | `enhanced_tenant_screening.rs:delete_risk_model` | partial | — | real handler, no test |
| `POST /api/v1/tenant-screening/models/{id}/activate` | `enhanced_tenant_screening.rs:activate_risk_model` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/providers` | `enhanced_tenant_screening.rs:list_provider_configs` | partial | — | real handler, no test |
| `POST /api/v1/tenant-screening/providers` | `enhanced_tenant_screening.rs:create_provider_config` | partial | — | only repo-level IDOR test |
| `GET /api/v1/tenant-screening/providers/{id}` | `enhanced_tenant_screening.rs:get_provider_config` | partial | — | only repo-level IDOR test |
| `DELETE /api/v1/tenant-screening/providers/{id}` | `enhanced_tenant_screening.rs:delete_provider_config` | partial | — | real handler, no test |
| `PUT /api/v1/tenant-screening/providers/{id}/status` | `enhanced_tenant_screening.rs:update_provider_status` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/results` | `enhanced_tenant_screening.rs:list_ai_results` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/results/{screening_id}` | `enhanced_tenant_screening.rs:get_ai_result` | partial | — | only repo-level IDOR test |
| `GET /api/v1/tenant-screening/results/{screening_id}/factors` | `enhanced_tenant_screening.rs:get_risk_factors` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/results/{screening_id}/complete` | `enhanced_tenant_screening.rs:get_complete_screening_data` | partial | — | only repo-level IDOR test |
| `POST /api/v1/tenant-screening/score` | `enhanced_tenant_screening.rs:run_ai_scoring` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/credit/{screening_id}` | `enhanced_tenant_screening.rs:get_credit_result` | partial | — | real handler, no test |
| `POST /api/v1/tenant-screening/credit` | `enhanced_tenant_screening.rs:create_credit_result` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/background/{screening_id}` | `enhanced_tenant_screening.rs:get_background_result` | partial | — | real handler, no test |
| `POST /api/v1/tenant-screening/background` | `enhanced_tenant_screening.rs:create_background_result` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/eviction/{screening_id}` | `enhanced_tenant_screening.rs:get_eviction_result` | partial | — | real handler, no test |
| `POST /api/v1/tenant-screening/eviction` | `enhanced_tenant_screening.rs:create_eviction_result` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/queue` | `enhanced_tenant_screening.rs:get_pending_queue` | partial | — | real handler, no test |
| `POST /api/v1/tenant-screening/queue` | `enhanced_tenant_screening.rs:create_queue_item` | partial | — | real handler, no test |
| `PUT /api/v1/tenant-screening/queue/{id}/status` | `enhanced_tenant_screening.rs:update_queue_status` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/reports/{screening_id}` | `enhanced_tenant_screening.rs:get_reports` | partial | — | real handler, no test |
| `POST /api/v1/tenant-screening/reports` | `enhanced_tenant_screening.rs:create_report` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/statistics` | `enhanced_tenant_screening.rs:get_statistics` | partial | — | real handler, no test |
| `GET /api/v1/tenant-screening/distribution` | `enhanced_tenant_screening.rs:get_risk_distribution` | partial | — | real handler, no test |
| `POST /api/v1/buildings` | `buildings.rs:create_building` | done | `building_manager_rbac_tests.rs` | RBAC tested |
| `GET /api/v1/buildings` | `buildings.rs:list_buildings` | done | `building_manager_rbac_tests.rs` | RBAC tested |
| `POST /api/v1/buildings/bulk` | `buildings.rs:bulk_import_buildings` | done | `building_manager_rbac_tests.rs` | RBAC tested |
| `GET /api/v1/buildings/{id}` | `buildings.rs:get_building` | partial | — | real handler, no test |
| `PUT /api/v1/buildings/{id}` | `buildings.rs:update_building` | partial | — | real handler, no test |
| `DELETE /api/v1/buildings/{id}` | `buildings.rs:archive_building` | done | `building_manager_rbac_tests.rs` | RBAC tested |
| `POST /api/v1/buildings/{id}/restore` | `buildings.rs:restore_building` | partial | — | real handler, no test |
| `GET /api/v1/buildings/{id}/statistics` | `buildings.rs:get_building_statistics` | partial | — | real handler, no test |
| `GET /api/v1/buildings/{id}/units` | `buildings.rs:list_units` | partial | — | real handler, no test |
| `POST /api/v1/buildings/{id}/units` | `buildings.rs:create_unit` | partial | — | real handler, no test |
| `GET /api/v1/buildings/{building_id}/units/{unit_id}` | `buildings.rs:get_unit` | partial | — | real handler, no test |
| `PUT /api/v1/buildings/{building_id}/units/{unit_id}` | `buildings.rs:update_unit` | partial | — | real handler, no test |
| `DELETE /api/v1/buildings/{building_id}/units/{unit_id}` | `buildings.rs:archive_unit` | partial | — | real handler, no test |
| `POST /api/v1/buildings/{building_id}/units/{unit_id}/restore` | `buildings.rs:restore_unit` | partial | — | real handler, no test |
| `GET /api/v1/buildings/{building_id}/units/{unit_id}/owners` | `buildings.rs:list_unit_owners` | partial | — | real handler, no test |
| `POST /api/v1/buildings/{building_id}/units/{unit_id}/owners` | `buildings.rs:assign_unit_owner` | partial | — | real handler, no test |
| `PUT /api/v1/buildings/{building_id}/units/{unit_id}/owners/{user_id}` | `buildings.rs:update_unit_owner` | partial | — | real handler, no test |
| `DELETE /api/v1/buildings/{building_id}/units/{unit_id}/owners/{user_id}` | `buildings.rs:remove_unit_owner` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/dashboard` | `building_certifications.rs:get_dashboard` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications` | `building_certifications.rs:list_certifications` | partial | — | real handler, no test |
| `POST /api/v1/building-certifications` | `building_certifications.rs:create_certification` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/expiring` | `building_certifications.rs:get_expiring_certifications` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}` | `building_certifications.rs:get_certification` | partial | — | real handler, no test |
| `PUT /api/v1/building-certifications/{cert_id}` | `building_certifications.rs:update_certification` | partial | — | real handler, no test |
| `DELETE /api/v1/building-certifications/{cert_id}` | `building_certifications.rs:delete_certification` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/with-credits` | `building_certifications.rs:get_certification_with_credits` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/credits` | `building_certifications.rs:list_credits` | partial | — | real handler, no test |
| `POST /api/v1/building-certifications/{cert_id}/credits` | `building_certifications.rs:create_credit` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/credits/{credit_id}` | `building_certifications.rs:get_credit` | partial | — | real handler, no test |
| `PUT /api/v1/building-certifications/{cert_id}/credits/{credit_id}` | `building_certifications.rs:update_credit` | partial | — | real handler, no test |
| `DELETE /api/v1/building-certifications/{cert_id}/credits/{credit_id}` | `building_certifications.rs:delete_credit` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/documents` | `building_certifications.rs:list_documents` | partial | — | real handler, no test |
| `POST /api/v1/building-certifications/{cert_id}/documents` | `building_certifications.rs:create_document` | partial | — | real handler, no test |
| `DELETE /api/v1/building-certifications/{cert_id}/documents/{doc_id}` | `building_certifications.rs:delete_document` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/milestones` | `building_certifications.rs:list_milestones` | partial | — | real handler, no test |
| `POST /api/v1/building-certifications/{cert_id}/milestones` | `building_certifications.rs:create_milestone` | partial | — | real handler, no test |
| `PUT /api/v1/building-certifications/{cert_id}/milestones/{milestone_id}` | `building_certifications.rs:update_milestone` | partial | — | real handler, no test |
| `DELETE /api/v1/building-certifications/{cert_id}/milestones/{milestone_id}` | `building_certifications.rs:delete_milestone` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/benchmarks` | `building_certifications.rs:list_benchmarks` | partial | — | real handler, no test |
| `POST /api/v1/building-certifications/{cert_id}/benchmarks` | `building_certifications.rs:create_benchmark` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/costs` | `building_certifications.rs:list_costs` | partial | — | real handler, no test |
| `POST /api/v1/building-certifications/{cert_id}/costs` | `building_certifications.rs:create_cost` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/costs/total` | `building_certifications.rs:get_total_costs` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/reminders` | `building_certifications.rs:list_reminders` | partial | — | real handler, no test |
| `POST /api/v1/building-certifications/{cert_id}/reminders` | `building_certifications.rs:create_reminder` | partial | — | real handler, no test |
| `GET /api/v1/building-certifications/{cert_id}/audit-logs` | `building_certifications.rs:list_audit_logs` | partial | — | real handler, no test |
| `GET /api/v1/buildings/{building_id}/units/{unit_id}/residents` | `unit_residents.rs:list_residents` | partial | — | nested under buildings, no test |
| `POST /api/v1/buildings/{building_id}/units/{unit_id}/residents` | `unit_residents.rs:add_resident` | partial | — | nested under buildings, no test |
| `GET /api/v1/buildings/{building_id}/units/{unit_id}/residents/{resident_id}` | `unit_residents.rs:get_resident` | partial | — | nested under buildings, no test |
| `PUT /api/v1/buildings/{building_id}/units/{unit_id}/residents/{resident_id}` | `unit_residents.rs:update_resident` | partial | — | nested under buildings, no test |
| `DELETE /api/v1/buildings/{building_id}/units/{unit_id}/residents/{resident_id}` | `unit_residents.rs:remove_resident` | partial | — | nested under buildings, no test |
| `POST /api/v1/buildings/{building_id}/units/{unit_id}/residents/{resident_id}/end` | `unit_residents.rs:end_residency` | partial | — | nested under buildings, no test |
| `GET /api/v1/buildings/{building_id}/units/{unit_id}/residents/history` | `unit_residents.rs:list_resident_history` | partial | — | nested under buildings, no test |
| `GET /api/v1/onboarding/tours` | `onboarding.rs:get_user_tours` | partial | — | real handler, no test |
| `GET /api/v1/onboarding/tours/{tour_id}` | `onboarding.rs:get_tour` | partial | — | real handler, no test |
| `POST /api/v1/onboarding/tours/{tour_id}/start` | `onboarding.rs:start_tour` | partial | — | real handler, no test |
| `POST /api/v1/onboarding/tours/{tour_id}/steps/{step_id}/complete` | `onboarding.rs:complete_step` | partial | — | real handler, no test |
| `POST /api/v1/onboarding/tours/{tour_id}/complete` | `onboarding.rs:complete_tour` | partial | — | real handler, no test |
| `POST /api/v1/onboarding/tours/{tour_id}/skip` | `onboarding.rs:skip_tour` | partial | — | real handler, no test |
| `POST /api/v1/onboarding/tours/{tour_id}/reset` | `onboarding.rs:reset_tour` | partial | — | real handler, no test |
| `GET /api/v1/onboarding/status` | `onboarding.rs:get_onboarding_status` | partial | — | real handler, no test |
| `POST /api/v1/packages` | `package_visitor.rs:create_package` | partial | — | real handler, no test |
| `GET /api/v1/packages` | `package_visitor.rs:list_packages` | partial | — | real handler, no test |
| `GET /api/v1/packages/{id}` | `package_visitor.rs:get_package` | partial | — | real handler, no test |
| `PUT /api/v1/packages/{id}` | `package_visitor.rs:update_package` | partial | — | real handler, no test |
| `DELETE /api/v1/packages/{id}` | `package_visitor.rs:delete_package` | partial | — | real handler, no test |
| `POST /api/v1/packages/{id}/receive` | `package_visitor.rs:receive_package` | partial | — | real handler, no test |
| `POST /api/v1/packages/{id}/pickup` | `package_visitor.rs:pickup_package` | partial | — | real handler, no test |
| `GET /api/v1/packages/buildings/{building_id}/settings` | `package_visitor.rs:get_package_settings` | partial | — | real handler, no test |
| `PUT /api/v1/packages/buildings/{building_id}/settings` | `package_visitor.rs:update_package_settings` | partial | — | real handler, no test |
| `GET /api/v1/packages/buildings/{building_id}/statistics` | `package_visitor.rs:get_package_statistics` | partial | — | real handler, no test |
| `POST /api/v1/visitors` | `package_visitor.rs:create_visitor` | partial | — | real handler, no test |
| `GET /api/v1/visitors` | `package_visitor.rs:list_visitors` | partial | — | real handler, no test |
| `POST /api/v1/visitors/verify-code` | `package_visitor.rs:verify_access_code` | partial | — | real handler, no test |
| `GET /api/v1/visitors/{id}` | `package_visitor.rs:get_visitor` | partial | — | real handler, no test |
| `PUT /api/v1/visitors/{id}` | `package_visitor.rs:update_visitor` | partial | — | real handler, no test |
| `DELETE /api/v1/visitors/{id}` | `package_visitor.rs:delete_visitor` | partial | — | real handler, no test |
| `POST /api/v1/visitors/{id}/check-in` | `package_visitor.rs:check_in_visitor` | partial | — | real handler, no test |
| `POST /api/v1/visitors/{id}/check-out` | `package_visitor.rs:check_out_visitor` | partial | — | real handler, no test |
| `POST /api/v1/visitors/{id}/cancel` | `package_visitor.rs:cancel_visitor` | partial | — | real handler, no test |
| `GET /api/v1/visitors/buildings/{building_id}/settings` | `package_visitor.rs:get_visitor_settings` | partial | — | real handler, no test |
| `PUT /api/v1/visitors/buildings/{building_id}/settings` | `package_visitor.rs:update_visitor_settings` | partial | — | real handler, no test |
| `GET /api/v1/visitors/buildings/{building_id}/statistics` | `package_visitor.rs:get_visitor_statistics` | partial | — | real handler, no test |

## Tally
done: 19  partial: 285  stub: 14  missing: 0  total: 318
