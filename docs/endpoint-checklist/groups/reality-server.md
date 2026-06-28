# Reality Server — Portal Endpoints

_Server: reality-server. Modules: agencies, agency_branding, agency_imports, agent_reviews, articles, compare, favorites, health, imports, inquiries, listings, portal_listings, price_map, realtors, reports, saved_searches, sso, users._

> Test reality: only `imports_idor_tests.rs` and `portal_listings_idor_tests.rs` exercise real HTTP routes (build the module router + `oneshot`). The inquiry tests (`buyer_inquiries_tests.rs`, `inquiry_idor_tests.rs`, `inquiry_pagination_tests.rs`) hit `RealityPortalRepository` **directly**, NOT the endpoint, so they do NOT count as path coverage. `favorite_alert_worker_tests.rs` and `search_alert_drainer_tests.rs` test background workers, not endpoints. `raw_pool_audit_tests.rs` uses a fake stub handler. Per spec, repo-level/worker tests → endpoints stay `partial`.

## health.rs  (mount: / via main.rs `.route`)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /health | liveness | done | health_tests.rs | liveness_returns_200 |
| GET | /readiness | readiness | done | health_tests.rs | readiness_returns_200_or_degraded |

## listings.rs  (mount: /api/v1/listings)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/listings | search | done | listings_tests.rs | search_returns_200_empty_result, search_with_query_param_returns_200 |
| GET | /api/v1/listings/featured | get_featured | done | listings_tests.rs | get_featured_returns_200 |
| GET | /api/v1/listings/categories | get_categories | done | listings_tests.rs | get_categories_returns_200 |
| GET | /api/v1/listings/suggestions | get_suggestions | done | listings_tests.rs | get_suggestions_returns_200 |
| GET | /api/v1/listings/{id} | get_listing | done | listings_tests.rs | get_listing_unknown_id_returns_404 |
| POST | /api/v1/listings/{id}/view | record_view | done | listings_tests.rs | record_view_unknown_listing_returns_404 |

## users.rs  (mount: /api/v1/users)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/users/register | register | done | users_authz_tests.rs | Real; public endpoint — non-401 verified; OpenAPI drift |
| POST | /api/v1/users/login | login | done | users_authz_tests.rs | Real; public endpoint — non-401 verified; OpenAPI drift |
| POST | /api/v1/users/password-reset | request_password_reset | done | users_authz_tests.rs | Real; public endpoint — non-401 verified; OpenAPI drift |
| POST | /api/v1/users/password-reset/confirm | confirm_password_reset | done | users_authz_tests.rs | Real; public endpoint — non-401 verified; OpenAPI drift |
| POST | /api/v1/users/logout | logout | done | users_authz_tests.rs | Real; auth boundary + happy path |
| GET | /api/v1/users/me | get_me | done | users_authz_tests.rs | Real; auth boundary + happy path |
| PUT | /api/v1/users/me | update_me | done | users_authz_tests.rs | Real; auth boundary + happy path |

## favorites.rs  (mount: /api/v1/favorites)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/favorites | list_favorites | done | favorites_authz_tests.rs | list_favorites_unauthenticated_returns_401, list_favorites_authenticated_returns_non_401 |
| GET | /api/v1/favorites/alerts | list_favorite_alerts | done | favorites_authz_tests.rs | list_favorite_alerts_unauthenticated_returns_401, list_favorite_alerts_authenticated_returns_non_401 (OpenAPI drift) |
| POST | /api/v1/favorites/alerts/read-all | mark_all_favorite_alerts_read | done | favorites_authz_tests.rs | mark_all_favorite_alerts_read_unauthenticated_returns_401, mark_all_favorite_alerts_read_authenticated_returns_non_401 |
| POST | /api/v1/favorites/alerts/{alert_id}/read | mark_favorite_alert_read | done | favorites_authz_tests.rs | mark_favorite_alert_read_unauthenticated_returns_401, mark_favorite_alert_read_authenticated_unknown_returns_non_401 |
| GET | /api/v1/favorites/ids | list_favorite_ids | done | favorites_authz_tests.rs | list_favorite_ids_unauthenticated_returns_200, list_favorite_ids_authenticated_returns_200 (SSR-anonymous, OptionalRequestPrincipal) |
| POST | /api/v1/favorites/{listing_id} | add_favorite | done | favorites_authz_tests.rs | add_favorite_unauthenticated_returns_401, add_favorite_authenticated_unknown_listing_returns_non_401 |
| DELETE | /api/v1/favorites/{listing_id} | remove_favorite | done | favorites_authz_tests.rs | remove_favorite_unauthenticated_returns_401, remove_favorite_authenticated_unknown_returns_non_401 |
| GET | /api/v1/favorites/{listing_id}/check | check_favorite | done | favorites_authz_tests.rs | check_favorite_unauthenticated_returns_401, check_favorite_authenticated_returns_non_401 |

