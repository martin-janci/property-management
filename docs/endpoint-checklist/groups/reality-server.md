# Reality Server — Portal Endpoints

_Server: reality-server. Modules: agencies, agency_branding, agency_imports, agent_reviews, articles, compare, favorites, health, imports, inquiries, listings, portal_listings, price_map, realtors, reports, saved_searches, sso, users._

> Test reality: only `imports_idor_tests.rs` and `portal_listings_idor_tests.rs` exercise real HTTP routes (build the module router + `oneshot`). The inquiry tests (`buyer_inquiries_tests.rs`, `inquiry_idor_tests.rs`, `inquiry_pagination_tests.rs`) hit `RealityPortalRepository` **directly**, NOT the endpoint, so they do NOT count as path coverage. `favorite_alert_worker_tests.rs` and `search_alert_drainer_tests.rs` test background workers, not endpoints. `raw_pool_audit_tests.rs` uses a fake stub handler. Per spec, repo-level/worker tests → endpoints stay `partial`.

## health.rs  (mount: / via main.rs `.route`)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /health | liveness | partial | — | Real liveness probe (static service info, by design); no HTTP test |
| GET | /readiness | readiness | partial | — | Real deep dep check (DB + PM API); no HTTP test |

## listings.rs  (mount: /api/v1/listings)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/listings | search | partial | — | Real (portal_repo.search_listings + count); raw_pool_audit uses a fake stub handler at this path, not the real one |
| GET | /api/v1/listings/featured | get_featured | partial | — | Real (fetch_top_listings) |
| GET | /api/v1/listings/categories | get_categories | partial | — | Real (sqlx count by property_type) |
| GET | /api/v1/listings/suggestions | get_suggestions | partial | — | Real (portal_repo.get_nearby_cities) |
| GET | /api/v1/listings/{id} | get_listing | partial | — | Real (sqlx full listing + photos); raw_pool_audit uses fake stub at this path |
| POST | /api/v1/listings/{id}/view | record_view | partial | — | Real (reality_portal_repo.track_view) |

## users.rs  (mount: /api/v1/users)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/users/register | register | partial | — | Real; not in OpenAPI paths list (doc drift) |
| POST | /api/v1/users/login | login | partial | — | Real; OpenAPI drift |
| POST | /api/v1/users/password-reset | request_password_reset | partial | — | Real; OpenAPI drift |
| POST | /api/v1/users/password-reset/confirm | confirm_password_reset | partial | — | Real; OpenAPI drift |
| POST | /api/v1/users/logout | logout | partial | — | Real; OpenAPI drift |
| GET | /api/v1/users/me | get_me | partial | — | Real; OpenAPI drift |
| PUT | /api/v1/users/me | update_me | partial | — | Real; OpenAPI drift |

## favorites.rs  (mount: /api/v1/favorites)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/favorites | list_favorites | partial | — | Real |
| GET | /api/v1/favorites/alerts | list_favorite_alerts | partial | — | Real; not in OpenAPI list (drift) |
| POST | /api/v1/favorites/alerts/read-all | mark_all_favorite_alerts_read | partial | — | Real; OpenAPI drift |
| POST | /api/v1/favorites/alerts/{alert_id}/read | mark_favorite_alert_read | partial | — | Real; OpenAPI drift |
| GET | /api/v1/favorites/ids | list_favorite_ids | partial | — | Real |
| POST | /api/v1/favorites/{listing_id} | add_favorite | partial | — | Real |
| DELETE | /api/v1/favorites/{listing_id} | remove_favorite | partial | — | Real |
| GET | /api/v1/favorites/{listing_id}/check | check_favorite | partial | — | Real |

## saved_searches.rs  (mount: /api/v1/saved-searches)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/saved-searches | list_saved_searches | partial | — | Real |
| POST | /api/v1/saved-searches | create_saved_search | partial | — | Real |
| GET | /api/v1/saved-searches/alerts | list_search_alerts | partial | — | Real; not in OpenAPI list (drift) |
| POST | /api/v1/saved-searches/alerts/read-all | mark_all_alerts_read | partial | — | Real; OpenAPI drift |
| POST | /api/v1/saved-searches/alerts/{alert_id}/read | mark_alert_read | partial | — | Real; OpenAPI drift |
| GET | /api/v1/saved-searches/{id} | get_saved_search | partial | — | Real |
| PUT | /api/v1/saved-searches/{id} | update_saved_search | partial | — | Real |
| DELETE | /api/v1/saved-searches/{id} | delete_saved_search | partial | — | Real |
| POST | /api/v1/saved-searches/{id}/run | run_saved_search | partial | — | Real (search_listings + count) |

