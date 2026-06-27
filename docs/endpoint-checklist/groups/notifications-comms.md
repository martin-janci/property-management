# Notifications & Communications

_Server: api-server. Modules: messaging.rs, notification_preferences.rs, granular_notifications.rs, critical_notifications.rs, ws_notifications.rs, news_articles.rs, announcements/ (crud, lifecycle, engagement, comments, stats, ai_draft)._

## messaging.rs  (mount: /api/v1/messages)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/messages/threads | list_threads | done | messaging_group_conversations_tests.rs, messaging_thread_state_authz_tests.rs | 200 happy-path asserts body (total/search) |
| POST | /api/v1/messages/threads | start_thread | done | messaging_group_conversations_tests.rs | 200 happy-path (group thread create) |
| GET | /api/v1/messages/threads/{id} | get_thread | done | messaging_group_conversations_tests.rs | 200 happy-path; cross-tenant 403 also covered |
| DELETE | /api/v1/messages/threads/{id} | delete_thread | done | messaging_thread_state_authz_tests.rs | 200 happy-path; verifies per-user soft hide |
| POST | /api/v1/messages/threads/{id}/archive | archive_thread | partial | messaging_thread_state_authz_tests.rs | only FORBIDDEN (non-participant); no success path |
| DELETE | /api/v1/messages/threads/{id}/archive | unarchive_thread | partial | none | no test |
| POST | /api/v1/messages/threads/{id}/messages | send_message | partial | none | no direct happy-path test |
| DELETE | /api/v1/messages/threads/{id}/messages/{message_id} | delete_message | partial | none | also re-exported in main.rs (OpenAPI), same mount |
| POST | /api/v1/messages/threads/{id}/attachments/upload-url | request_attachment_upload_url | partial | none | no test |
| POST | /api/v1/messages/threads/{id}/messages/{message_id}/attachments | link_message_attachment | done | messaging_attachments_authz_tests.rs | 201 CREATED happy-path |
| GET | /api/v1/messages/threads/{id}/messages/{message_id}/attachments | list_message_attachments | done | messaging_attachments_authz_tests.rs | 200 happy-path asserts count/fileName |
| GET | /api/v1/messages/threads/{id}/attachments/{attachment_id}/download | get_attachment_download_url | partial | messaging_attachments_authz_tests.rs | authz-gate only: participant reaches storage (503 in no-S3 harness), non-participant/cross-org 403; no 200 success |
| POST | /api/v1/messages/threads/{id}/read | mark_thread_read | partial | none | no test |
| GET | /api/v1/messages/users/blocked | list_blocked_users | partial | none | no test |
| POST | /api/v1/messages/users/{id}/block | block_user | partial | none | no test |
| DELETE | /api/v1/messages/users/{id}/block | unblock_user | partial | none | no test |
| GET | /api/v1/messages/unread-count | get_unread_count | partial | none | no test |

## notification_preferences.rs  (mount: /api/v1/users/me/notification-preferences)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/users/me/notification-preferences | get_preferences | done | notification_preference_realtime_sync_tests.rs | 200 happy-path asserts state |
| PATCH | /api/v1/users/me/notification-preferences/{channel} | update_preference | done | notification_preference_realtime_sync_tests.rs | 200 happy-path (email/push/in_app/sms); CONFLICT/BAD_REQUEST edge cases also covered |

## granular_notifications.rs  (mount: /api/v1/users/me/notification-preferences/granular)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/users/me/notification-preferences/granular/events | list_event_preferences | partial | none | no test |
| PUT | /api/v1/users/me/notification-preferences/granular/events/{event_type} | update_event_preference | partial | none | no test |
| POST | /api/v1/users/me/notification-preferences/granular/events/reset | reset_event_preferences | partial | none | no test |
| PUT | /api/v1/users/me/notification-preferences/granular/events/category/{category} | update_category_preferences | partial | none | no test |
| GET | /api/v1/users/me/notification-preferences/granular/schedule | get_schedule | partial | none | no test |
| PUT | /api/v1/users/me/notification-preferences/granular/schedule | update_schedule | partial | none | no test |
| GET | /api/v1/users/me/notification-preferences/granular/roles | list_role_defaults | partial | none | no test (role_defaults_rbac only hits {role}) |
| GET | /api/v1/users/me/notification-preferences/granular/roles/{role} | get_role_defaults | partial | none | no test |
| PUT | /api/v1/users/me/notification-preferences/granular/roles/{role} | update_role_defaults | partial | granular_notif_role_defaults_rbac_tests.rs | RBAC-only: FORBIDDEN, no success path |
| DELETE | /api/v1/users/me/notification-preferences/granular/roles/{role} | delete_role_defaults | partial | granular_notif_role_defaults_rbac_tests.rs | RBAC-only: FORBIDDEN, no success path |
| POST | /api/v1/users/me/notification-preferences/granular/roles/{role}/apply | apply_role_defaults | partial | none | no test |
| GET | /api/v1/users/me/notification-preferences/granular/groups | list_notification_groups | partial | none | no test |
| GET | /api/v1/users/me/notification-preferences/granular/groups/{group_id} | get_notification_group | partial | none | no test |
| DELETE | /api/v1/users/me/notification-preferences/granular/groups/{group_id} | delete_notification_group | partial | none | no test |
| POST | /api/v1/users/me/notification-preferences/granular/groups/{group_id}/read | mark_group_read | partial | none | no test |
| POST | /api/v1/users/me/notification-preferences/granular/groups/read-all | mark_all_groups_read | partial | none | no test |
| GET | /api/v1/users/me/notification-preferences/granular/digests | list_digests | partial | none | no test |