## saved_searches.rs  (mount: /api/v1/saved-searches)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/saved-searches | list_saved_searches | done | saved_searches_authz_tests.rs | list_saved_searches_unauthenticated_returns_401, list_saved_searches_authenticated_returns_non_401 |
| POST | /api/v1/saved-searches | create_saved_search | done | saved_searches_authz_tests.rs | create_saved_search_unauthenticated_returns_401, create_saved_search_authenticated_returns_non_401 |
| GET | /api/v1/saved-searches/alerts | list_search_alerts | done | saved_searches_authz_tests.rs | list_search_alerts_unauthenticated_returns_401, list_search_alerts_authenticated_returns_non_401 (OpenAPI drift) |
| POST | /api/v1/saved-searches/alerts/read-all | mark_all_alerts_read | done | saved_searches_authz_tests.rs | mark_all_alerts_read_unauthenticated_returns_401, mark_all_alerts_read_authenticated_returns_non_401 |
| POST | /api/v1/saved-searches/alerts/{alert_id}/read | mark_alert_read | done | saved_searches_authz_tests.rs | mark_alert_read_unauthenticated_returns_401, mark_alert_read_authenticated_unknown_returns_non_401 |
| GET | /api/v1/saved-searches/{id} | get_saved_search | done | saved_searches_authz_tests.rs | get_saved_search_unauthenticated_returns_401, get_saved_search_authenticated_unknown_returns_non_401 |
| PUT | /api/v1/saved-searches/{id} | update_saved_search | done | saved_searches_authz_tests.rs | update_saved_search_unauthenticated_returns_401, update_saved_search_authenticated_unknown_returns_non_401 |
| DELETE | /api/v1/saved-searches/{id} | delete_saved_search | done | saved_searches_authz_tests.rs | delete_saved_search_unauthenticated_returns_401, delete_saved_search_authenticated_unknown_returns_non_401 |
| POST | /api/v1/saved-searches/{id}/run | run_saved_search | done | saved_searches_authz_tests.rs | run_saved_search_unauthenticated_returns_401, run_saved_search_authenticated_unknown_returns_non_401 |

## inquiries.rs  (mount: /api/v1/inquiries)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/inquiries/contact/{listing_id} | send_contact_message | done | inquiries_authz_tests.rs | send_contact_message_unauthenticated_does_not_return_401 (public endpoint, OpenAPI drift) |
| POST | /api/v1/inquiries/viewing/{listing_id} | request_viewing | done | inquiries_authz_tests.rs | request_viewing_unauthenticated_does_not_return_401 (public endpoint) |
| GET | /api/v1/inquiries | list_my_inquiries | done | inquiries_authz_tests.rs | list_my_inquiries_unauthenticated_returns_401, list_my_inquiries_authenticated_returns_non_401 (HTTP; inquiry_pagination_tests.rs is repo-level) |
| GET | /api/v1/inquiries/mine | list_buyer_inquiries | done | inquiries_authz_tests.rs | list_buyer_inquiries_unauthenticated_returns_401, list_buyer_inquiries_authenticated_returns_non_401 (HTTP; buyer_inquiries_tests.rs is repo-level) |
| GET | /api/v1/inquiries/{id} | get_inquiry | done | inquiries_authz_tests.rs | get_inquiry_unauthenticated_returns_401, get_inquiry_authenticated_unknown_returns_non_401 |
| PUT | /api/v1/inquiries/{id}/read | mark_as_read | done | inquiries_authz_tests.rs | mark_as_read_unauthenticated_returns_401, mark_as_read_authenticated_unknown_returns_non_401 (HTTP; inquiry_idor_tests.rs is repo-level) |
| POST | /api/v1/inquiries/{id}/respond | respond_to_inquiry | done | inquiries_authz_tests.rs | respond_to_inquiry_unauthenticated_returns_401, respond_to_inquiry_authenticated_unknown_returns_non_401 |

