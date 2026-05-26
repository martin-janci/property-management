# PPT Action List

_Generated: 2026-05-26T22:32:04Z — 58 open, 14 in-progress, 37 done._

| Priority | Status | Action | Owner | Dependency | Source |
|---|---|---|---|---|---|
| high | open | Add OAuth 2.0 authorization server integration tests: full PKCE S256 authorization code flow, consent/revok... | pm-backend | none | gap-scan 2026-05-24 |
| high | open | Build admin health monitoring frontend page in admin-web wired to backend health dashboard + metric history... | pm-frontend | none | gap-scan 2026-05-24 |
| high | open | Wire AnnouncementsPage and FaultsPage to @ppt/api-client query hooks — infra exists (queryKeys/axios interc... | pm-frontend | none | gap-scan 2026-05-24 |
| high | open | Build dedicated folder-tree UI page for document organization — backend supports 5-level hierarchy + circul... | pm-frontend | None | gap-scan |
| high | open | Implement mobile document download/preview with presigned URLs — web preview (react-pdf) shipped in PR #446... | pm-frontend | none | gap-scan 2026-05-24 |
| high | open | Build WebSocket realtime sync infra for notification preferences and integrate with notification preference... | pm-backend | Epic 2B WebSocket infrastructure | gap-scan |
| high | open | Implement Epic 81 report-schedule endpoints the frontend already calls (/schedules/{id}/pause, /resume, /ex... | pm-backend | none | pm-analysis 2026-05-24 |
| high | open | Add CI secret-scanning + env-scoped secrets handling for new OTA channel credentials (Booking.com/Airbnb AP... | pm-devops | pm-security | pm-analysis 2026-05-26 |
| high | open | Add metrics/logging + dead-letter + backoff monitoring for the new push-fanout background worker (push_fano... | pm-devops | pm-backend | pm-analysis 2026-05-26 |
| high | open | Create backend/servers/reality-server/tests/inquiry_idor_tests.rs covering PR #497: realtor B mark A inquir... | pm-qa | rust-backend | pm-analysis 2026-05-25 |
| high | open | Verify respond_to_inquiry (POST /api/v1/inquiries/{id}/respond) enforces realtor ownership at repo layer; a... | pm-qa | rust-backend | pm-analysis 2026-05-25 |
| high | open | Implement POST /api/v1/documents/upload backend handler to unblock story 7a-1 (not promotable per PR #502) | pm-scrum-master | pm-backend | pm-analysis 2026-05-25 |
| high | open | Wire App.tsx dispute routes: replace inline JSX stub with DisputeDetailPage and add /disputes/:disputeId/me... | pm-scrum-master | pm-frontend | pm-analysis 2026-05-25 |
| high | open | Resolve the residual post-merge security findings in issues #438/#439 — P0-12 cookie scope, P1-04 Debug-for... | pm-security | none | pm-analysis 2026-05-24 |
| medium | open | Implement system announcements handler bodies in platform_admin.rs — CRUD routes mounted but handlers retur... | pm-backend | none | gap-scan 2026-05-24 |
| medium | open | Implement support data access handler bodies — search/get user/memberships/sessions/activity routes defined... | pm-backend | none | gap-scan 2026-05-24 |
| medium | open | Build Support Data page in admin-web calling /api/v1/platform-admin/support-data; show tenant diagnostics t... | pm-frontend | gap-10b-5-support-data-api | gap-scan 2026-05-25 |
| medium | open | Implement user onboarding tour handler bodies in onboarding.rs — get/start/complete/skip/reset routes mount... | pm-backend | none | gap-scan 2026-05-24 |
| medium | open | Implement contextual help handler bodies in help.rs — articles/FAQ/tooltips with search+context routes moun... | pm-backend | none | gap-scan 2026-05-24 |
| medium | open | Add POST/GET /api/v1/announcements/{id}/comments endpoints and migration for announcement_comments table | pm-backend | none | gap-scan 2026-05-25 |
| medium | open | Add PATCH /api/v1/announcements/{id}/pin endpoint and is_pinned column in migration; return pinned announce... | pm-backend | none | gap-scan 2026-05-25 |
| medium | open | Show pinned announcements in sticky band at top of announcements list in ppt-web and mobile | pm-frontend | gap-6-4-pinned-announcements-api | gap-scan 2026-05-25 |
| medium | open | Build folder browser UI in ppt-web document library: create folder, move documents, breadcrumb navigation | pm-frontend | gap-7a-2-folder-api | gap-scan 2026-05-25 |
| medium | open | Add download button and PDF/image inline preview modal in ppt-web document list | pm-frontend | gap-7a-4-document-download-api | gap-scan 2026-05-25 |
| medium | open | Add POST /api/v1/disputes endpoint with migration for disputes table; implement dispute state machine (open... | pm-backend | none | gap-scan 2026-05-25 |
| medium | open | Build dispute filing form in ppt-web with reason selector, description, and evidence upload | pm-frontend | gap-80-2-dispute-filing-api | gap-scan 2026-05-25 |
| medium | open | Verify dispute filing flow (FileDisputePage + EvidenceUploader) meets all acceptance criteria; update story... | pm-frontend | none | gap-scan 2026-05-24 |
| medium | open | Build mediation workspace in ppt-web: dispute timeline, resolution form, manager/tenant chat thread | pm-frontend | gap-80-3-mediation-api | gap-scan 2026-05-25 |
| medium | open | Verify mediation resolution flow (MediationPage/DisputeDetailPage) meets all ACs including escalation paths... | pm-frontend | none | gap-scan 2026-05-24 |
| medium | open | Add PUT /api/v1/reports/schedules/{id} to update cron expression, recipients, and enabled flag | pm-backend | none | gap-scan 2026-05-25 |
| medium | open | Build report schedule editor modal in ppt-web: cron picker, recipient list, enabled toggle | pm-frontend | gap-81-1-report-schedule-edit-api | gap-scan 2026-05-25 |
| medium | open | Verify EditScheduleModal pause/resume works end-to-end against new backend endpoints from PR #448; confirm ... | pm-frontend | none | gap-scan 2026-05-24 |
| medium | open | Verify ExecutionHistory UI works end-to-end against new backend execution endpoints from PR #448; validate ... | pm-frontend | none | gap-scan 2026-05-24 |
| medium | open | Add GET /api/v1/reports/schedules/{id}/executions returning paginated execution log with status and downloa... | pm-backend | none | gap-scan 2026-05-25 |
| medium | open | Implement HomeView and SearchView screens in SwiftUI iOS app with listing cards and search bar binding to r... | pm-frontend | gap-82-2-deep-linking | gap-scan 2026-05-25 |
| medium | open | Implement ListingDetailView and FavoritesView in SwiftUI iOS app; wire to reality-server listings/{id} and ... | pm-frontend | gap-82-3-swiftui-home-search | gap-scan 2026-05-25 |
| medium | open | Implement InquiriesView and AccountView in SwiftUI iOS app; wire to reality-server inquiries and user profi... | pm-frontend | gap-82-4-swiftui-listing-detail | gap-scan 2026-05-25 |
| medium | open | Configure Android Release build: keystore setup, Google Services JSON, release Gradle variant, EAS build pr... | pm-frontend | gap-85-1-rn-env-config | gap-scan 2026-05-25 |
| medium | open | Configure iOS Release build: Bundle ID, provisioning profile, EAS credentials, App Store Connect integration | pm-frontend | gap-85-1-rn-env-config | gap-scan 2026-05-25 |
| medium | open | Define rollback/runbook + feature-flag gating for channel-sync workers (booking_channel.rs) so a bad OTA pu... | pm-devops | none | pm-analysis 2026-05-26 |
| medium | open | Slot post-merge follow-up issues #480-#487 (test gaps + minor security/UX follow-ups on messaging realtime,... | pm-frontend | none | pm-analysis 2026-05-25 |
| medium | open | Verify Epic 81 report-schedule pause/resume + execution-history work end-to-end as PRs #488/#489 land; conf... | pm-frontend | none | pm-analysis 2026-05-25 |
| medium | open | Establish PR-merge policy requiring a test file for every security-labelled fix (mirroring PR #493; PR #497... | pm-qa | pm-tech-lead | pm-analysis 2026-05-25 |
| medium | open | Define OAuth provider (10A) token/storage/rotation design before 10a-1 pickup to avoid rework | pm-tech-lead | none | pm-analysis 2026-05-23 |
| medium | open | Add ai.rs (3142 LOC), platform_admin.rs (2762), announcements.rs (2722) to the module-split backlog; add a ... | pm-tech-lead | none | pm-analysis 2026-05-25 |
| medium | open | Split churn-hot route modules (integrations.rs, organizations.rs, documents.rs) by surface (install/oauth/s... | pm-tech-lead | none | pm-analysis 2026-05-23 |
| low | open | Add GET /api/v1/platform-admin/onboarding-config returning step definitions; persist user_onboarding_progre... | pm-backend | none | gap-scan 2026-05-25 |
| low | open | Build step-by-step onboarding tour overlay in admin-web using onboarding config API | pm-frontend | gap-10b-6-onboarding-tour-api | gap-scan 2026-05-25 |
| low | open | Add contextual help tooltip/sidebar in admin-web pages linking to docs; use static markdown files or help_a... | pm-frontend | none | gap-scan 2026-05-25 |
| low | open | Verify document upload metadata integration complete for web + mobile after PR #447 (mobile) and existing b... | pm-backend | none | gap-scan 2026-05-24 |
| low | open | Audit SwiftUI Reality Portal project structure: verify NavigationCoordinator routing, auth guard implementa... | pm-frontend | none | gap-scan 2026-05-24 |
| low | open | Implement debounced search, infinite scroll pagination, CoreLocation integration + FilterSheet in HomeView/... | pm-frontend | none | gap-scan 2026-05-24 |
| low | open | Implement pinch-zoom PhotoGalleryView + cross-view favorites sync in ListingDetailView for SwiftUI Reality ... | pm-frontend | none | gap-scan 2026-05-24 |
| low | open | Implement push notification manager + Keychain secure token storage in iOS Reality Portal app (82-5 Inquiri... | pm-frontend | none | gap-scan 2026-05-24 |
| low | open | Wire HomeScreen and SearchScreen in KMP Android Compose UI to reality-server listings API via shared SDK | pm-frontend | none | gap-scan 2026-05-25 |
| low | open | Wire ListingDetailScreen and FavoritesScreen in KMP Android Compose UI to reality-server endpoints | pm-frontend | gap-82-6-kmp-android-home | gap-scan 2026-05-25 |
| low | open | Implement Airbnb OAuth token exchange route (/integrations/airbnb/connect) + realtime webhook handler for l... | pm-backend | none | gap-scan 2026-05-24 |
| low | open | Implement Booking.com OTA XML message parsing/generation, push notification endpoint, rate/availability pus... | pm-backend | none | gap-scan 2026-05-24 |
| high | done | Build OAuth client management admin UI in admin-web — backend client CRUD fully implemented (register/list/... | pm-frontend | none | gap-scan 2026-05-24 |
| high | done | Build user OAuth grants management page in ppt-web so users can revoke third-party app access — backend lis... | pm-frontend | none | gap-scan 2026-05-24 |
| high | in-progress | System announcements admin page in admin-web with CRUD, banner preview, severity badges | pm-frontend | none | gap-scan 2026-05-25 |
| high | done | Implement handler bodies for 10b-4 system announcements, 10b-5 support data access, 10b-6 user onboarding t... | pm-backend | None | gap-scan |
| high | done | Implement Epic 2B core notification delivery pipeline (stories 2b-1 through 2b-5): channel infrastructure, ... | pm-backend | none | gap-scan 2026-05-24 |
| high | in-progress | Wire read-receipt / acknowledgment flow for announcements in ppt-web and RN mobile; backend PUT /api/v1/ann... | pm-frontend | none | gap-scan 2026-05-25 |
| high | in-progress | Wire AnnouncementsPage web frontend to viewing/acknowledgment APIs (mark_read, acknowledge, get_acknowledgm... | pm-frontend | none | gap-scan 2026-05-24 |
| high | in-progress | Wire announcement comments/discussion web UI to backend list/create/delete comment routes — AnnouncementCom... | pm-frontend | none | gap-scan 2026-05-24 |
| high | in-progress | Wire pin/unpin announcement UI to backend pin_announcement route — frontend buildStatus planned; blocked on... | pm-frontend | none | gap-scan 2026-05-24 |
| high | done | Promote direct messaging screens (messages, message-thread, messages-new) from apiStatus: stub to integrate... | pm-frontend | None | gap-scan |
| high | done | Verify and wire neighbor listing frontend with privacy-aware filtering to backend list_neighbors + privacy_... | pm-frontend | none | gap-scan 2026-05-24 |
| high | in-progress | Wire @ppt/api-client into ppt-web and mobile apps: replace raw fetch calls with generated SDK hooks for aut... | pm-frontend | none | gap-scan 2026-05-25 |
| high | in-progress | Complete SSO / OAuth callback wiring in ppt-web: handle /auth/callback, store tokens, refresh flow; wire us... | pm-frontend | gap-79-1-api-client-core-wiring | gap-scan 2026-05-25 |
| high | in-progress | Wire login form, logout button, and session cleanup to AuthContext (79-2 Authentication Flow Implementation) | pm-frontend | None | gap-scan |
| high | in-progress | Implement POST /api/v1/documents/upload in api-server: multipart handler, S3 upload, document metadata inse... | pm-backend | none | gap-scan 2026-05-25 |
| high | done | Implement mobile (React Native) document upload UI with metadata against existing backend (7a-1 Document Up... | pm-frontend | None | gap-scan |
| high | done | Complete frontend API integration for documents permission-based access — promote screen apiStatus from par... | pm-backend | None | gap-scan |
| high | done | Implement mobile document permission/RLS UI — web access-scope filtering wired in PR #443; mobile DocumentP... | pm-frontend | none | gap-scan 2026-05-24 |
| high | done | Implement PDF.js client-side rendering for document preview — backend presigned URLs already wired (7a-4 Do... | pm-frontend | None | gap-scan |
| high | done | Implement mobile document sharing UI (user/role/public link/password) — backend share workflows already exi... | pm-frontend | None | gap-scan |
| high | done | Implement web document sharing UI (user/role/building targeting) in DocumentDetail — backend share workflow... | pm-frontend | none | gap-scan 2026-05-24 |
| high | in-progress | Implement mobile OS push notification registration and system channel binding for 8a-3 preference sync — We... | pm-backend | none | gap-scan 2026-05-24 |
| high | done | Implement background worker for push notification fanout: read push_tokens from DB, call FCM/APNs, store de... | pm-backend | gap-8a-3-mobile-push must be merged first | gap-scan 2026-05-25 |
| high | done | Verify TOTP MFA works end-to-end after gap-9-1-mfa-frontend-integration (PR #441) merge: QR scan → TOTP ver... | pm-security | none | gap-scan 2026-05-24 |
| high | done | Wire TwoFactorAuthPage to /api/v1/auth/mfa/* — add useMfa hooks (setup/verify/disable/status/backup-codes) ... | pm-security | None | gap-scan |
| high | done | Fix cross-tenant IDOR cluster in ai.rs: thread tenant_id into equipment_repo.update/delete/update_maintenan... | pm-backend | none | code-review-finding 2026-05-25 |
| high | done | Fix P1-05 SSRF: extract validate_external_url into a shared module and apply it before signatures.rs:628 cl... | pm-backend | none | pm-analysis 2026-05-24 |
| high | done | Sequence and land the Epic 6 announcement web UI draft PRs in dependency order: #474 (viewing/ack) → #475 (... | pm-frontend | none | pm-analysis 2026-05-25 |
| high | done | Get a review decision on PR #435 (security auth/security fixes) and merge to dev | pm-scrum-master | none | pm-analysis 2026-05-23 |
| high | done | Complete code review of Epic 8A (8a-1/8a-2/8a-3, all in review) and move to done | pm-scrum-master | none | pm-analysis 2026-05-23 |
| high | done | Decide build order: land Epic 2B notification infra before Epic 6 publish + 8A dispatch, or formally defer ... | pm-tech-lead | pm-scrum-master | pm-analysis 2026-05-23 |
| high | done | Remove dead duplicate AuthHandler/BuildingHandler modules so security fixes cannot diverge between handler/... | pm-tech-lead | none | pm-analysis 2026-05-23 |
| medium | in-progress | Add GET /api/v1/platform-admin/support-data endpoint returning tenant diagnostics (user counts, active sess... | pm-backend | none | gap-scan 2026-05-25 |
| medium | done | Build comment thread UI in ppt-web announcement detail view (list + post comment) | pm-frontend | gap-6-3-announcement-comments-api | gap-scan 2026-05-25 |
| medium | done | Add CRUD endpoints for document folders: POST/GET /api/v1/documents/folders with folder_id FK on documents | pm-backend | gap-7a-1-backend-multipart-upload | gap-scan 2026-05-25 |
| medium | done | Add GET /api/v1/documents/{id}/download returning presigned S3 URL; add GET /api/v1/documents/{id}/preview ... | pm-backend | gap-7a-1-backend-multipart-upload | gap-scan 2026-05-25 |
| medium | done | Add PATCH /api/v1/disputes/{id}/resolve and /api/v1/disputes/{id}/mediation-notes endpoints | pm-backend | gap-80-2-dispute-filing-api | gap-scan 2026-05-25 |
| medium | done | Show execution history table in ppt-web report schedule detail page | pm-frontend | gap-81-2-report-execution-history-api | gap-scan 2026-05-25 |
| medium | done | Implement Airbnb channel sync: POST /api/v1/integrations/airbnb/connect, webhook handler, availability sync... | pm-backend | none | gap-scan 2026-05-25 |
| medium | done | Implement Booking.com channel sync: connect OAuth, listing push, reservation pull, conflict detection | pm-backend | none | gap-scan 2026-05-25 |
| medium | done | Integrate e-signature provider (DocuSign/HelloSign) for lease agreements: POST /api/v1/leases/{id}/sign-req... | pm-backend | none | gap-scan 2026-05-25 |
| medium | done | Build e-signature request flow in ppt-web: select signers, send for signature, track status badge on lease ... | pm-frontend | gap-84-2-esignature-email-backend | gap-scan 2026-05-25 |
| medium | done | Implement e-signature email workflow: integrate DocuSign or lightweight PDF signing provider, signature req... | pm-backend | none | gap-scan 2026-05-24 |
| medium | done | Harden ProtectedRoute.tsx:117 to deny on missing role instead of skipping the role check, and populate user... | pm-frontend | none | pm-analysis 2026-05-24 |
| medium | done | Review and merge story 6.1 (announcement creation/targeting) | pm-scrum-master | none | pm-analysis 2026-05-23 |
| medium | done | Sequence Epic 2B notification infrastructure ahead of Epic 6 publish + 8A.2 dispatch | pm-scrum-master | pm-tech-lead | pm-analysis 2026-05-23 |
| low | in-progress | Implement deep-linking with URL schemes + navigation state preservation for SwiftUI Reality Portal app (82-... | pm-frontend | none | gap-scan 2026-05-24 |
| low | in-progress | Set up react-native-config + Metro bundler for ppt-mobile, create app.config.ts for Expo, add API_BASE_URL/... | pm-frontend | none | gap-scan 2026-05-24 |
| low | in-progress | Create iOS xcconfig files + schemes with dev/staging/prod variants, generate app icon sets with DEV/STG bad... | pm-frontend | none | gap-scan 2026-05-24 |
| low | done | Land the security-voice-device-idor plan (ready in .research/plans/) | pm-scrum-master | none | pm-analysis 2026-05-23 |
| low | done | Confirm WebSocket infra ownership for 8A.3 sync (ADR-008) is scheduled, not implicitly assumed | pm-tech-lead | pm-scrum-master | pm-analysis 2026-05-23 |
