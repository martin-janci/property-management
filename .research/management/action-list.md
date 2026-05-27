# Action list

_Generated 2026-05-27 from `action-list.json` (merged: pm-analysis rotation + gap-scan + code-review-finding). 100 open · 14 in-progress · 29 done._

## New this run (2026-05-27 — pm-devops + Scrum Master rotation)

| Priority | Action | Owner | Status |
|---|---|---|---|
| high | Land the two blocked EAS mobile CI fixes together (gap-85-2-android-ci-fix + gap-85-2-ios-ci-fix): downgrade non-existen | pm-devops | open |
| medium | Confirm security-test-gate.yml actually fails PRs labelled security that ship without a test file (the policy QA flagged | pm-devops | open |
| medium | App.tsx remains the top frontend churn hotspot (route wiring lands there every sprint); enable a merge queue / auto-reba | pm-devops | open |
| high | Promote the two overnight-approved draft PRs out of draft and merge to dev: #566 (gap-84-2 esignature UI fixes) and #567 | pm-scrum-master | open |
| medium | Build mediation workspace in ppt-web: dispute timeline, resolution form, manager/tenant chat thread | pm-frontend | done |

## Open queue (by priority)

| Priority | Action | Owner | Status |
|---|---|---|---|
| high | Wire login form, logout button, and session cleanup to AuthContext (79-2 Authentication Flow Implementation) | pm-frontend | in-progress |
| high | Wire AnnouncementsPage web frontend to viewing/acknowledgment APIs (mark_read, acknowledge, get_acknowledgments) — apiSt | pm-frontend | in-progress |
| high | Wire announcement comments/discussion web UI to backend list/create/delete comment routes — AnnouncementComments fronten | pm-frontend | in-progress |
| high | Wire pin/unpin announcement UI to backend pin_announcement route — frontend buildStatus planned; blocked on Epic 2B (6-4 | pm-frontend | in-progress |
| high | Implement mobile OS push notification registration and system channel binding for 8a-3 preference sync — WebSocket defer | pm-backend | in-progress |
| high | System announcements admin page in admin-web with CRUD, banner preview, severity badges | pm-frontend | in-progress |
| high | Wire read-receipt / acknowledgment flow for announcements in ppt-web and RN mobile; backend PUT /api/v1/announcements/{i | pm-frontend | in-progress |
| high | Wire @ppt/api-client into ppt-web and mobile apps: replace raw fetch calls with generated SDK hooks for auth, announceme | pm-frontend | in-progress |
| high | Complete SSO / OAuth callback wiring in ppt-web: handle /auth/callback, store tokens, refresh flow; wire useAuth hook to | pm-frontend | in-progress |
| high | Implement POST /api/v1/documents/upload in api-server: multipart handler, S3 upload, document metadata insert, return Do | pm-backend | in-progress |
| high | Build WebSocket realtime sync infra for notification preferences and integrate with notification preference flow (8a-3 N | pm-backend | open |
| high | Build dedicated folder-tree UI page for document organization — backend supports 5-level hierarchy + circular check (7a- | pm-frontend | open |
| high | Resolve the residual post-merge security findings in issues #438/#439 — P0-12 cookie scope, P1-04 Debug-format audit-has | pm-security | open |
| high | Implement Epic 81 report-schedule endpoints the frontend already calls (/schedules/{id}/pause, /resume, /executions) — c | pm-backend | open |
| high | Implement mobile document download/preview with presigned URLs — web preview (react-pdf) shipped in PR #446; implement p | pm-frontend | open |
| high | Wire AnnouncementsPage and FaultsPage to @ppt/api-client query hooks — infra exists (queryKeys/axios interceptors/51 hoo | pm-frontend | open |
| high | Add OAuth 2.0 authorization server integration tests: full PKCE S256 authorization code flow, consent/revoke, token refr | pm-backend | open |
| high | Build admin health monitoring frontend page in admin-web wired to backend health dashboard + metric history + alert rout | pm-frontend | open |
| high | Implement background worker for push notification fanout: read push_tokens from DB, call FCM/APNs, store delivery receip | pm-backend | open |
| high | Create backend/servers/reality-server/tests/inquiry_idor_tests.rs covering PR #497: realtor B mark A inquiry read -> 404 | pm-qa | open |
| high | Verify respond_to_inquiry (POST /api/v1/inquiries/{id}/respond) enforces realtor ownership at repo layer; add cross-real | pm-qa | open |
| high | Implement POST /api/v1/documents/upload backend handler to unblock story 7a-1 (not promotable per PR #502) | pm-scrum-master | open |
| high | Wire App.tsx dispute routes: replace inline JSX stub with DisputeDetailPage and add /disputes/:disputeId/mediation route | pm-scrum-master | open |
| high | Fix P0-12 cookie scope from post-merge security findings on PR#435: ensure session cookies use SameSite=Strict and Secur | pm-security | open |
| high | Add inquiry IDOR regression tests in backend/servers/reality-server/tests/inquiry_idor_tests.rs: realtor B cannot mark r | pm-qa | open |
| high | Wire read-receipt and acknowledgment flow for announcements in ppt-web and RN mobile; PUT /api/v1/announcements/{id}/rea | pm-frontend | open |
| high | Wire LoginPage to AuthContext.login(), handle /auth/callback route and store JWT tokens via tokenProvider, wire logout b | pm-frontend | open |
| high | Land the two blocked EAS mobile CI fixes together (gap-85-2-android-ci-fix + gap-85-2-ios-ci-fix): downgrade non-existen | pm-devops | open |
| high | Promote the two overnight-approved draft PRs out of draft and merge to dev: #566 (gap-84-2 esignature UI fixes) and #567 | pm-scrum-master | open |
| medium | Add GET /api/v1/platform-admin/support-data endpoint returning tenant diagnostics (user counts, active sessions, fault s | pm-backend | in-progress |
| medium | Slot post-merge follow-up issues #480-#487 (test gaps + minor security/UX follow-ups on messaging realtime, document sha | pm-frontend | open |
| medium | Add ai.rs (3142 LOC), platform_admin.rs (2762), announcements.rs (2722) to the module-split backlog; add a CI grep/clipp | pm-tech-lead | open |
| medium | Verify Epic 81 report-schedule pause/resume + execution-history work end-to-end as PRs #488/#489 land; confirm frontend  | pm-frontend | open |
| medium | Split churn-hot route modules (integrations.rs, organizations.rs, documents.rs) by surface (install/oauth/sync/webhook) | pm-tech-lead | open |
| medium | Define OAuth provider (10A) token/storage/rotation design before 10a-1 pickup to avoid rework | pm-tech-lead | open |
| medium | Implement system announcements handler bodies in platform_admin.rs — CRUD routes mounted but handlers return 501; add SQ | pm-backend | open |
| medium | Implement support data access handler bodies — search/get user/memberships/sessions/activity routes defined but stubs; a | pm-backend | open |
| medium | Implement user onboarding tour handler bodies in onboarding.rs — get/start/complete/skip/reset routes mounted; handlers  | pm-backend | open |
| medium | Implement contextual help handler bodies in help.rs — articles/FAQ/tooltips with search+context routes mounted; handlers | pm-backend | open |
| medium | Verify dispute filing flow (FileDisputePage + EvidenceUploader) meets all acceptance criteria; update story status from  | pm-frontend | open |
| medium | Verify mediation resolution flow (MediationPage/DisputeDetailPage) meets all ACs including escalation paths; promote sto | pm-frontend | open |
| medium | Verify EditScheduleModal pause/resume works end-to-end against new backend endpoints from PR #448; confirm frontend API  | pm-frontend | open |
| medium | Verify ExecutionHistory UI works end-to-end against new backend execution endpoints from PR #448; validate download URL  | pm-frontend | open |
| medium | Implement e-signature email workflow: integrate DocuSign or lightweight PDF signing provider, signature request email te | pm-backend | open |
| medium | Add POST/GET /api/v1/announcements/{id}/comments endpoints and migration for announcement_comments table | pm-backend | open |
| medium | Add PATCH /api/v1/announcements/{id}/pin endpoint and is_pinned column in migration; return pinned announcements sorted  | pm-backend | open |
| medium | Show pinned announcements in sticky band at top of announcements list in ppt-web and mobile | pm-frontend | open |
| medium | Add CRUD endpoints for document folders: POST/GET /api/v1/documents/folders with folder_id FK on documents | pm-backend | open |
| medium | Build folder browser UI in ppt-web document library: create folder, move documents, breadcrumb navigation | pm-frontend | open |
| medium | Add GET /api/v1/documents/{id}/download returning presigned S3 URL; add GET /api/v1/documents/{id}/preview for inline vi | pm-backend | open |
| medium | Add download button and PDF/image inline preview modal in ppt-web document list | pm-frontend | open |
| medium | Add POST /api/v1/disputes endpoint with migration for disputes table; implement dispute state machine (open→in-review→re | pm-backend | open |
| medium | Build dispute filing form in ppt-web with reason selector, description, and evidence upload | pm-frontend | open |
| medium | Add PATCH /api/v1/disputes/{id}/resolve and /api/v1/disputes/{id}/mediation-notes endpoints | pm-backend | open |
| medium | Add PUT /api/v1/reports/schedules/{id} to update cron expression, recipients, and enabled flag | pm-backend | open |
| medium | Build report schedule editor modal in ppt-web: cron picker, recipient list, enabled toggle | pm-frontend | open |
| medium | Add GET /api/v1/reports/schedules/{id}/executions returning paginated execution log with status and download link | pm-backend | open |
| medium | Show execution history table in ppt-web report schedule detail page | pm-frontend | open |
| medium | Implement HomeView and SearchView screens in SwiftUI iOS app with listing cards and search bar binding to reality-server | pm-frontend | open |
| medium | Implement ListingDetailView and FavoritesView in SwiftUI iOS app; wire to reality-server listings/{id} and favorites end | pm-frontend | open |
| medium | Implement InquiriesView and AccountView in SwiftUI iOS app; wire to reality-server inquiries and user profile endpoints | pm-frontend | open |
| medium | Implement Airbnb channel sync: POST /api/v1/integrations/airbnb/connect, webhook handler, availability sync job | pm-backend | open |
| medium | Implement Booking.com channel sync: connect OAuth, listing push, reservation pull, conflict detection | pm-backend | open |
| medium | Build e-signature request flow in ppt-web: select signers, send for signature, track status badge on lease card | pm-frontend | open |
| medium | Configure Android Release build: keystore setup, Google Services JSON, release Gradle variant, EAS build profile | pm-frontend | open |
| medium | Configure iOS Release build: Bundle ID, provisioning profile, EAS credentials, App Store Connect integration | pm-frontend | open |
| medium | Build Support Data page in admin-web calling /api/v1/platform-admin/support-data; show tenant diagnostics table | pm-frontend | open |
| medium | Establish PR-merge policy requiring a test file for every security-labelled fix (mirroring PR #493; PR #497 shipped with | pm-qa | open |
| medium | Refactor admin health UI (PR#471) health API calls to use @ppt/api-client factory fetchJson() so 401 mfa_required respon | pm-frontend | open |
| medium | Fix 6 reviewer issues from PR#513 esignature UI: (1) remove duplicate LeaseSignatureStatus type, import from api-client; | pm-frontend | open |
| medium | Fix GH Actions version pins in eas-build-android.yml: checkout@v6/setup-node@v6/pnpm-action-setup@v6 do not exist; downg | pm-frontend | open |
| medium | Fix GH Actions version pins in eas-build-ios.yml: checkout@v6/setup-node@v6/pnpm-action-setup@v6 do not exist; downgrade | pm-frontend | open |
| medium | Fix two must-fix bugs in PR#537 iOS search enhancements: (1) nearMeChip alert binding uses no-op setter making location- | pm-frontend | open |
| medium | Fix blocking issues in PR#530 dispute filing form: (1) add react-hook-form/zod/hookform-resolvers to ppt-web package.jso | pm-frontend | open |
| medium | Add RequireCapability extractor to PUT /api/v1/reports/schedules/{id} so only authorized users can mutate schedules; add | pm-backend | open |
| medium | Implement platform_admin.rs system announcement SQLx handler bodies (create/list/get/update/delete system_announcements  | pm-backend | open |
| medium | Re-implement GET /api/v1/platform-admin/support-data and GET /api/v1/platform-admin/support/users/{id} with full SQLx qu | pm-backend | open |
| medium | Re-implement GET /api/v1/reports/schedules/{id}/executions returning paginated execution log with status and download li | pm-backend | open |
| medium | Add integration tests for POST /api/v1/documents/upload handler: MIME type validation (allowed/denied), file size limit  | pm-qa | open |
| medium | Add integration tests for FCM/APNs push fanout worker: successful FCM delivery receipt stored, APNs-only path (no fcm_at | pm-qa | open |
| medium | Add OAuth introspection + refresh-rotation security tests: revoked tokens rejected, family-reuse detection blocks replay | pm-qa | open |
| medium | Fix P1-04 Debug-format audit-hash domain leak from post-merge findings on PR#435: audit trail records must not include D | pm-security | open |
| medium | Add WebSocket push integration tests: authenticated WS upgrade, JWT expiry enforcement (15min), idle-timeout behaviour,  | pm-qa | open |
| medium | Build Support Data admin page in admin-web wired to GET /api/v1/platform-admin/support-data; show tenant diagnostics tab | pm-frontend | open |
| medium | Implement ListingDetailView and FavoritesView in SwiftUI Reality Portal with pinch-zoom photo gallery and cross-view fav | pm-frontend | open |
| medium | Implement InquiriesView and AccountView in SwiftUI Reality Portal; wire to reality-server inquiries and user profile end | pm-frontend | open |
| medium | Add E2E integration test for ppt-web OAuth callback flow: /auth/callback stores tokens, redirects to /dashboard, logout  | pm-qa | open |
| medium | Add backend integration tests for POST /api/v1/documents/upload: multipart form parsing, S3 mock upload, document record | pm-qa | open |
| medium | Confirm security-test-gate.yml actually fails PRs labelled security that ship without a test file (the policy QA flagged | pm-devops | open |
| medium | App.tsx remains the top frontend churn hotspot (route wiring lands there every sprint); enable a merge queue / auto-reba | pm-devops | open |
| low | Set up react-native-config + Metro bundler for ppt-mobile, create app.config.ts for Expo, add API_BASE_URL/ENVIRONMENT k | pm-frontend | in-progress |
| low | Create iOS xcconfig files + schemes with dev/staging/prod variants, generate app icon sets with DEV/STG badges, create b | pm-frontend | in-progress |
| low | Implement deep-linking with URL schemes + navigation state preservation for SwiftUI Reality Portal app (82-2 Navigation  | pm-frontend | in-progress |
| low | Verify document upload metadata integration complete for web + mobile after PR #447 (mobile) and existing backend; confi | pm-backend | open |
| low | Audit SwiftUI Reality Portal project structure: verify NavigationCoordinator routing, auth guard implementation, documen | pm-frontend | open |
| low | Implement debounced search, infinite scroll pagination, CoreLocation integration + FilterSheet in HomeView/SearchView fo | pm-frontend | open |
| low | Implement pinch-zoom PhotoGalleryView + cross-view favorites sync in ListingDetailView for SwiftUI Reality Portal (82-4  | pm-frontend | open |
| low | Implement push notification manager + Keychain secure token storage in iOS Reality Portal app (82-5 Inquiries and Accoun | pm-frontend | open |
| low | Implement Airbnb OAuth token exchange route (/integrations/airbnb/connect) + realtime webhook handler for listing/reserv | pm-backend | open |
| low | Implement Booking.com OTA XML message parsing/generation, push notification endpoint, rate/availability push endpoint (8 | pm-backend | open |
| low | Add GET /api/v1/platform-admin/onboarding-config returning step definitions; persist user_onboarding_progress table | pm-backend | open |
| low | Build step-by-step onboarding tour overlay in admin-web using onboarding config API | pm-frontend | open |
| low | Add contextual help tooltip/sidebar in admin-web pages linking to docs; use static markdown files or help_articles table | pm-frontend | open |
| low | Wire HomeScreen and SearchScreen in KMP Android Compose UI to reality-server listings API via shared SDK | pm-frontend | open |
| low | Wire ListingDetailScreen and FavoritesScreen in KMP Android Compose UI to reality-server endpoints | pm-frontend | open |
| low | Re-implement onboarding tour SQLx handlers in onboarding.rs: GET/POST/PATCH routes with user_onboarding_progress migrati | pm-backend | open |
| low | Implement contextual help handler bodies with SQLx for help.rs: articles/FAQ search routes; add help_articles and faq_en | pm-backend | open |
| low | Wire HomeScreen and SearchScreen in KMP Android Compose UI to reality-server listings API via shared SDK; retry of faile | pm-frontend | open |
| low | Wire ListingDetailScreen and FavoritesScreen in KMP Android Compose UI to reality-server listings/{id} and favorites end | pm-frontend | open |
| low | Wire contextual help tooltips in admin-web to GET /api/v1/help/articles dynamic content instead of static markdown; requ | pm-frontend | open |
| low | Build Airbnb integration management page in admin-web: OAuth connect button, connection status, disconnect, listing sync | pm-frontend | open |
| low | Build Booking.com integration management page in admin-web: connect OAuth, availability push status, OTA sync log; wire  | pm-frontend | open |
| low | Create docs/screens/ppt/system-announcements.md screen-map for system announcements admin route (CLAUDE.md rule B — requ | pm-frontend | open |
| low | Add NSUserNotificationUsageDescription to iOS Reality Portal Info.plist and UNUserNotificationCenterDelegate wiring (req | pm-frontend | open |

## Done (resolved)

| Action | Owner | Resolved |
|---|---|---|
| Fix cross-tenant IDOR cluster in ai.rs: thread tenant_id into equipment_repo.update/delete/update_maintenance and scope  | pm-backend | PR #493 merged 2026-05-25 (with regression test) |
| Sequence and land the Epic 6 announcement web UI draft PRs in dependency order: #474 (viewing/ack) → #475 (comments) → # | pm-frontend | PR #492 merged 2026-05-25 |
| Get a review decision on PR #435 (security auth/security fixes) and merge to dev | pm-scrum-master | done |
| Complete code review of Epic 8A (8a-1/8a-2/8a-3, all in review) and move to done | pm-scrum-master | done |
| Review and merge story 6.1 (announcement creation/targeting) | pm-scrum-master | done |
| Sequence Epic 2B notification infrastructure ahead of Epic 6 publish + 8A.2 dispatch | pm-scrum-master | 2026-05-24 — sprint-status.yaml sequenced, DEC-001 augmented with unblock-trigge |
| Land the security-voice-device-idor plan (ready in .research/plans/) | pm-scrum-master | 2026-05-25 — voice device IDOR fix PR #461 merged |
| Decide build order: land Epic 2B notification infra before Epic 6 publish + 8A dispatch, or formally defer those slices | pm-tech-lead | 2026-05-24 — DEC-001 merged via PR #442 |
| Remove dead duplicate AuthHandler/BuildingHandler modules so security fixes cannot diverge between handler/route copies | pm-tech-lead | done |
| Confirm WebSocket infra ownership for 8A.3 sync (ADR-008) is scheduled, not implicitly assumed | pm-tech-lead | 2026-05-25 — WS infra ownership confirmed: delivered as Epic 2B story 2B-C.1 (PR |
| Wire TwoFactorAuthPage to /api/v1/auth/mfa/* — add useMfa hooks (setup/verify/disable/status/backup-codes) to @ppt/api-c | pm-security | 2026-05-25 — PR #441 merged (MFA frontend integration); e2e coverage PR #473 mer |
| Complete frontend API integration for documents permission-based access — promote screen apiStatus from partial to compl | pm-backend | 2026-05-25 — web access-scope UI PR #443 + mobile permission UI PRs #462/#465 me |
| Implement mobile (React Native) document upload UI with metadata against existing backend (7a-1 Document Upload with Met | pm-frontend | 2026-05-25 — mobile document upload UI PR #447 merged |
| Implement mobile document sharing UI (user/role/public link/password) — backend share workflows already exist (7a-5 Docu | pm-frontend | 2026-05-25 — mobile DocumentShareSheet PR #445 merged |
| Implement PDF.js client-side rendering for document preview — backend presigned URLs already wired (7a-4 Document Downlo | pm-frontend | done |
| Promote direct messaging screens (messages, message-thread, messages-new) from apiStatus: stub to integrated (6-5 Direct | pm-frontend | 2026-05-25 — messaging screens wired PR #449 + WebSocket realtime sync PR #472 m |
| Implement handler bodies for 10b-4 system announcements, 10b-5 support data access, 10b-6 user onboarding tour, 10b-7 co | pm-backend | Already implemented in feat(epic-10b) commit cf533ae4; all four handler groups h |
| Fix P1-05 SSRF: extract validate_external_url into a shared module and apply it before signatures.rs:628 client.get(sign | pm-backend | 2026-05-25 — PR #450 merged (SSRF outbound URL validation) |
| Harden ProtectedRoute.tsx:117 to deny on missing role instead of skipping the role check, and populate user.role in Auth | pm-frontend | 2026-05-25 — PR #459 merged (ProtectedRoute fail-open fix) |
| Implement Epic 2B core notification delivery pipeline (stories 2b-1 through 2b-5): channel infrastructure, preference ro | pm-backend | 2026-05-25 — Epic 2B notification pipeline PR #463 merged; WebSocket realtime sy |
| Verify and wire neighbor listing frontend with privacy-aware filtering to backend list_neighbors + privacy_settings rout | pm-frontend | 2026-05-25 — neighbor listing UI with privacy-aware filtering PR #464 merged |
| Implement web document sharing UI (user/role/building targeting) in DocumentDetail — backend share workflows complete; m | pm-frontend | 2026-05-25 — web document sharing UI PRs #451/#467 merged |
| Implement mobile document permission/RLS UI — web access-scope filtering wired in PR #443; mobile DocumentPermissionsScr | pm-frontend | 2026-05-25 — mobile document permission/RLS UI PRs #462/#465 merged |
| Build OAuth client management admin UI in admin-web — backend client CRUD fully implemented (register/list/update/revoke | pm-frontend | 2026-05-25 — OAuth provider client-management admin UI PRs #468/#469/#471 merged |
| Build user OAuth grants management page in ppt-web so users can revoke third-party app access — backend list_user_grants | pm-frontend | 2026-05-25 — user OAuth grants management UI PRs #468/#469/#471 merged |
| Verify TOTP MFA works end-to-end after gap-9-1-mfa-frontend-integration (PR #441) merge: QR scan → TOTP verify → login 2 | pm-security | 2026-05-25 — MFA e2e test coverage PR #473 merged |
| Build comment thread UI in ppt-web announcement detail view (list + post comment) | pm-frontend | PR #475 merged 2026-05-25 |
| Build mediation workspace in ppt-web: dispute timeline, resolution form, manager/tenant chat thread | pm-frontend | PR #555 merged 2026-05-27 (mediation workspace UI + App.tsx route wiring + dispu |
| Integrate e-signature provider (DocuSign/HelloSign) for lease agreements: POST /api/v1/leases/{id}/sign-request, webhook | pm-backend | PR #495 merged 2026-05-25 (HMAC provider) |