## sso.rs  (mount: /api/v1/sso)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/sso/login | sso_login | done | sso_authz_tests.rs | Real; public endpoint — non-401 (302 redirect) verified |
| GET | /api/v1/sso/callback | sso_callback | done | sso_authz_tests.rs | Real; public endpoint — non-401 (400 missing params) verified |
| POST | /api/v1/sso/logout | sso_logout | done | sso_authz_tests.rs | Real; cookie-protected — 401 without portal_session cookie |
| POST | /api/v1/sso/mobile/token | create_mobile_sso_token | done | sso_authz_tests.rs | Real; body-validated — non-401 without body (422), 401 with invalid PM token |
| POST | /api/v1/sso/mobile/validate | validate_mobile_sso_token | done | sso_authz_tests.rs | Real; body-validated — non-401 without body (422), 401 with invalid SSO token |
| GET | /api/v1/sso/session | get_session | done | sso_authz_tests.rs | Real; Bearer-protected — 401 without token |
| POST | /api/v1/sso/refresh | refresh_session | done | sso_authz_tests.rs | Real; Bearer-protected — 401 without token |
| POST | /api/v1/sso/exchange | exchange_pm_token | done | sso_authz_tests.rs | Real; body-validated — non-401 without body (422), 401 with invalid PM token; OpenAPI drift |
| POST | /api/v1/sso/sync | sync_session | done | sso_authz_tests.rs | Real; body-validated — non-401 without body (422), 401 with invalid PM token; OpenAPI drift |
| GET | /api/v1/sso/roles | get_mapped_roles | done | sso_authz_tests.rs | Real; public static — 200 verified; OpenAPI drift |

## agencies.rs  (mount: /api/v1/agencies)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/agencies | create_agency | done | agencies_authz_tests.rs | Real; auth boundary + happy path |
| GET | /api/v1/agencies | list_agencies | done | agencies_authz_tests.rs | Real; public — non-401 verified; OpenAPI drift |
| GET | /api/v1/agencies/{id} | get_agency | done | agencies_authz_tests.rs | Real; public — non-401 (404 for unknown id) verified |
| PUT | /api/v1/agencies/{id} | update_agency | done | agencies_authz_tests.rs | Real; auth boundary + happy path |
| GET | /api/v1/agencies/{id}/members | list_members | done | agencies_authz_tests.rs | Real; public — non-401 verified |
| POST | /api/v1/agencies/{id}/invitations | create_invitation | done | agencies_authz_tests.rs | Real; auth boundary + happy path |
| GET | /api/v1/agencies/by-slug/{slug} | get_agency_by_slug | done | agencies_authz_tests.rs | Real; public — non-401 (404 for unknown slug) verified |
| POST | /api/v1/agencies/invitations/{token}/accept | accept_invitation | done | agencies_authz_tests.rs | Real; auth boundary + happy path |

## realtors.rs  (mount: /api/v1/realtors)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/realtors/profile | get_my_profile | partial | — | Real |
| POST | /api/v1/realtors/profile | create_profile | partial | — | Real (upsert) |
| PUT | /api/v1/realtors/profile | update_profile | partial | — | Real |
| GET | /api/v1/realtors/{user_id}/profile | get_profile | partial | — | Real |
| GET | /api/v1/realtors/inquiries | list_inquiries | partial | inquiry_pagination_tests.rs (repo-level) | Real; test exercises repo not endpoint |
| POST | /api/v1/realtors/inquiries/{id}/read | mark_inquiry_read | partial | — | Real |
| POST | /api/v1/realtors/inquiries/{id}/respond | respond_to_inquiry | partial | — | Real |

## imports.rs  (mount: /api/v1/imports)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/imports/jobs | list_import_jobs | partial | — | Real |
| POST | /api/v1/imports/jobs | create_import_job | partial | — | Real |
| GET | /api/v1/imports/jobs/{id} | get_import_job | done | imports_idor_tests.rs | Real; happy-path 200 (import_job_owner_returns_200) + IDOR 404/401 |
| PUT | /api/v1/imports/jobs/{id} | update_import_job | partial | — | Real |
| POST | /api/v1/imports/jobs/{id}/start | start_import_job | partial | — | Real |
| POST | /api/v1/imports/jobs/{id}/cancel | cancel_import_job | partial | — | Real |
| GET | /api/v1/imports/feeds | list_feeds | partial | imports_idor_tests.rs (403 only) | Real; only authz test (feed_caller_without_agency_returns_403), no happy path |
| POST | /api/v1/imports/feeds | create_feed | partial | — | Real |
| GET | /api/v1/imports/feeds/{id} | get_feed | done | imports_idor_tests.rs | Real; happy-path 200 (feed_is_shared_across_agency_members) + 404 |
| PUT | /api/v1/imports/feeds/{id} | update_feed | partial | — | Real |
| POST | /api/v1/imports/feeds/{id}/sync | sync_feed | partial | — | Real |