## critical_notifications.rs  (mount: /api/v1/organizations/{org_id}/critical-notifications)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/organizations/{org_id}/critical-notifications | create_notification | partial | none | real handler (admin gate + repo create); no test |
| GET | /api/v1/organizations/{org_id}/critical-notifications | list_notifications | partial | none | no test |
| GET | /api/v1/organizations/{org_id}/critical-notifications/unacknowledged | get_unacknowledged | partial | none | no test |
| POST | /api/v1/organizations/{org_id}/critical-notifications/{notification_id}/acknowledge | acknowledge | partial | none | no test |
| GET | /api/v1/organizations/{org_id}/critical-notifications/{notification_id}/stats | get_stats | partial | none | no test |

## ws_notifications.rs  (mount: /api/v1/users/me/notifications)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/users/me/notifications/ws | ws_handler | done | ws_integration_tests.rs | real HTTP upgrade → 101 success (ws_upgrade_with_valid_token_succeeds); expired/refresh/no-token → 401 also covered |

## news_articles.rs  (mount: /api/v1/news)
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/news | list_articles | partial | none | real handler (repo.list by tenant); no test |
| POST | /api/v1/news | create_article | partial | none | no test |
| GET | /api/v1/news/{id} | get_article | partial | none | no test |
| PUT | /api/v1/news/{id} | update_article | partial | none | no test |
| DELETE | /api/v1/news/{id} | delete_article | partial | none | no test |
| POST | /api/v1/news/{id}/publish | publish_article | partial | none | no test |
| POST | /api/v1/news/{id}/archive | archive_article | partial | none | no test |
| POST | /api/v1/news/{id}/restore | restore_article | partial | none | no test |
| POST | /api/v1/news/{id}/pin | pin_article | partial | none | no test |
| GET | /api/v1/news/{id}/media | list_media | partial | none | no test |
| POST | /api/v1/news/{id}/media | add_media | partial | none | no test |
| DELETE | /api/v1/news/{id}/media/{media_id} | delete_media | partial | none | no test |
| POST | /api/v1/news/{id}/reactions | toggle_reaction | partial | none | no test |
| GET | /api/v1/news/{id}/reactions/counts | get_reaction_counts | partial | none | no test |
| GET | /api/v1/news/{id}/comments | list_comments | partial | none | no test |
| POST | /api/v1/news/{id}/comments | create_comment | partial | none | no test |
| PUT | /api/v1/news/{id}/comments/{comment_id} | update_comment | partial | none | no test |
| DELETE | /api/v1/news/{id}/comments/{comment_id} | delete_comment | partial | none | no test |
| POST | /api/v1/news/{id}/comments/{comment_id}/moderate | moderate_comment | partial | none | no test |
| GET | /api/v1/news/{id}/comments/{comment_id}/replies | list_comment_replies | partial | none | no test |
| POST | /api/v1/news/{id}/view | record_view | partial | none | no test |
| GET | /api/v1/news/statistics | get_statistics | partial | none | no test |

## announcements/ (mount: /api/v1/announcements)

### crud.rs
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/announcements | create_announcement | partial | endpoints_smoke_tests.rs | smoke = auth-only (assert_protected); no happy-path |
| GET | /api/v1/announcements | list_announcements | partial | endpoints_smoke_tests.rs | smoke = auth-only; no happy-path |
| GET | /api/v1/announcements/published | list_published_announcements | partial | none | no test |
| GET | /api/v1/announcements/{id} | get_announcement | partial | none | no test |
| PUT | /api/v1/announcements/{id} | update_announcement | partial | none | no test |
| DELETE | /api/v1/announcements/{id} | delete_announcement | partial | none | no test |

### lifecycle.rs
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/announcements/{id}/publish | publish_announcement | partial | none | no test |
| POST | /api/v1/announcements/{id}/schedule | schedule_announcement | partial | none | no test |
| POST | /api/v1/announcements/{id}/archive | archive_announcement | partial | none | no test |
| POST | /api/v1/announcements/{id}/pin | pin_announcement | partial | none | also accepts PATCH (same handler) |
| PATCH | /api/v1/announcements/{id}/pin | pin_announcement | partial | none | same handler as POST /pin |

### engagement.rs
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/announcements/{id}/attachments | list_attachments | partial | none | no test |
| POST | /api/v1/announcements/{id}/attachments | add_attachment | partial | none | no test |
| DELETE | /api/v1/announcements/{id}/attachments/{attachment_id} | delete_attachment | partial | none | no test |
| POST | /api/v1/announcements/{id}/read | mark_read | partial | none | no test |
| POST | /api/v1/announcements/{id}/acknowledge | acknowledge | partial | none | no test |
| GET | /api/v1/announcements/{id}/acknowledgments | get_acknowledgments | partial | none | no test |

### comments.rs
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/announcements/{id}/comments | list_comments | done | announcement_comments_threading_tests.rs | 200 happy-path |
| POST | /api/v1/announcements/{id}/comments | create_comment | done | announcement_comments_threading_tests.rs | 201 happy-path; threading/nesting validation covered |
| DELETE | /api/v1/announcements/{id}/comments/{comment_id} | delete_comment | done | announcement_comments_threading_tests.rs | 200 happy-path; idempotent re-delete 400 |

### stats.rs
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| GET | /api/v1/announcements/statistics | get_statistics | partial | none | no test |
| GET | /api/v1/announcements/unread-count | get_unread_count | partial | none | no test |

### ai_draft.rs
| Method | Path | Handler | Status | Tests | Notes |
|---|---|---|---|---|---|
| POST | /api/v1/announcements/ai-draft | generate_ai_draft | partial | none | no test |

## Summary
- done: 12 | partial: 75 | stub: 0 | missing: 0 | total: 87
