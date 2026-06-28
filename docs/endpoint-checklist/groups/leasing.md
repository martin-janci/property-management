# Leasing

_Server: api-server. Modules: leases.rs, rentals.rs, listings.rs._

Classification basis: handlers in all three modules are real (query the repo/db layer and return typed data); no `todo!()`/`unimplemented!()`/mock bodies exist. The single exception is `extract_guest_id_document`, whose success path is unreachable because the wired OCR provider is a stub that returns `501 OCR_NOT_CONFIGURED` (the only test asserts 501). Test coverage is thin: only 6 test files touch these paths, almost all authz/IDOR/RBAC (assert 401/403/404 only). Per spec, authz-only tests do NOT prove the success path → those endpoints are `partial`. Endpoints with no test hitting their path are `partial`.

## leases.rs  (mount: /api/v1/leases)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/leases/applications | create_application | done | leases_core_backfill_batch1_tests.rs | happy-path 201 |
| GET | /api/v1/leases/applications | list_applications | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| GET | /api/v1/leases/applications/{id} | get_application | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| PUT | /api/v1/leases/applications/{id} | update_application | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| POST | /api/v1/leases/applications/{id}/submit | submit_application | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| POST | /api/v1/leases/applications/{id}/review | review_application | done | leases_core_backfill_batch1_tests.rs | happy-path 200 (was RBAC-only) |
| POST | /api/v1/leases/applications/{id}/screening | initiate_screening | partial | enhanced_screening_cross_org_idor_tests.rs | cross-org IDOR only, no happy path; needs external screening provider |
| POST | /api/v1/leases/applications/{id}/screening/consent | record_consent | partial | — | requires existing screening row |
| GET | /api/v1/leases/screenings/{id} | get_screening | partial | — | requires existing screening row |
| PATCH | /api/v1/leases/screenings/{id}/result | update_screening_result | partial | — | requires existing screening row |
| POST | /api/v1/leases/templates | create_template | done | leases_core_backfill_batch1_tests.rs | happy-path 201 |
| GET | /api/v1/leases/templates | list_templates | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| GET | /api/v1/leases/templates/{id} | get_template | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| PUT | /api/v1/leases/templates/{id} | update_template | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| POST | /api/v1/leases | create_lease | done | leases_core_backfill_batch1_tests.rs | happy-path 201 |
| GET | /api/v1/leases | list_leases | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| GET | /api/v1/leases/{id} | get_lease | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| PUT | /api/v1/leases/{id} | update_lease | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| POST | /api/v1/leases/{id}/terminate | terminate_lease | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| POST | /api/v1/leases/{id}/renew | renew_lease | done | leases_core_backfill_batch1_tests.rs | happy-path 201 |
| POST | /api/v1/leases/{id}/send-for-signature | send_lease_for_signature | partial | lease_send_for_signature_rbac_tests.rs | RBAC only (403 asserts); needs seeded document_id |
| POST | /api/v1/leases/{id}/amendments | create_amendment | done | leases_core_backfill_batch1_tests.rs | happy-path 201 |
| GET | /api/v1/leases/{id}/amendments | list_amendments | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| POST | /api/v1/leases/{id}/payments | record_payment | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| GET | /api/v1/leases/{id}/payments | list_payments | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| GET | /api/v1/leases/{id}/payments/summary | get_payment_summary | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| POST | /api/v1/leases/{id}/reminders | create_reminder | done | leases_core_backfill_batch1_tests.rs | happy-path 201 |
| GET | /api/v1/leases/{id}/reminders | list_reminders | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| GET | /api/v1/leases/expiring | get_expiring_leases | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |
| GET | /api/v1/leases/statistics | get_statistics | done | leases_core_backfill_batch1_tests.rs | happy-path 200 |