## inquiries.rs  (mount: /api/v1/inquiries)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/inquiries/contact/{listing_id} | send_contact_message | partial | — | Real; module not in OpenAPI list (drift). inquiry tests are repo-level, not HTTP |
| POST | /api/v1/inquiries/viewing/{listing_id} | request_viewing | partial | — | Real; repo-level tests only |
| GET | /api/v1/inquiries | list_my_inquiries | partial | inquiry_pagination_tests.rs (repo-level) | Real; test exercises repo not endpoint |
| GET | /api/v1/inquiries/mine | list_buyer_inquiries | partial | buyer_inquiries_tests.rs (repo-level) | Real; test exercises repo not endpoint |
| GET | /api/v1/inquiries/{id} | get_inquiry | partial | — | Real |
| PUT | /api/v1/inquiries/{id}/read | mark_as_read | partial | inquiry_idor_tests.rs (repo-level) | Real; test exercises repo not endpoint |
| POST | /api/v1/inquiries/{id}/respond | respond_to_inquiry | partial | — | Real |

## sso.rs  (mount: /api/v1/sso)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/sso/login | sso_login | partial | — | Real (OAuth PKCE init) |
| GET | /api/v1/sso/callback | sso_callback | partial | — | Real |
| POST | /api/v1/sso/logout | sso_logout | partial | — | Real |
| POST | /api/v1/sso/mobile/token | create_mobile_sso_token | partial | — | Real |
| POST | /api/v1/sso/mobile/validate | validate_mobile_sso_token | partial | — | Real |
| GET | /api/v1/sso/session | get_session | partial | — | Real |
| POST | /api/v1/sso/refresh | refresh_session | partial | — | Real |
| POST | /api/v1/sso/exchange | exchange_pm_token | partial | — | Real; not in OpenAPI list (drift) |
| POST | /api/v1/sso/sync | sync_session | partial | — | Real; OpenAPI drift |
| GET | /api/v1/sso/roles | get_mapped_roles | partial | — | Real (static role-mapping config); OpenAPI drift |

## agencies.rs  (mount: /api/v1/agencies)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/agencies | create_agency | partial | — | Real |
| GET | /api/v1/agencies | list_agencies | partial | — | Real; not in OpenAPI list (drift) |
| GET | /api/v1/agencies/{id} | get_agency | partial | — | Real |
| PUT | /api/v1/agencies/{id} | update_agency | partial | — | Real |
| GET | /api/v1/agencies/{id}/members | list_members | partial | — | Real |
| POST | /api/v1/agencies/{id}/invitations | create_invitation | partial | — | Real |
| GET | /api/v1/agencies/by-slug/{slug} | get_agency_by_slug | partial | — | Real |
| POST | /api/v1/agencies/invitations/{token}/accept | accept_invitation | partial | — | Real |

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
| POST | /api/v1/my/listings | create_listing | partial | portal_listings_idor_tests.rs (400 only) | Real; only validation 400 tests, no success path |
| GET | /api/v1/my/listings/{id} | get_my_listing | done | portal_listings_idor_tests.rs | Real; happy-path 200 (get_owner_returns_200) + IDOR 404/401 |
| PATCH | /api/v1/my/listings/{id} | update_listing | done | portal_listings_idor_tests.rs | Real; happy-path 200 (patch_owner_returns_200, patch_status_paused_is_accepted_200) |

## compare.rs  (mount: /api/v1/compare)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/compare | get_compare_list | partial | — | Real (compare_lists + listings join; NULL-safe placeholder comment, not a stub) |
| POST | /api/v1/compare/{listing_id} | add_to_compare | partial | — | Real |
| DELETE | /api/v1/compare/{listing_id} | remove_from_compare | partial | — | Real |

## reports.rs  (mount: /api/v1/reports)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/reports | submit_report | partial | — | Real (INSERT listing_reports) |
| GET | /api/v1/reports/me | list_my_reports | partial | — | Real |

## agent_reviews.rs  (mount: /api/v1/realtors/{id}/reviews)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/realtors/{id}/reviews | list_reviews | partial | — | Real (realtor_reviews + users) |
| POST | /api/v1/realtors/{id}/reviews | create_review | partial | — | Real (INSERT realtor_reviews) |

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
| GET | /api/v1/price-map | get_price_map | partial | — | Real (sqlx district aggregation) |

## articles.rs  (mount: /api/v1/articles)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/articles | list_articles | partial | — | Real (reality_articles) |
| GET | /api/v1/articles/{slug} | get_article | partial | — | Real |
| GET | /api/v1/articles/{slug}/comments | list_comments | partial | — | Real |
| POST | /api/v1/articles/{slug}/comments | create_comment | partial | — | Real (INSERT reality_article_comments) |

## Summary
- done: 4 | partial: 91 | stub: 1 | missing: 0 | total: 96
