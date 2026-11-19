# Shipped Stories — Events & KPI Definitions

> Scope: a KPI/analytics backfill for stories marked `done` in
> `_bmad-output/implementation-artifacts/sprint-status.yaml` (the
> `development_status:` map). For each shipped story it records the
> **success event(s)** the feature produces and the **KPI** that reads those
> events, so a dashboard or a scheduled metrics job has one place to look up
> "what does *done* mean in numbers?".
> Audience: pm-data / analytics, product owners, anyone wiring a delivery or
> product-health dashboard.
> Sibling reference: [`dispute-lifecycle-kpis.md`](./dispute-lifecycle-kpis.md)
> (Epic 77/80 funnel) and [`layout-publish-event-tracking.md`](./layout-publish-event-tracking.md)
> (layout publish events) go deeper per-feature; this doc is the breadth-first
> backfill across every shipped story.

This is a **definitions reference**, not a query cookbook. SQL is illustrative
(Postgres dialect) and omits the `organization_id` / RLS scoping that every real
query must add.

## How to read this doc

Each row has three parts:

| Field | Meaning |
|-------|---------|
| **Success event(s)** | The user- or system-level action whose occurrence means the story is being used as intended. Written as `verb_noun` so it maps cleanly onto an analytics event name even where no event table is wired yet. |
| **KPI** | A named metric computed from those events (count, rate, latency percentile), with its formula. |
| **Backing** | Where the event is observable today — a concrete endpoint, repository method, migration, or table named in the story's `sprint-status` note. |

**Instrumentation status.** Most PPT features are observable through their write
paths (API request + the row they persist) but do **not** yet emit a dedicated
analytics event into a telemetry table. Unless a row's **Backing** names an
explicit event/audit table (e.g. `support_tooling_events`, `dispute_activities`),
treat the KPI as **derivable from domain rows** (count/aggregate the entity
table) and the event name as the **proposed** analytics name to use when first-
party instrumentation lands. Rows whose backing *is* an event/audit table are
marked **(instrumented)**.

## Story template — Events / KPI section (the new column)

Every new story under `_bmad-output/implementation-artifacts/stories/` should
carry an **Events / KPI** section so the success metric is defined at authoring
time rather than backfilled later. Paste the block below into the story template,
after `## Acceptance Criteria` and before `## Tasks / Subtasks`:

```markdown
## Events / KPI

| Success event(s) | KPI | Backing |
|------------------|-----|---------|
| `verb_noun` — what the user/system did | Metric name = formula (count / rate / pXX latency) | endpoint, repo method, table, or migration that makes the event observable; mark **(instrumented)** if it writes a dedicated event/audit row |

> Instrumentation status: `derivable from domain rows` | `(instrumented)` —
> if no telemetry table exists yet, the event name above is the **proposed**
> analytics name to adopt when instrumentation lands.
```

The same three-column shape (`Success event(s) | KPI | Backing`) is used by the
backfill tables below, so a story's section and this reference stay symmetric.

---

## Epic 6 — Announcements & Communication

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 6-1 announcement-creation-targeting | `announcement_created`, `announcement_published` | Announcements published / manager / week; share of announcements using a narrowed target (`target_type != all`) ÷ all published | `POST /api/v1/announcements` (routes/announcements/crud.rs), `announcements` table (mig 00015), `target_type`/`target_ids` JSONB |
| 6-2 announcement-viewing-acknowledgment | `announcement_read`, `announcement_acknowledged` | **Acknowledgment rate** = distinct `acknowledged` ÷ targeted recipients; **read rate** = distinct `read` ÷ targeted | `POST /{id}/read`, `POST /{id}/acknowledge`, acknowledgment-stats endpoint (routes/announcements), `get_acknowledgment_stats` repo method |
| 6-3 announcement-comments-discussion | `announcement_comment_created`, `announcement_comment_deleted` | Comments / published announcement (discussion density); reply depth distribution (cap = 2) | `POST /{id}/comments`, `announcement_comments` (mig 00016, threading `parent_id` + soft-delete) |
| 6-4 pinned-announcements | `announcement_pinned`, `announcement_auto_unpinned` | Active pins / org (cap = 3); mean pin dwell-time until manual unpin or 30-day auto-expire | `pin_with_cap_rls`/`unpin_rls`, `pinned_at`/`pinned_by` cols (mig 00015), auto-unpin scheduler |
| 6-5 direct-messaging | `message_thread_created`, `message_sent`, `message_read` | Messages sent / active user / week; thread reply rate = threads with ≥1 reply ÷ threads created; unread backlog | `routes/messaging.rs` (threads/messages/unread-count), `messaging` repo, migs 00017-00019/00189-00191 |
| 6-6 neighbor-information | `neighbor_directory_viewed` | Directory views / resident / month (engagement proxy) | neighbor-information feature (derivable from domain rows) |

