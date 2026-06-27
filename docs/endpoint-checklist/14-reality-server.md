# Reality Server endpoints

Public Reality Portal API (`backend/servers/reality-server`, port 8081) + SSO consumer.
Mount prefixes resolved from `src/main.rs`. All listed modules are mounted (none commented out).
`done` requires a test exercising the route handler end-to-end (oneshot against the router).
Inquiry repo-layer tests (`inquiry_idor`, `buyer_inquiries`, `inquiry_pagination`) hit
`RealityPortalRepository` directly, not the handlers → those handlers stay `partial`.

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `GET /health` | `health.rs:liveness` | partial | — | shallow liveness, no test |
| `GET /readiness` | `health.rs:readiness` | partial | — | deep dep check, no test |
| `GET /api/v1/listings` | `listings.rs:search` | done | `raw_pool_audit_tests.rs` | oneshot search |
| `GET /api/v1/listings/featured` | `listings.rs:get_featured` | partial | — | real query, no test |
| `GET /api/v1/listings/categories` | `listings.rs:get_categories` | partial | — | real query, no test |
| `GET /api/v1/listings/suggestions` | `listings.rs:get_suggestions` | partial | — | real query, no test |
| `GET /api/v1/listings/{id}` | `listings.rs:get_listing` | done | `raw_pool_audit_tests.rs` | oneshot detail |
| `POST /api/v1/listings/{id}/view` | `listings.rs:record_view` | partial | — | real insert, no test |
| `POST /api/v1/users/register` | `users.rs:register` | partial | — | real handler, no test |
| `POST /api/v1/users/login` | `users.rs:login` | partial | — | real handler, no test |
| `POST /api/v1/users/password-reset` | `users.rs:request_password_reset` | partial | — | real handler, no test |
| `POST /api/v1/users/password-reset/confirm` | `users.rs:confirm_password_reset` | partial | — | real handler, no test |
| `POST /api/v1/users/logout` | `users.rs:logout` | partial | — | real handler, no test |
| `GET /api/v1/users/me` | `users.rs:get_me` | partial | — | real handler, no test |
| `PUT /api/v1/users/me` | `users.rs:update_me` | partial | — | real handler, no test |
| `GET /api/v1/favorites` | `favorites.rs:list_favorites` | partial | — | real query, no test |
| `GET /api/v1/favorites/ids` | `favorites.rs:list_favorite_ids` | partial | — | real query, no test |
| `POST /api/v1/favorites/{listing_id}` | `favorites.rs:add_favorite` | partial | — | real insert, no test |
| `DELETE /api/v1/favorites/{listing_id}` | `favorites.rs:remove_favorite` | partial | — | real delete, no test |
| `GET /api/v1/favorites/{listing_id}/check` | `favorites.rs:check_favorite` | partial | — | real query, no test |
| `GET /api/v1/saved-searches` | `saved_searches.rs:list_saved_searches` | partial | — | real query, no test |
| `POST /api/v1/saved-searches` | `saved_searches.rs:create_saved_search` | partial | — | real insert, no test |
| `GET /api/v1/saved-searches/alerts` | `saved_searches.rs:list_search_alerts` | partial | — | real query, no test |
| `POST /api/v1/saved-searches/alerts/read-all` | `saved_searches.rs:mark_all_alerts_read` | partial | — | real update, no test |
| `POST /api/v1/saved-searches/alerts/{alert_id}/read` | `saved_searches.rs:mark_alert_read` | partial | — | real update, no test |
| `GET /api/v1/saved-searches/{id}` | `saved_searches.rs:get_saved_search` | partial | — | real query, no test |
| `PUT /api/v1/saved-searches/{id}` | `saved_searches.rs:update_saved_search` | partial | — | real update, no test |
| `DELETE /api/v1/saved-searches/{id}` | `saved_searches.rs:delete_saved_search` | partial | — | real delete, no test |
| `POST /api/v1/saved-searches/{id}/run` | `saved_searches.rs:run_saved_search` | partial | — | real query, no test |
| `POST /api/v1/inquiries/contact/{listing_id}` | `inquiries.rs:send_contact_message` | partial | — | real insert, no test |
| `POST /api/v1/inquiries/viewing/{listing_id}` | `inquiries.rs:request_viewing` | partial | — | real insert, no test |
| `GET /api/v1/inquiries` | `inquiries.rs:list_my_inquiries` | partial | repo-only | pagination repo test only |
| `GET /api/v1/inquiries/mine` | `inquiries.rs:list_buyer_inquiries` | partial | repo-only | buyer repo test only |
| `GET /api/v1/inquiries/{id}` | `inquiries.rs:get_inquiry` | partial | — | real query, no test |
| `PUT /api/v1/inquiries/{id}/read` | `inquiries.rs:mark_as_read` | partial | repo-only | IDOR fix tested at repo layer |
| `POST /api/v1/inquiries/{id}/respond` | `inquiries.rs:respond_to_inquiry` | partial | repo-only | IDOR fix tested at repo layer |
| `GET /api/v1/sso/login` | `sso.rs:sso_login` | partial | — | real handler, no test |
| `GET /api/v1/sso/callback` | `sso.rs:sso_callback` | partial | — | real handler, no test |
| `POST /api/v1/sso/logout` | `sso.rs:sso_logout` | partial | — | real handler, no test |
| `POST /api/v1/sso/mobile/token` | `sso.rs:create_mobile_sso_token` | partial | — | real handler, no test |
| `POST /api/v1/sso/mobile/validate` | `sso.rs:validate_mobile_sso_token` | partial | — | real handler, no test |
| `GET /api/v1/sso/session` | `sso.rs:get_session` | partial | — | real handler, no test |
| `POST /api/v1/sso/refresh` | `sso.rs:refresh_session` | partial | — | real handler, no test |
| `POST /api/v1/sso/exchange` | `sso.rs:exchange_pm_token` | partial | — | real handler, no test |
| `POST /api/v1/sso/sync` | `sso.rs:sync_session` | partial | — | real handler, no test |
| `GET /api/v1/sso/roles` | `sso.rs:get_mapped_roles` | partial | — | real handler, no test |
| `POST /api/v1/agencies` | `agencies.rs:create_agency` | partial | — | real insert, no test |
| `GET /api/v1/agencies` | `agencies.rs:list_agencies` | partial | — | real query, no test |
| `GET /api/v1/agencies/{id}` | `agencies.rs:get_agency` | partial | — | real query, no test |
| `PUT /api/v1/agencies/{id}` | `agencies.rs:update_agency` | partial | — | real update, no test |
| `GET /api/v1/agencies/{id}/members` | `agencies.rs:list_members` | partial | — | real query, no test |
| `POST /api/v1/agencies/{id}/invitations` | `agencies.rs:create_invitation` | partial | — | real insert, no test |
| `GET /api/v1/agencies/by-slug/{slug}` | `agencies.rs:get_agency_by_slug` | partial | — | real query, no test |
| `POST /api/v1/agencies/invitations/{token}/accept` | `agencies.rs:accept_invitation` | partial | — | real handler, no test |
| `GET /api/v1/realtors/profile` | `realtors.rs:get_my_profile` | partial | — | real query, no test |
| `POST /api/v1/realtors/profile` | `realtors.rs:create_profile` | partial | — | real insert, no test |
| `PUT /api/v1/realtors/profile` | `realtors.rs:update_profile` | partial | — | real update, no test |
| `GET /api/v1/realtors/{user_id}/profile` | `realtors.rs:get_profile` | partial | — | real query, no test |
| `GET /api/v1/realtors/inquiries` | `realtors.rs:list_inquiries` | partial | repo-only | pagination repo test only |
| `POST /api/v1/realtors/inquiries/{id}/read` | `realtors.rs:mark_inquiry_read` | partial | repo-only | IDOR fix tested at repo layer |
| `POST /api/v1/realtors/inquiries/{id}/respond` | `realtors.rs:respond_to_inquiry` | partial | repo-only | IDOR fix tested at repo layer |
| `GET /api/v1/imports/jobs` | `imports.rs:list_import_jobs` | partial | — | not oneshot-tested |
| `POST /api/v1/imports/jobs` | `imports.rs:create_import_job` | partial | — | helper-only in test |
| `GET /api/v1/imports/jobs/{id}` | `imports.rs:get_import_job` | done | `imports_idor_tests.rs` | oneshot IDOR |
| `PUT /api/v1/imports/jobs/{id}` | `imports.rs:update_import_job` | partial | — | real update, no test |
| `POST /api/v1/imports/jobs/{id}/start` | `imports.rs:start_import_job` | partial | — | real handler, no test |
| `POST /api/v1/imports/jobs/{id}/cancel` | `imports.rs:cancel_import_job` | partial | — | real handler, no test |
| `GET /api/v1/imports/feeds` | `imports.rs:list_feeds` | done | `imports_idor_tests.rs` | oneshot list |
| `POST /api/v1/imports/feeds` | `imports.rs:create_feed` | partial | — | real insert, no test |
| `GET /api/v1/imports/feeds/{id}` | `imports.rs:get_feed` | done | `imports_idor_tests.rs` | oneshot IDOR |
| `PUT /api/v1/imports/feeds/{id}` | `imports.rs:update_feed` | partial | — | real update, no test |
| `POST /api/v1/imports/feeds/{id}/sync` | `imports.rs:sync_feed` | partial | — | real handler, no test |
| `POST /api/v1/my/listings` | `portal_listings.rs:create_listing` | partial | — | real insert, no test |
| `GET /api/v1/my/listings/{id}` | `portal_listings.rs:get_my_listing` | partial | — | real query, no test |
| `PATCH /api/v1/my/listings/{id}` | `portal_listings.rs:update_listing` | partial | — | real update, no test |
| `GET /api/v1/compare` | `compare.rs:get_compare_list` | partial | — | real query, no test |
| `POST /api/v1/compare/{listing_id}` | `compare.rs:add_to_compare` | partial | — | real insert, no test |
| `DELETE /api/v1/compare/{listing_id}` | `compare.rs:remove_from_compare` | partial | — | real delete, no test |
| `POST /api/v1/reports` | `reports.rs:submit_report` | partial | — | real insert, no test |
| `GET /api/v1/reports/me` | `reports.rs:list_my_reports` | partial | — | real query, no test |
| `GET /api/v1/realtors/{id}/reviews` | `agent_reviews.rs:list_reviews` | partial | — | real query, no test |
| `POST /api/v1/realtors/{id}/reviews` | `agent_reviews.rs:create_review` | partial | — | real insert, no test |
| `GET /api/v1/agencies/{id}/branding` | `agency_branding.rs:get_branding` | partial | — | real query, no test |
| `PUT /api/v1/agencies/{id}/branding` | `agency_branding.rs:update_branding` | partial | — | real update, no test |
| `GET /api/v1/agencies/{id}/imports` | `agency_imports.rs:list_import_history` | partial | — | real query, no test |
| `POST /api/v1/agencies/{id}/imports/test-connection` | `agency_imports.rs:test_connection` | partial | — | real handler, no test |
| `POST /api/v1/agencies/{id}/imports/run` | `agency_imports.rs:run_import` | partial | — | real handler, no test |
| `GET /api/v1/agencies/{id}/imports/{job_id}` | `agency_imports.rs:get_import_job_status` | partial | — | real query, no test |
| `GET /api/v1/price-map` | `price_map.rs:get_price_map` | partial | — | real aggregation, no test |
| `GET /api/v1/articles` | `articles.rs:list_articles` | partial | — | real query, no test |
| `GET /api/v1/articles/{slug}` | `articles.rs:get_article` | partial | — | real query, no test |
| `GET /api/v1/articles/{slug}/comments` | `articles.rs:list_comments` | partial | — | real query, no test |
| `POST /api/v1/articles/{slug}/comments` | `articles.rs:create_comment` | partial | — | real insert, no test |

## Tally
done: 5  partial: 86  stub: 0  missing: 0  total: 91
