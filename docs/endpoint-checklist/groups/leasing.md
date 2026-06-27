# Leasing

_Server: api-server. Modules: leases.rs, rentals.rs, listings.rs._

Classification basis: handlers in all three modules are real (query the repo/db layer and return typed data); no `todo!()`/`unimplemented!()`/mock bodies exist. The single exception is `extract_guest_id_document`, whose success path is unreachable because the wired OCR provider is a stub that returns `501 OCR_NOT_CONFIGURED` (the only test asserts 501). Test coverage is thin: only 6 test files touch these paths, almost all authz/IDOR/RBAC (assert 401/403/404 only). Per spec, authz-only tests do NOT prove the success path → those endpoints are `partial`. Endpoints with no test hitting their path are `partial`.

## leases.rs  (mount: /api/v1/leases)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/leases/applications | create_application | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/applications | list_applications | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/applications/{id} | get_application | partial | — | real handler, no test hits path |
| PUT | /api/v1/leases/applications/{id} | update_application | partial | — | real handler, no test hits path |
| POST | /api/v1/leases/applications/{id}/submit | submit_application | partial | — | real handler, no test hits path |
| POST | /api/v1/leases/applications/{id}/review | review_application | partial | lease_review_rbac_tests.rs | RBAC only (403 asserts), no happy path |
| POST | /api/v1/leases/applications/{id}/screening | initiate_screening | partial | enhanced_screening_cross_org_idor_tests.rs | cross-org IDOR only, no happy path |
| POST | /api/v1/leases/applications/{id}/screening/consent | record_consent | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/screenings/{id} | get_screening | partial | — | real handler, no test hits path |
| PATCH | /api/v1/leases/screenings/{id}/result | update_screening_result | partial | — | real handler, no test hits path |
| POST | /api/v1/leases/templates | create_template | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/templates | list_templates | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/templates/{id} | get_template | partial | — | real handler, no test hits path |
| PUT | /api/v1/leases/templates/{id} | update_template | partial | — | real handler, no test hits path |
| POST | /api/v1/leases | create_lease | partial | — | real handler, no test hits path |
| GET | /api/v1/leases | list_leases | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/{id} | get_lease | partial | — | real handler, no test hits path |
| PUT | /api/v1/leases/{id} | update_lease | partial | — | real handler, no test hits path |
| POST | /api/v1/leases/{id}/terminate | terminate_lease | partial | — | real handler, no test hits path |
| POST | /api/v1/leases/{id}/renew | renew_lease | partial | — | real handler, no test hits path |
| POST | /api/v1/leases/{id}/send-for-signature | send_lease_for_signature | partial | lease_send_for_signature_rbac_tests.rs | RBAC only (403 asserts), no happy path |
| POST | /api/v1/leases/{id}/amendments | create_amendment | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/{id}/amendments | list_amendments | partial | — | real handler, no test hits path |
| POST | /api/v1/leases/{id}/payments | record_payment | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/{id}/payments | list_payments | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/{id}/payments/summary | get_payment_summary | partial | — | real handler, no test hits path |
| POST | /api/v1/leases/{id}/reminders | create_reminder | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/{id}/reminders | list_reminders | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/expiring | get_expiring_leases | partial | — | real handler, no test hits path |
| GET | /api/v1/leases/statistics | get_statistics | partial | — | real handler, no test hits path |