## Epic 7A — Basic Document Management

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 7a-1 document-upload-metadata | `document_uploaded` | Uploads / org / week; upload success rate = 2xx ÷ attempts; mean file size by category | `POST /api/v1/documents/upload` (documents/core.rs), `documents` table (mig 00020), S3 `create_rls` |
| 7a-2 folder-organization | `document_folder_created`, `document_moved` | Documents filed into a folder ÷ total documents (organization rate); folder depth distribution | `routes/documents/folders.rs` (6 endpoints), folder tables |
| 7a-3 permission-based-access | `document_access_granted`, `document_access_denied` | Access-denied ÷ access-attempts (gate pressure); share of documents using a non-`organization` scope | `document_access_allowed` + `scope_membership_allows`, `access_scope` enum (mig 00020), 00172 FORCE RLS |
| 7a-4 document-download-preview | `document_downloaded`, `document_previewed` | Downloads + previews / document (reuse); preview-unsupported rate = 400 ÷ preview attempts | `GET /{id}/download`, `GET /{id}/preview` (presigned S3, 15-min / 1-hr TTL) |
| 7a-5 document-sharing | `document_shared`, `document_share_revoked` | Shares / document; external vs internal share mix; mean share lifetime | `routes/documents/shares.rs`, `document_share` repo + enum |

