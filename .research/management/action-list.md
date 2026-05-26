# Action list

_Generated 2026-05-25 from `action-list.json` (merged: pm-analysis rotation + gap-scan + code-review-finding). 64 open · 14 in-progress · 28 done._

## New this run (2026-05-25 — pm-qa + Scrum Master rotation)

| Priority | Action | Owner | Status |
|---|---|---|---|
| high | Sequence and land the Epic 6 announcement web UI draft PRs in dependency order: #474 (viewing/ack) → #475 (comments) → # | pm-frontend | done |
| medium | Slot post-merge follow-up issues #480-#487 (test gaps + minor security/UX follow-ups on messaging realtime, document sha | pm-frontend | open |
| medium | Add ai.rs (3142 LOC), platform_admin.rs (2762), announcements.rs (2722) to the module-split backlog; add a CI grep/clipp | pm-tech-lead | open |
| medium | Verify Epic 81 report-schedule pause/resume + execution-history work end-to-end as PRs #488/#489 land; confirm frontend  | pm-frontend | open |
| high | Create backend/servers/reality-server/tests/inquiry_idor_tests.rs covering PR #497: realtor B mark A inquiry read -> 404 | pm-qa | open |
| high | Verify respond_to_inquiry (POST /api/v1/inquiries/{id}/respond) enforces realtor ownership at repo layer; add cross-real | pm-qa | open |
| medium | Establish PR-merge policy requiring a test file for every security-labelled fix (mirroring PR #493; PR #497 shipped with | pm-qa | open |
| high | Implement POST /api/v1/documents/upload backend handler to unblock story 7a-1 (not promotable per PR #502) | pm-scrum-master | open |
| high | Wire App.tsx dispute routes: replace inline JSX stub with DisputeDetailPage and add /disputes/:disputeId/mediation route | pm-scrum-master | open |

## Open queue (by priority)

| Priority | Action | Owner | Source |
|---|---|---|---|
| high | Add OAuth 2.0 authorization server integration tests: full PKCE S256 authorization code flow, consent/revoke,  | pm-backend | gap-scan 2026-05-24 |
| high | Build admin health monitoring frontend page in admin-web wired to backend health dashboard + metric history +  | pm-frontend | gap-scan 2026-05-24 |
| high | Wire AnnouncementsPage and FaultsPage to @ppt/api-client query hooks — infra exists (queryKeys/axios intercept | pm-frontend | gap-scan 2026-05-24 |
| high | Build dedicated folder-tree UI page for document organization — backend supports 5-level hierarchy + circular  | pm-frontend | gap-scan |
| high | Implement mobile document download/preview with presigned URLs — web preview (react-pdf) shipped in PR #446; i | pm-frontend | gap-scan 2026-05-24 |
| high | Implement background worker for push notification fanout: read push_tokens from DB, call FCM/APNs, store deliv | pm-backend | gap-scan 2026-05-25 |
| high | Build WebSocket realtime sync infra for notification preferences and integrate with notification preference fl | pm-backend | gap-scan |
| high | Implement Epic 81 report-schedule endpoints the frontend already calls (/schedules/{id}/pause, /resume, /execu | pm-backend | pm-analysis 2026-05-24 |
| high | Create backend/servers/reality-server/tests/inquiry_idor_tests.rs covering PR #497: realtor B mark A inquiry r | pm-qa | pm-analysis 2026-05-25 |
| high | Verify respond_to_inquiry (POST /api/v1/inquiries/{id}/respond) enforces realtor ownership at repo layer; add  | pm-qa | pm-analysis 2026-05-25 |
| high | Implement POST /api/v1/documents/upload backend handler to unblock story 7a-1 (not promotable per PR #502) | pm-scrum-master | pm-analysis 2026-05-25 |
| high | Wire App.tsx dispute routes: replace inline JSX stub with DisputeDetailPage and add /disputes/:disputeId/media | pm-scrum-master | pm-analysis 2026-05-25 |
| high | Resolve the residual post-merge security findings in issues #438/#439 — P0-12 cookie scope, P1-04 Debug-format | pm-security | pm-analysis 2026-05-24 |
| medium | Implement system announcements handler bodies in platform_admin.rs — CRUD routes mounted but handlers return 5 | pm-backend | gap-scan 2026-05-24 |
| medium | Implement support data access handler bodies — search/get user/memberships/sessions/activity routes defined bu | pm-backend | gap-scan 2026-05-24 |
| medium | Build Support Data page in admin-web calling /api/v1/platform-admin/support-data; show tenant diagnostics tabl | pm-frontend | gap-scan 2026-05-25 |
| medium | Implement user onboarding tour handler bodies in onboarding.rs — get/start/complete/skip/reset routes mounted; | pm-backend | gap-scan 2026-05-24 |
| medium | Implement contextual help handler bodies in help.rs — articles/FAQ/tooltips with search+context routes mounted | pm-backend | gap-scan 2026-05-24 |
| medium | Add POST/GET /api/v1/announcements/{id}/comments endpoints and migration for announcement_comments table | pm-backend | gap-scan 2026-05-25 |
| medium | Add PATCH /api/v1/announcements/{id}/pin endpoint and is_pinned column in migration; return pinned announcemen | pm-backend | gap-scan 2026-05-25 |
| medium | Show pinned announcements in sticky band at top of announcements list in ppt-web and mobile | pm-frontend | gap-scan 2026-05-25 |
| medium | Add CRUD endpoints for document folders: POST/GET /api/v1/documents/folders with folder_id FK on documents | pm-backend | gap-scan 2026-05-25 |
| medium | Build folder browser UI in ppt-web document library: create folder, move documents, breadcrumb navigation | pm-frontend | gap-scan 2026-05-25 |
| medium | Add GET /api/v1/documents/{id}/download returning presigned S3 URL; add GET /api/v1/documents/{id}/preview for | pm-backend | gap-scan 2026-05-25 |
| medium | Add download button and PDF/image inline preview modal in ppt-web document list | pm-frontend | gap-scan 2026-05-25 |
| medium | Add POST /api/v1/disputes endpoint with migration for disputes table; implement dispute state machine (open→in | pm-backend | gap-scan 2026-05-25 |
| medium | Build dispute filing form in ppt-web with reason selector, description, and evidence upload | pm-frontend | gap-scan 2026-05-25 |
| medium | Verify dispute filing flow (FileDisputePage + EvidenceUploader) meets all acceptance criteria; update story st | pm-frontend | gap-scan 2026-05-24 |
| medium | Add PATCH /api/v1/disputes/{id}/resolve and /api/v1/disputes/{id}/mediation-notes endpoints | pm-backend | gap-scan 2026-05-25 |
| medium | Build mediation workspace in ppt-web: dispute timeline, resolution form, manager/tenant chat thread | pm-frontend | gap-scan 2026-05-25 |
| medium | Verify mediation resolution flow (MediationPage/DisputeDetailPage) meets all ACs including escalation paths; p | pm-frontend | gap-scan 2026-05-24 |
| medium | Add PUT /api/v1/reports/schedules/{id} to update cron expression, recipients, and enabled flag | pm-backend | gap-scan 2026-05-25 |
| medium | Build report schedule editor modal in ppt-web: cron picker, recipient list, enabled toggle | pm-frontend | gap-scan 2026-05-25 |
| medium | Verify EditScheduleModal pause/resume works end-to-end against new backend endpoints from PR #448; confirm fro | pm-frontend | gap-scan 2026-05-24 |
| medium | Verify ExecutionHistory UI works end-to-end against new backend execution endpoints from PR #448; validate dow | pm-frontend | gap-scan 2026-05-24 |
| medium | Add GET /api/v1/reports/schedules/{id}/executions returning paginated execution log with status and download l | pm-backend | gap-scan 2026-05-25 |
| medium | Show execution history table in ppt-web report schedule detail page | pm-frontend | gap-scan 2026-05-25 |
| medium | Implement HomeView and SearchView screens in SwiftUI iOS app with listing cards and search bar binding to real | pm-frontend | gap-scan 2026-05-25 |
| medium | Implement ListingDetailView and FavoritesView in SwiftUI iOS app; wire to reality-server listings/{id} and fav | pm-frontend | gap-scan 2026-05-25 |
| medium | Implement InquiriesView and AccountView in SwiftUI iOS app; wire to reality-server inquiries and user profile  | pm-frontend | gap-scan 2026-05-25 |
| medium | Implement Airbnb channel sync: POST /api/v1/integrations/airbnb/connect, webhook handler, availability sync jo | pm-backend | gap-scan 2026-05-25 |
| medium | Implement Booking.com channel sync: connect OAuth, listing push, reservation pull, conflict detection | pm-backend | gap-scan 2026-05-25 |
| medium | Build e-signature request flow in ppt-web: select signers, send for signature, track status badge on lease car | pm-frontend | gap-scan 2026-05-25 |
| medium | Implement e-signature email workflow: integrate DocuSign or lightweight PDF signing provider, signature reques | pm-backend | gap-scan 2026-05-24 |
| medium | Configure Android Release build: keystore setup, Google Services JSON, release Gradle variant, EAS build profi | pm-frontend | gap-scan 2026-05-25 |
| medium | Configure iOS Release build: Bundle ID, provisioning profile, EAS credentials, App Store Connect integration | pm-frontend | gap-scan 2026-05-25 |
| medium | Slot post-merge follow-up issues #480-#487 (test gaps + minor security/UX follow-ups on messaging realtime, do | pm-frontend | pm-analysis 2026-05-25 |
| medium | Verify Epic 81 report-schedule pause/resume + execution-history work end-to-end as PRs #488/#489 land; confirm | pm-frontend | pm-analysis 2026-05-25 |
| medium | Establish PR-merge policy requiring a test file for every security-labelled fix (mirroring PR #493; PR #497 sh | pm-qa | pm-analysis 2026-05-25 |
| medium | Define OAuth provider (10A) token/storage/rotation design before 10a-1 pickup to avoid rework | pm-tech-lead | pm-analysis 2026-05-23 |
| medium | Add ai.rs (3142 LOC), platform_admin.rs (2762), announcements.rs (2722) to the module-split backlog; add a CI  | pm-tech-lead | pm-analysis 2026-05-25 |
| medium | Split churn-hot route modules (integrations.rs, organizations.rs, documents.rs) by surface (install/oauth/sync | pm-tech-lead | pm-analysis 2026-05-23 |
| low | Add GET /api/v1/platform-admin/onboarding-config returning step definitions; persist user_onboarding_progress  | pm-backend | gap-scan 2026-05-25 |
| low | Build step-by-step onboarding tour overlay in admin-web using onboarding config API | pm-frontend | gap-scan 2026-05-25 |
| low | Add contextual help tooltip/sidebar in admin-web pages linking to docs; use static markdown files or help_arti | pm-frontend | gap-scan 2026-05-25 |
| low | Verify document upload metadata integration complete for web + mobile after PR #447 (mobile) and existing back | pm-backend | gap-scan 2026-05-24 |
| low | Audit SwiftUI Reality Portal project structure: verify NavigationCoordinator routing, auth guard implementatio | pm-frontend | gap-scan 2026-05-24 |
| low | Implement debounced search, infinite scroll pagination, CoreLocation integration + FilterSheet in HomeView/Sea | pm-frontend | gap-scan 2026-05-24 |
| low | Implement pinch-zoom PhotoGalleryView + cross-view favorites sync in ListingDetailView for SwiftUI Reality Por | pm-frontend | gap-scan 2026-05-24 |
| low | Implement push notification manager + Keychain secure token storage in iOS Reality Portal app (82-5 Inquiries  | pm-frontend | gap-scan 2026-05-24 |
| low | Wire HomeScreen and SearchScreen in KMP Android Compose UI to reality-server listings API via shared SDK | pm-frontend | gap-scan 2026-05-25 |
| low | Wire ListingDetailScreen and FavoritesScreen in KMP Android Compose UI to reality-server endpoints | pm-frontend | gap-scan 2026-05-25 |
| low | Implement Airbnb OAuth token exchange route (/integrations/airbnb/connect) + realtime webhook handler for list | pm-backend | gap-scan 2026-05-24 |
| low | Implement Booking.com OTA XML message parsing/generation, push notification endpoint, rate/availability push e | pm-backend | gap-scan 2026-05-24 |