## rentals.rs  (mount: /api/v1/rentals)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/rentals/statistics | get_statistics | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| GET | /api/v1/rentals/sync-status | get_sync_status | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| GET | /api/v1/rentals/connections | list_connections | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| POST | /api/v1/rentals/connections | create_connection | done | rentals_core_backfill_batch2_tests.rs | happy-path 201 |
| GET | /api/v1/rentals/connections/{id} | get_connection | done | rental_connection_token_leak_tests.rs | happy-path 200 (asserts token not leaked) |
| PUT | /api/v1/rentals/connections/{id} | update_connection | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| DELETE | /api/v1/rentals/connections/{id} | delete_connection | done | rentals_core_backfill_batch2_tests.rs | happy-path 204 |
| GET | /api/v1/rentals/units/{unit_id}/connections | get_unit_connections | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| GET | /api/v1/rentals/bookings | list_bookings | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| POST | /api/v1/rentals/bookings | create_booking | done | rentals_core_backfill_batch2_tests.rs | happy-path 201 |
| GET | /api/v1/rentals/bookings/{id} | get_booking | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| PUT | /api/v1/rentals/bookings/{id} | update_booking | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| PUT | /api/v1/rentals/bookings/{id}/status | update_booking_status | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| GET | /api/v1/rentals/bookings/{id}/guests | get_booking_with_guests | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| GET | /api/v1/rentals/calendar/{unit_id} | get_calendar | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| GET | /api/v1/rentals/calendar/{unit_id}/availability | check_availability | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| POST | /api/v1/rentals/calendar/blocks | create_calendar_block | done | rentals_core_backfill_batch2_tests.rs | happy-path 201 |
| DELETE | /api/v1/rentals/calendar/blocks/{id} | delete_calendar_block | done | rentals_core_backfill_batch2_tests.rs | happy-path 204 |
| POST | /api/v1/rentals/guests | create_guest | done | rentals_core_backfill_batch2_tests.rs | happy-path 201 |
| GET | /api/v1/rentals/guests/{id} | get_guest | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| PUT | /api/v1/rentals/guests/{id} | update_guest | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| DELETE | /api/v1/rentals/guests/{id} | delete_guest | done | rentals_core_backfill_batch2_tests.rs | happy-path 204 |
| POST | /api/v1/rentals/guests/{id}/register | register_guest | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| POST | /api/v1/rentals/guests/{id}/id-document | upload_guest_id_document | done | rental_guest_id_document_tests.rs | happy-path 201, verifies url + DB row |
| POST | /api/v1/rentals/guests/{id}/id-document/extract | extract_guest_id_document | stub | rental_guest_id_document_tests.rs | OCR provider is a stub → returns 501 OCR_NOT_CONFIGURED; only test asserts 501 |
| GET | /api/v1/rentals/checkin-reminders | get_checkin_reminders | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| GET | /api/v1/rentals/reports | list_reports | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| POST | /api/v1/rentals/reports/preview | generate_report_preview | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| POST | /api/v1/rentals/reports | create_report | done | rentals_core_backfill_batch2_tests.rs | happy-path 201 |
| GET | /api/v1/rentals/reports/{id} | get_report | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| POST | /api/v1/rentals/reports/{id}/submit | submit_report | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| POST | /api/v1/rentals/ical | create_ical_feed | done | rentals_core_backfill_batch2_tests.rs | happy-path 201 |
| PUT | /api/v1/rentals/ical/{id} | update_ical_feed | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |
| DELETE | /api/v1/rentals/ical/{id} | delete_ical_feed | done | rentals_core_backfill_batch2_tests.rs | happy-path 204 |
| GET | /api/v1/rentals/units/{unit_id}/ical | get_unit_ical_feeds | done | rentals_core_backfill_batch2_tests.rs | happy-path 200 |

## listings.rs  (mount: /api/v1/listings)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/listings | create_listing | done | listings_core_backfill_batch3_tests.rs | happy-path 201 |
| GET | /api/v1/listings | list_listings | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| POST | /api/v1/listings/from-unit | create_from_unit | done | listings_core_backfill_batch3_tests.rs | happy-path 201 |
| GET | /api/v1/listings/statistics | get_statistics | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| GET | /api/v1/listings/syndication/dashboard | get_syndication_dashboard | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| GET | /api/v1/listings/syndication/stats | get_organization_syndication_stats | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| GET | /api/v1/listings/{id} | get_listing | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| PUT | /api/v1/listings/{id} | update_listing | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| DELETE | /api/v1/listings/{id} | delete_listing | done | listings_core_backfill_batch3_tests.rs | happy-path 204 |
| PUT | /api/v1/listings/{id}/status | update_status | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| POST | /api/v1/listings/{id}/publish | publish_listing | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| POST | /api/v1/listings/{id}/global-publish | global_publish | done | listings_global_publish_authz_tests.rs | happy-path 200 + verifies is_published |
| POST | /api/v1/listings/{id}/global-unpublish | global_unpublish | done | listings_core_backfill_batch3_tests.rs | happy-path 200 (was authz-only) |
| GET | /api/v1/listings/{id}/photos | get_photos | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| POST | /api/v1/listings/{id}/photos | add_photo | done | listings_core_backfill_batch3_tests.rs | happy-path 201 |
| POST | /api/v1/listings/{id}/photos/reorder | reorder_photos | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| DELETE | /api/v1/listings/{id}/photos/{photo_id} | delete_photo | done | listings_core_backfill_batch3_tests.rs | happy-path 204 |
| GET | /api/v1/listings/{id}/syndications | get_syndications | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |
| GET | /api/v1/listings/{id}/syndication/status | get_listing_syndication_status | done | listings_core_backfill_batch3_tests.rs | happy-path 200 |

## Summary
- done: 78 | partial: 5 | stub: 1 | missing: 0 | total: 84