## Epic 8A — Basic Notification Preferences

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 8a-1 channel-level-notification-toggles | `notification_channel_toggled` | Share of users who disabled ≥1 channel (opt-out rate) per channel (email/push/in-app) | notification-preferences channel toggles |
| 8a-2 critical-notification-override | `critical_notification_override_set` | Share of users overriding a critical-notification default; critical deliveries that bypassed a disabled channel | critical-override preference rows |
| 8a-3 notification-preference-sync | `notification_preference_synced` (WebSocket) | Preference-change → device-sync latency (p50/p95); sync success rate | WebSocket realtime sync (PR #472); mobile OS push (FCM/APNs) deferred |

## Epic 9 — Multi-Factor Authentication

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 9-1 totp-2fa-setup | `mfa_enrolled`, `mfa_verified`, `mfa_backup_code_used` | **MFA adoption** = users with MFA enabled ÷ eligible users; verify success rate; backup-code usage rate | `routes/mfa.rs` (setup/verify/disable/status/backup-codes), `two_factor_auth` repo, `services/totp.rs` |

## Epic 10A — OAuth Provider Foundation

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 10a-1 oauth-authorization-server | `oauth_authorize_granted`, `oauth_authorize_denied` | Authorization grant rate; PKCE-S256 compliance = S256 flows ÷ all (target 100%) | `routes/oauth.rs` authorize endpoint (PKCE S256 enforced) |
| 10a-2 oauth-client-registration | `oauth_client_registered` | Registered clients / org; active clients = clients with ≥1 grant in 30d ÷ registered | `routes/oauth.rs` client registration |
| 10a-3 oauth-token-management | `oauth_token_issued`, `oauth_token_revoked`, `oauth_token_refreshed` | Token revocation rate; refresh-to-issue ratio; revoked-token reuse attempts (must be 0 — RFC 9700) | `routes/oauth.rs` token/introspect/revoke, `revoked_at` checks |

## Epic 10B — Platform Administration

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 10b-1 organization-management-dashboard | `org_dashboard_viewed` | Admin dashboard sessions / week (ops engagement) | platform-admin org dashboard |
| 10b-2 feature-flag-management | `feature_flag_changed` | Flag changes / week; flags enabled ÷ total (rollout coverage) | feature-flag management routes |
| 10b-3 platform-health-monitoring | `health_check_recorded` | Health-check pass rate; mean time-to-detect a degraded subsystem | platform-health monitoring |
| 10b-4 system-announcements | `system_announcement_published`, `system_announcement_acknowledged` | Platform-wide ack rate; reach = distinct recipients ÷ active orgs | `SystemAnnouncementRepository` CRUD + ack, `public_announcements_router` |
| 10b-5 support-data-access **(instrumented)** | `support_data_accessed` | Support-access events / week / admin; access-without-reason rate (should be 0) | `GET /api/v1/platform-admin/support-data`, append-only **`support_tooling_events`** audit trail (mig 00163, RLS + ON DELETE RESTRICT 00165) — see [support-tooling-audit-events.md](./support-tooling-audit-events.md) |
| 10b-6 user-onboarding-tour | `onboarding_tour_started`, `onboarding_step_completed`, `onboarding_tour_completed`, `onboarding_tour_skipped` | **Tour completion rate** = completed ÷ started; per-step drop-off; skip rate | `/api/v1/onboarding` (`start_tour`/`complete_step`/`complete_tour`/`skip_tour`), `OnboardingRepository` |
| 10b-7 contextual-help-documentation | `help_article_viewed`, `help_tooltip_opened` | Help views / active user; top-viewed articles; deflection proxy (views preceding no support contact) | `/api/v1/help` (11 handlers), `contextual_help` (mig 00034), `HelpRepository` |

## Epic 79 — ppt-web API-client / app integration

Integration-layer stories: their KPIs are **client reliability** metrics rather
than product-usage metrics.

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 79-1 api-client-integration | `api_request_completed` | Client request success rate = 2xx/3xx ÷ all; p95 request latency | generated `@ppt/api-client`, wired fetch helper |
| 79-2 authentication-flow | `login_succeeded`, `login_failed`, `token_refreshed` | Login success rate; silent-refresh success rate; re-auth frequency | AuthContext / LoginPage / ProtectedRoute / AuthCallbackPage |
| 79-3 error-handling-toasts | `error_toast_shown`, `offline_detected` | Error-toast rate / session; time spent offline / session | Toast provider, `useNetworkStatus`, `errorHandler`, OfflineIndicator |
| 79-4 websocket-realtime | `ws_connected`, `ws_message_delivered`, `ws_reconnected` | WS uptime = connected time ÷ session time; reconnect rate; message-delivery latency | `lib/websocket.ts`, WebSocketContext, `useWebSocket`, ConnectionStatus |

## Epic 80 — Dispute Resolution (frontend wiring)

See [`dispute-lifecycle-kpis.md`](./dispute-lifecycle-kpis.md) for the full
funnel; the rows below are the story-level success signals.

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 80-1 disputes-api-integration | `dispute_list_viewed` | Dispute list sessions / manager; API success rate | disputes api-client integration |
| 80-2 dispute-filing-flow | `dispute_draft_saved`, `dispute_filed` | **Filing completion rate** = filed ÷ drafts started; draft-restore rate (AC-4 auto-save) | `useDraftStorage` (debounced localStorage), FileDisputePage, `POST /disputes` |
| 80-3 mediation-resolution | `mediation_submission_created`, `dispute_resolved` | Submissions / dispute in mediation; mediation → resolved conversion | `GET/POST /api/v1/disputes/{id}/submissions`, MediationSubmissionsPanel, `dispute_activities` **(instrumented)** |

## Epic 82 — Reality Portal iOS (SwiftUI)

Client-shell stories; KPIs are **mobile engagement/reliability**.

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 82-1 swiftui-project-setup | `app_launched` | iOS DAU / MAU; cold-start time | iosApp RealityPortalApp / MainTabView / DI container |
| 82-2 navigation-routing | `screen_viewed`, `deep_link_opened` | Screens / session; deep-link open rate | NavigationCoordinator, Route, DeepLinkHandler, state restoration |
| 82-3 home-search-screens | `search_performed` | Searches / session; zero-result search rate | HomeView, SearchView |
| 82-4 listing-detail-favorites | `listing_viewed`, `listing_favorited` | Listing detail views / session; favorite rate = favorited ÷ viewed | ListingDetailView, ListingMapView, FavoritesService |
| 82-5 inquiries-account | `inquiry_sent` | Inquiries / listing view (conversion); account-screen visits | InquiriesView, SendInquiryView, AccountView (9 stub destinations tracked in audit) |

## Epic 84 — Backend Integrations

System-integration stories; KPIs are **integration throughput / reliability**.

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 84-1 s3-presigned-urls | `presigned_url_issued` | Presigned URLs issued / day; URL-expiry-before-use rate | `crates/integrations/storage.rs` (SigV4, `S3_PRESIGNED_URL_TTL_SECS`) |
| 84-2 esignature-email | `signature_request_sent`, `signature_completed` | **Signature completion rate** = completed ÷ requested; mean time-to-sign; reminder effectiveness | `services/email.rs` send_signature_request_email, `routes/signatures.rs`, `signature_request` repo |
| 84-3 price-tracking | `price_change_detected`, `favorite_alert_sent` | Alerts sent / tracked listing; price-change → alert latency | `reality-server services/favorite_alerts.rs`, `listing_price_history`, `favorite_alert_queue` (mig 00194) |
| 84-4 notification-triggers | `notification_triggered` | Triggered notifications / day by type; trigger → dispatch latency | `crates/common/notifications`, `services/email.rs` + scheduler |
| 84-5 pgvector-rag | `embedding_written`, `rag_query_served` | Documents embedded ÷ total documents (index coverage); embedding write success rate | `embedding.rs` (EmbeddingProvider), `document_embeddings vector(1536)` (mig 00081) |

## Epic 85 — Mobile Environment Configuration

Build/config stories — no runtime product KPI; success is **build-config
correctness**, verified by the tests named in `sprint-status` rather than a
telemetry metric.

| Story | Success event(s) | KPI | Backing |
|-------|------------------|-----|---------|
| 85-1 environment-variables | `config_loaded` (per-env) | Build-config test pass rate (per environment) | `.env.{development,staging,production}`, `src/config/api.ts`, `app.config.ts`; `app.config.env.test.ts` |
| 85-2 build-configuration | `build_produced` | Build success rate per flavor | Expo `app.config.ts` iOS build config |

---

## Notes

- **Coverage.** This backfill covers the 44 stories marked `done` in
  `development_status:` as of branch
  `auto-impl/pm-data-backfill-kpi-definitions-stories-60320845`. Stories still in
  `backlog`/`ready-for-dev` are intentionally excluded — add a row here when a
  story is promoted to `done`.
- **Proposed vs instrumented.** Only rows marked **(instrumented)** read from a
  dedicated event/audit table today (`support_tooling_events`,
  `dispute_activities`). Every other event name is the **proposed** analytics
  name; until first-party telemetry lands, compute the KPI by aggregating the
  domain rows named in **Backing** (e.g. count `announcements`, join
  acknowledgment rows). Keep the proposed names stable so a later telemetry layer
  can adopt them without breaking dashboards.
- **RLS.** Every illustrative aggregate omits `organization_id` scoping; real
  queries must add it (and respect `FORCE ROW LEVEL SECURITY` where migrations
  set it, e.g. documents 00172).
