# Announcements, Messaging & Notifications endpoints

| Method + Path | Handler | Status | Test | Notes |
|---|---|---|---|---|
| `POST /api/v1/announcements` | `announcements/crud.rs:create_announcement` | partial | `endpoints_smoke_tests.rs` | Smoke auth-only, no behaviour assert |
| `GET /api/v1/announcements` | `announcements/crud.rs:list_announcements` | partial | `endpoints_smoke_tests.rs` | Smoke auth-only |
| `GET /api/v1/announcements/published` | `announcements/crud.rs:list_published_announcements` | partial | — | Only repo-level residency test |
| `GET /api/v1/announcements/{id}` | `announcements/crud.rs:get_announcement` | partial | — | Real, untested |
| `PUT /api/v1/announcements/{id}` | `announcements/crud.rs:update_announcement` | partial | — | Real, untested |
| `DELETE /api/v1/announcements/{id}` | `announcements/crud.rs:delete_announcement` | partial | — | Real, untested |
| `POST /api/v1/announcements/{id}/publish` | `announcements/lifecycle.rs:publish_announcement` | partial | — | Real, untested |
| `POST /api/v1/announcements/{id}/schedule` | `announcements/lifecycle.rs:schedule_announcement` | partial | — | Real, untested |
| `POST /api/v1/announcements/{id}/archive` | `announcements/lifecycle.rs:archive_announcement` | partial | — | Real, untested |
| `POST/PATCH /api/v1/announcements/{id}/pin` | `announcements/lifecycle.rs:pin_announcement` | partial | — | Real, untested |
| `GET /api/v1/announcements/{id}/attachments` | `announcements/engagement.rs:list_attachments` | partial | — | Real, untested |
| `POST /api/v1/announcements/{id}/attachments` | `announcements/engagement.rs:add_attachment` | partial | — | Real, untested |
| `DELETE /api/v1/announcements/{id}/attachments/{attachment_id}` | `announcements/engagement.rs:delete_attachment` | partial | — | Real, untested |
| `POST /api/v1/announcements/{id}/read` | `announcements/engagement.rs:mark_read` | partial | — | Real, untested |
| `POST /api/v1/announcements/{id}/acknowledge` | `announcements/engagement.rs:acknowledge` | partial | — | Real, untested |
| `GET /api/v1/announcements/{id}/acknowledgments` | `announcements/engagement.rs:get_acknowledgments` | partial | — | Real, untested |
| `GET /api/v1/announcements/{id}/comments` | `announcements/comments.rs:list_comments` | done | `announcement_comments_threading_tests.rs` | |
| `POST /api/v1/announcements/{id}/comments` | `announcements/comments.rs:create_comment` | done | `announcement_comments_threading_tests.rs` | |
| `DELETE /api/v1/announcements/{id}/comments/{comment_id}` | `announcements/comments.rs:delete_comment` | done | `announcement_comments_threading_tests.rs` | |
| `GET /api/v1/announcements/statistics` | `announcements/stats.rs:get_statistics` | partial | — | Real, untested |
| `GET /api/v1/announcements/unread-count` | `announcements/stats.rs:get_unread_count` | partial | — | Real, untested |
| `POST /api/v1/announcements/ai-draft` | `announcements/ai_draft.rs:generate_ai_draft` | partial | — | Real LLM call, untested |
| `GET /api/v1/messages/threads` | `messaging.rs:list_threads` | partial | `endpoints_smoke_tests.rs` | Smoke auth-only |
| `POST /api/v1/messages/threads` | `messaging.rs:start_thread` | partial | — | Real, untested |
| `GET /api/v1/messages/threads/{id}` | `messaging.rs:get_thread` | partial | — | Real, untested |
| `POST /api/v1/messages/threads/{id}/messages` | `messaging.rs:send_message` | partial | — | Real, untested |
| `DELETE /api/v1/messages/threads/{id}/messages/{message_id}` | `messaging.rs:delete_message` | partial | — | Real, untested |
| `POST /api/v1/messages/threads/{id}/read` | `messaging.rs:mark_thread_read` | partial | — | Real, untested |
| `GET /api/v1/messages/users/blocked` | `messaging.rs:list_blocked_users` | partial | — | Real, untested |
| `POST /api/v1/messages/users/{id}/block` | `messaging.rs:block_user` | partial | — | Real, untested |
| `DELETE /api/v1/messages/users/{id}/block` | `messaging.rs:unblock_user` | partial | — | Real, untested |
| `GET /api/v1/messages/unread-count` | `messaging.rs:get_unread_count` | partial | — | Real, untested |
| `POST /api/v1/organizations/{org_id}/critical-notifications` | `critical_notifications.rs:create_notification` | partial | — | Real, untested |
| `GET /api/v1/organizations/{org_id}/critical-notifications` | `critical_notifications.rs:list_notifications` | partial | — | Real, untested |
| `GET /api/v1/organizations/{org_id}/critical-notifications/unacknowledged` | `critical_notifications.rs:get_unacknowledged` | partial | — | Real, untested |
| `POST /api/v1/organizations/{org_id}/critical-notifications/{notification_id}/acknowledge` | `critical_notifications.rs:acknowledge` | partial | — | Real, untested |
| `GET /api/v1/organizations/{org_id}/critical-notifications/{notification_id}/stats` | `critical_notifications.rs:get_stats` | partial | — | Real, untested |
| `GET /api/v1/users/me/notification-preferences/granular/events` | `granular_notifications.rs:list_event_preferences` | partial | — | Real, untested |
| `PUT /api/v1/users/me/notification-preferences/granular/events/{event_type}` | `granular_notifications.rs:update_event_preference` | partial | — | Real, untested |
| `POST /api/v1/users/me/notification-preferences/granular/events/reset` | `granular_notifications.rs:reset_event_preferences` | partial | — | Real, untested |
| `PUT /api/v1/users/me/notification-preferences/granular/events/category/{category}` | `granular_notifications.rs:update_category_preferences` | partial | — | Real, untested |
| `GET/PUT /api/v1/users/me/notification-preferences/granular/schedule` | `granular_notifications.rs:get_schedule/update_schedule` | partial | — | Real, untested |
| `GET /api/v1/users/me/notification-preferences/granular/roles` | `granular_notifications.rs:list_role_defaults` | partial | — | Real, untested |
| `GET /api/v1/users/me/notification-preferences/granular/roles/{role}` | `granular_notifications.rs:get_role_defaults` | partial | — | Real, untested |
| `PUT /api/v1/users/me/notification-preferences/granular/roles/{role}` | `granular_notifications.rs:update_role_defaults` | done | `granular_notif_role_defaults_rbac_tests.rs` | RBAC 403 assert |
| `DELETE /api/v1/users/me/notification-preferences/granular/roles/{role}` | `granular_notifications.rs:delete_role_defaults` | done | `granular_notif_role_defaults_rbac_tests.rs` | RBAC 403 assert |
| `POST /api/v1/users/me/notification-preferences/granular/roles/{role}/apply` | `granular_notifications.rs:apply_role_defaults` | partial | — | Real, untested |
| `GET /api/v1/users/me/notification-preferences/granular/groups` | `granular_notifications.rs:list_notification_groups` | partial | — | Real, untested |
| `GET /api/v1/users/me/notification-preferences/granular/groups/{group_id}` | `granular_notifications.rs:get_notification_group` | partial | — | Real, untested |
| `DELETE /api/v1/users/me/notification-preferences/granular/groups/{group_id}` | `granular_notifications.rs:delete_notification_group` | partial | — | Real, untested |
| `POST /api/v1/users/me/notification-preferences/granular/groups/{group_id}/read` | `granular_notifications.rs:mark_group_read` | partial | — | Real, untested |
| `POST /api/v1/users/me/notification-preferences/granular/groups/read-all` | `granular_notifications.rs:mark_all_groups_read` | partial | — | Real, untested |
| `GET /api/v1/users/me/notification-preferences/granular/digests` | `granular_notifications.rs:list_digests` | partial | — | Real, untested |
| `GET /api/v1/users/me/notification-preferences` | `notification_preferences.rs:get_preferences` | done | `notification_preference_realtime_sync_tests.rs` | |
| `PATCH /api/v1/users/me/notification-preferences/{channel}` | `notification_preferences.rs:update_preference` | done | `notification_preference_realtime_sync_tests.rs` | |
| `POST /api/v1/users/me/push-tokens` | `push_tokens.rs:register_push_token` | done | `push_token_tests.rs` | |
| `DELETE /api/v1/users/me/push-tokens/{token}` | `push_tokens.rs:unregister_push_token` | done | `push_token_tests.rs` | |
| `GET /api/v1/users/me/notifications/ws` | `ws_notifications.rs:ws_handler` | done | `ws_integration_tests.rs` | 101 upgrade asserted |
| `GET/POST /api/v1/news` | `news_articles.rs:list_articles/create_article` | partial | — | Real, untested |
| `GET/PUT/DELETE /api/v1/news/{id}` | `news_articles.rs:get_article/update_article/delete_article` | partial | — | Real, untested |
| `POST /api/v1/news/{id}/publish` | `news_articles.rs:publish_article` | partial | — | Real, untested |
| `POST /api/v1/news/{id}/archive` | `news_articles.rs:archive_article` | partial | — | Real, untested |
| `POST /api/v1/news/{id}/restore` | `news_articles.rs:restore_article` | partial | — | Real, untested |
| `POST /api/v1/news/{id}/pin` | `news_articles.rs:pin_article` | partial | — | Real, untested |
| `GET/POST /api/v1/news/{id}/media` | `news_articles.rs:list_media/add_media` | partial | — | Real, untested |
| `DELETE /api/v1/news/{id}/media/{media_id}` | `news_articles.rs:delete_media` | partial | — | Real, untested |
| `POST /api/v1/news/{id}/reactions` | `news_articles.rs:toggle_reaction` | partial | — | Real, untested |
| `GET /api/v1/news/{id}/reactions/counts` | `news_articles.rs:get_reaction_counts` | partial | — | Real, untested |
| `GET/POST /api/v1/news/{id}/comments` | `news_articles.rs:list_comments/create_comment` | partial | — | Real, untested |
| `PUT/DELETE /api/v1/news/{id}/comments/{comment_id}` | `news_articles.rs:update_comment/delete_comment` | partial | — | Real, untested |
| `POST /api/v1/news/{id}/comments/{comment_id}/moderate` | `news_articles.rs:moderate_comment` | partial | — | Real, untested |
| `GET /api/v1/news/{id}/comments/{comment_id}/replies` | `news_articles.rs:list_comment_replies` | partial | — | Real, untested |
| `POST /api/v1/news/{id}/view` | `news_articles.rs:record_view` | partial | — | Real, untested |
| `GET /api/v1/news/statistics` | `news_articles.rs:get_statistics` | partial | — | Real, untested |
| `GET /api/v1/help/articles` | `help.rs:list_articles` | done | `help_tests.rs` | |
| `GET /api/v1/help/articles/search` | `help.rs:search_articles` | done | `help_tests.rs` | |
| `GET /api/v1/help/articles/category/{category}` | `help.rs:list_articles_by_category` | done | `help_tests.rs` | |
| `GET /api/v1/help/articles/context/{context_key}` | `help.rs:list_articles_by_context` | done | `help_tests.rs` | |
| `GET /api/v1/help/articles/{slug}` | `help.rs:get_article` | done | `help_tests.rs` | Not-found assert |
| `POST /api/v1/help/articles/{slug}/feedback` | `help.rs:submit_article_feedback` | partial | `help_tests.rs` | Auth-only assert |
| `GET /api/v1/help/categories` | `help.rs:list_categories` | done | `help_tests.rs` | |
| `GET /api/v1/help/categories/{slug}` | `help.rs:get_category` | done | `help_tests.rs` | Not-found assert |
| `GET /api/v1/help/faq` | `help.rs:list_faq` | done | `help_tests.rs` | |
| `GET /api/v1/help/faq/search` | `help.rs:search_faq` | done | `help_tests.rs` | |
| `GET /api/v1/help/faq/category/{category}` | `help.rs:list_faq_by_category` | done | `help_tests.rs` | |
| `GET /api/v1/help/tooltips` | `help.rs:list_tooltips` | done | `help_tests.rs` | |
| `GET /api/v1/help/tooltips/{key}` | `help.rs:get_tooltip` | done | `help_tests.rs` | Not-found assert |
| `GET /api/v1/help/tooltips/prefix/{prefix}` | `help.rs:list_tooltips_by_prefix` | done | `help_tests.rs` | |
| `GET /api/v1/automation/organizations/{org_id}/rules` | `automation.rs:list_rules` | done | `automation_auth_tests.rs` | Authz assert |
| `POST /api/v1/automation/organizations/{org_id}/rules` | `automation.rs:create_rule` | partial | — | Real, not in auth test |
| `GET /api/v1/automation/rules/{id}` | `automation.rs:get_rule` | done | `automation_auth_tests.rs` | Authz assert |
| `PUT /api/v1/automation/rules/{id}` | `automation.rs:update_rule` | done | `automation_auth_tests.rs` | Authz assert |
| `DELETE /api/v1/automation/rules/{id}` | `automation.rs:delete_rule` | done | `automation_auth_tests.rs` | Authz assert |
| `POST /api/v1/automation/rules/{id}/toggle` | `automation.rs:toggle_rule` | done | `automation_auth_tests.rs` | Authz assert |
| `GET /api/v1/automation/rules/{id}/logs` | `automation.rs:get_rule_logs` | done | `automation_auth_tests.rs` | Authz assert |
| `GET /api/v1/automation/templates` | `automation.rs:list_templates` | done | `automation_auth_tests.rs` | Authz assert |
| `GET /api/v1/automation/templates/{id}` | `automation.rs:get_template` | done | `automation_auth_tests.rs` | Authz assert |
| `POST /api/v1/automation/organizations/{org_id}/rules/from-template` | `automation.rs:create_from_template` | partial | — | Real, not in auth test |

## Tally
done: 31  partial: 67  stub: 0  missing: 0  total: 98