## rentals.rs  (mount: /api/v1/rentals)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/rentals/statistics | get_statistics | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/sync-status | get_sync_status | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/connections | list_connections | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/connections | create_connection | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/connections/{id} | get_connection | done | rental_connection_token_leak_tests.rs | happy-path 200 (asserts token not leaked) |
| PUT | /api/v1/rentals/connections/{id} | update_connection | partial | — | real handler, no test hits path |
| DELETE | /api/v1/rentals/connections/{id} | delete_connection | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/units/{unit_id}/connections | get_unit_connections | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/bookings | list_bookings | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/bookings | create_booking | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/bookings/{id} | get_booking | partial | — | real handler, no test hits path |
| PUT | /api/v1/rentals/bookings/{id} | update_booking | partial | — | real handler, no test hits path |
| PUT | /api/v1/rentals/bookings/{id}/status | update_booking_status | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/bookings/{id}/guests | get_booking_with_guests | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/calendar/{unit_id} | get_calendar | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/calendar/{unit_id}/availability | check_availability | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/calendar/blocks | create_calendar_block | partial | — | real handler, no test hits path |
| DELETE | /api/v1/rentals/calendar/blocks/{id} | delete_calendar_block | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/guests | create_guest | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/guests/{id} | get_guest | partial | — | real handler, no test hits path |
| PUT | /api/v1/rentals/guests/{id} | update_guest | partial | — | real handler, no test hits path |
| DELETE | /api/v1/rentals/guests/{id} | delete_guest | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/guests/{id}/register | register_guest | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/guests/{id}/id-document | upload_guest_id_document | done | rental_guest_id_document_tests.rs | happy-path 201, verifies url + DB row |
| POST | /api/v1/rentals/guests/{id}/id-document/extract | extract_guest_id_document | stub | rental_guest_id_document_tests.rs | OCR provider is a stub → returns 501 OCR_NOT_CONFIGURED; only test asserts 501 |
| GET | /api/v1/rentals/checkin-reminders | get_checkin_reminders | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/reports | list_reports | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/reports/preview | generate_report_preview | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/reports | create_report | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/reports/{id} | get_report | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/reports/{id}/submit | submit_report | partial | — | real handler, no test hits path |
| POST | /api/v1/rentals/ical | create_ical_feed | partial | — | real handler, no test hits path |
| PUT | /api/v1/rentals/ical/{id} | update_ical_feed | partial | — | real handler, no test hits path |
| DELETE | /api/v1/rentals/ical/{id} | delete_ical_feed | partial | — | real handler, no test hits path |
| GET | /api/v1/rentals/units/{unit_id}/ical | get_unit_ical_feeds | partial | — | real handler, no test hits path |

## listings.rs  (mount: /api/v1/listings)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/listings | create_listing | partial | — | real handler, no test hits path |
| GET | /api/v1/listings | list_listings | partial | — | real handler, no test hits path |
| POST | /api/v1/listings/from-unit | create_from_unit | partial | — | real handler, no test hits path |
| GET | /api/v1/listings/statistics | get_statistics | partial | — | real handler, no test hits path |
| GET | /api/v1/listings/syndication/dashboard | get_syndication_dashboard | partial | — | real handler, no test hits path |
| GET | /api/v1/listings/syndication/stats | get_organization_syndication_stats | partial | — | real handler, no test hits path |
| GET | /api/v1/listings/{id} | get_listing | partial | — | real handler, no test hits path |
| PUT | /api/v1/listings/{id} | update_listing | partial | — | real handler, no test hits path |
| DELETE | /api/v1/listings/{id} | delete_listing | partial | — | real handler, no test hits path |
| PUT | /api/v1/listings/{id}/status | update_status | partial | — | real handler, no test hits path |
| POST | /api/v1/listings/{id}/publish | publish_listing | partial | — | real handler, no test hits path |
| POST | /api/v1/listings/{id}/global-publish | global_publish | done | listings_global_publish_authz_tests.rs | happy-path 200 + verifies is_published |
| POST | /api/v1/listings/{id}/global-unpublish | global_unpublish | partial | listings_global_publish_authz_tests.rs | authz only (403 assert), no happy path |
| GET | /api/v1/listings/{id}/photos | get_photos | partial | — | real handler, no test hits path |
| POST | /api/v1/listings/{id}/photos | add_photo | partial | — | real handler, no test hits path |
| POST | /api/v1/listings/{id}/photos/reorder | reorder_photos | partial | — | real handler, no test hits path |
| DELETE | /api/v1/listings/{id}/photos/{photo_id} | delete_photo | partial | — | real handler, no test hits path |
| GET | /api/v1/listings/{id}/syndications | get_syndications | partial | — | real handler, no test hits path |
| GET | /api/v1/listings/{id}/syndication/status | get_listing_syndication_status | partial | — | real handler, no test hits path |

## Summary
- done: 3 | partial: 80 | stub: 1 | missing: 0 | total: 84