## portal_listings.rs  (mount: /api/v1/my/listings)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/my/listings | create_listing | done | portal_listings_create_tests.rs | Real; auth boundary (401/201), happy-path 201 success, validation 400 rejections |
| GET | /api/v1/my/listings/{id} | get_my_listing | done | portal_listings_idor_tests.rs | Real; happy-path 200 (get_owner_returns_200) + IDOR 404/401 |
| PATCH | /api/v1/my/listings/{id} | update_listing | done | portal_listings_idor_tests.rs | Real; happy-path 200 (patch_owner_returns_200, patch_status_paused_is_accepted_200) |

## compare.rs  (mount: /api/v1/compare)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/compare | get_compare_list | done | compare_tests.rs | get_compare_list_unauthenticated_returns_401, get_compare_list_authenticated_returns_200 |
| POST | /api/v1/compare/{listing_id} | add_to_compare | done | compare_tests.rs | add_to_compare_unauthenticated_returns_401, add_to_compare_unknown_listing_returns_404 |
| DELETE | /api/v1/compare/{listing_id} | remove_from_compare | done | compare_tests.rs | remove_from_compare_unauthenticated_returns_401, remove_from_compare_authenticated_not_in_list_returns_404 |

## reports.rs  (mount: /api/v1/reports)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/reports | submit_report | done | reports_tests.rs | submit_report_unauthenticated_invalid_listing_returns_404_or_unprocessable |
| GET | /api/v1/reports/me | list_my_reports | done | reports_tests.rs | list_my_reports_unauthenticated_returns_401, list_my_reports_authenticated_returns_200 |

## agent_reviews.rs  (mount: /api/v1/realtors/{id}/reviews)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/realtors/{id}/reviews | list_reviews | done | agent_reviews_tests.rs | list_reviews_unknown_realtor_returns_200_empty |
| POST | /api/v1/realtors/{id}/reviews | create_review | done | agent_reviews_tests.rs | create_review_unauthenticated_returns_401 |

## agency_branding.rs  (mount: /api/v1/agencies/{id}/branding)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/agencies/{id}/branding | get_branding | partial | — | Real (sqlx agency_branding) |
| PUT | /api/v1/agencies/{id}/branding | update_branding | partial | — | Real (upsert agency_branding) |

## agency_imports.rs  (mount: /api/v1/agencies/{id}/imports)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/agencies/{id}/imports | list_import_history | partial | — | Real (portal_import_jobs) |
| POST | /api/v1/agencies/{id}/imports/test-connection | test_connection | stub | — | STUB: returns hardcoded "Connection ... successful (stub response)", sample_record_count: Some(42). Membership + SSRF URL checks are real, but provider call is faked |
| POST | /api/v1/agencies/{id}/imports/run | run_import | partial | — | Real (INSERT portal_import_jobs) |
| GET | /api/v1/agencies/{id}/imports/{job_id} | get_import_job_status | partial | — | Real (portal_import_jobs) |

## price_map.rs  (mount: /api/v1/price-map)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/price-map | get_price_map | done | price_map_tests.rs | get_price_map_returns_200, get_price_map_with_filters_returns_200 |

## articles.rs  (mount: /api/v1/articles)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/articles | list_articles | done | articles_tests.rs | list_articles_returns_200 |
| GET | /api/v1/articles/{slug} | get_article | done | articles_tests.rs | get_article_unknown_slug_returns_404 |
| GET | /api/v1/articles/{slug}/comments | list_comments | done | articles_tests.rs | list_comments_unknown_slug_returns_404 |
| POST | /api/v1/articles/{slug}/comments | create_comment | done | articles_tests.rs | create_comment_unauthenticated_returns_401, create_comment_authenticated_unknown_article_returns_404 |

## Summary
- done: 74 | partial: 21 | stub: 1 | missing: 0 | total: 96
