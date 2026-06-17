---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - _bmad-output/prd.md
  - _bmad-output/architecture.md
  - _bmad-output/epics-006.md
  - docs/ARCHITECTURE_REVIEW.md
  - _bmad-output/implementation-artifacts/gap-analysis-remediation.md
workflowType: 'epics-and-stories'
lastStep: 4
status: 'complete'
project_name: 'Property Management System (PPT) & Reality Portal'
user_name: 'Martin Janci'
date: '2025-12-31'
continues_from: 'epics-006.md'
phase_range: '22, 23, 24'
epic_range: '79-85'
---

# Property Management System (PPT) & Reality Portal - Epic Breakdown (Part 7)

## Overview

This document continues from `epics-006.md` and addresses **critical gaps identified in the comprehensive code review**. These epics focus on security hardening, test infrastructure, frontend completion, mobile app fixes, and silent failure remediation.

**Continuation from:** `epics-006.md` (Epics 66-78, Phases 19-21)

**Source:** Multi-angle code review findings from December 2025

---

## Epic List

### Phase 22: Security & Quality Foundations

#### Epic 79: Security Hardening
**Goal:** Address critical and high-priority security vulnerabilities identified in the code review, ensuring production-ready authentication and authorization.

**Target Apps:** api-server, reality-server, api-core
**Estimate:** 5 stories, ~2 weeks
**Dependencies:** None (foundational)
**Priority:** P0 - CRITICAL

---

##### Story 79.1: Tenant Membership Validation

As a **security engineer**,
I want to **validate user membership in tenants before granting access**,
So that **cross-tenant data access is prevented**.

**Acceptance Criteria:**

**Given** a user requests access to a tenant's data
**When** the TenantExtractor processes the request
**Then**:
  - User ID is extracted from validated JWT (not placeholders)
  - Tenant membership is verified against database
  - Role is validated against actual permissions
  - Request is rejected if user lacks membership
**And** unauthorized access attempts are logged

**Technical Notes:**
- Modify `api-core/src/extractors/tenant.rs`
- Add database lookup for tenant membership
- Remove `Uuid::nil()` fallback
- Add `OrganizationMemberRepository` query
- Log unauthorized access at WARN level

**Files to Modify:**
- `backend/crates/api-core/src/extractors/tenant.rs`
- `backend/crates/db/src/repositories/organization.rs`

---

##### Story 79.2: JWT Secret Production Enforcement

As a **security engineer**,
I want to **ensure JWT secrets are never defaulted in production**,
So that **authentication cannot be bypassed**.

**Acceptance Criteria:**

**Given** the application starts
**When** JWT_SECRET environment variable is not set
**Then**:
  - Application panics if RUST_ENV != "development"
  - Clear error message indicates required configuration
  - Development mode requires explicit opt-in
**And** minimum secret length is enforced (64+ chars recommended)

**Technical Notes:**
- Already partially fixed in main.rs
- Add secret strength validation
- Add startup health check for security config
- Document required environment variables

**Files to Modify:**
- `backend/servers/api-server/src/main.rs`
- `backend/servers/reality-server/src/main.rs`

---

##### Story 79.3: Authorization Middleware Layer

As a **security engineer**,
I want to **apply authorization middleware to all protected routes**,
So that **permission checks are consistent and mandatory**.

**Acceptance Criteria:**

**Given** a protected route is accessed
**When** the request is processed
**Then**:
  - AuthUser extractor validates JWT
  - TenantExtractor validates tenant access
  - Role-based permissions are checked
  - Unauthorized requests return 403
**And** all routes have explicit permission requirements

**Technical Notes:**
- Create `RequirePermission` middleware
- Define permission enum for all operations
- Apply to route groups (buildings, faults, voting, etc.)
- Log permission denials

**Files to Modify:**
- `backend/crates/api-core/src/middleware/` (new)
- `backend/servers/api-server/src/main.rs`

---

##### Story 79.4: TOTP Secret Encryption Enforcement

As a **security engineer**,
I want to **require encryption for all TOTP secrets**,
So that **MFA cannot be bypassed if database is compromised**.

**Acceptance Criteria:**

**Given** MFA is enabled for a user
**When** TOTP secrets are stored
**Then**:
  - TOTP_ENCRYPTION_KEY is required
  - Legacy unencrypted secrets are migrated
  - Application fails to start without encryption key
**And** secrets are encrypted with AES-256-GCM

**Technical Notes:**
- Add startup validation for TOTP_ENCRYPTION_KEY
- Create migration to re-encrypt existing secrets
- Remove fallback for unencrypted secrets
- Add key rotation support

**Files to Modify:**
- `backend/servers/api-server/src/services/totp.rs`
- `backend/crates/db/src/repositories/user.rs`

---

##### Story 79.5: SQL Query Builder Refactoring

As a **security engineer**,
I want to **replace dynamic SQL string concatenation with QueryBuilder**,
So that **SQL injection is prevented**.

**Acceptance Criteria:**

**Given** a query requires dynamic WHERE clauses
**When** the query is built
**Then**:
  - SQLx QueryBuilder is used
  - All user input is parameterized
  - No `format!()` for SQL construction
**And** existing tests pass

**Technical Notes:**
- Refactor `platform_admin.rs` search queries
- Refactor `organization.rs` dynamic queries
- Refactor `user.rs` search queries
- Use `QueryBuilder::push` and `push_bind`

**Files to Modify:**
- `backend/crates/db/src/repositories/platform_admin.rs`
- `backend/crates/db/src/repositories/organization.rs`
- `backend/crates/db/src/repositories/user.rs`

---

#### Epic 80: Test Infrastructure
**Goal:** Establish comprehensive test coverage across all platforms to prevent regressions and catch security issues early.

**Target Apps:** All
**Estimate:** 6 stories, ~3 weeks
**Dependencies:** Epic 79 (Security Hardening)
**Priority:** P0 - CRITICAL

---

##### Story 80.1: Enable RLS Penetration Tests in CI

As a **QA engineer**,
I want to **run RLS penetration tests automatically in CI**,
So that **tenant isolation is continuously verified**.

**Acceptance Criteria:**

**Given** a PR is submitted
**When** CI runs
**Then**:
  - RLS penetration tests execute (not ignored)
  - Cross-tenant access attempts are tested
  - Super admin boundary tests pass
  - Test database is provisioned with proper isolation
**And** failures block merge

**Technical Notes:**
- Remove `#[ignore]` from RLS tests
- Configure test database with RLS enabled
- Add CI workflow step for security tests
- Ensure tests clean up after themselves

**Files to Modify:**
- `backend/crates/db/tests/rls_penetration_tests.rs`
- `.github/workflows/test.yml`

---

##### Story 80.2: API Route Integration Tests

As a **QA engineer**,
I want to **test all authentication endpoints end-to-end**,
So that **auth flows work correctly**.

**Acceptance Criteria:**

**Given** auth endpoints exist
**When** integration tests run
**Then**:
  - Register, login, logout flows are tested
  - Token refresh is tested
  - Password reset is tested
  - MFA flows are tested
  - Error responses are validated
**And** HTTP status codes are correct

**Technical Notes:**
- Create `tests/integration/auth_tests.rs`
- Use `axum::test` for HTTP testing
- Mock email service
- Test with test database

**Files to Create:**
- `backend/servers/api-server/tests/integration/auth_tests.rs`
- `backend/servers/api-server/tests/integration/mod.rs`

---

##### Story 80.3: Frontend Test Infrastructure (Vitest)

As a **frontend developer**,
I want to **set up Vitest for frontend testing**,
So that **components can be tested**.

**Acceptance Criteria:**

**Given** the frontend codebase
**When** I run `pnpm test`
**Then**:
  - Vitest executes tests
  - React Testing Library is available
  - Coverage reports are generated
  - Tests run in CI
**And** at least 5 example tests exist

**Technical Notes:**
- Add vitest to ppt-web
- Configure React Testing Library
- Add coverage thresholds
- Create example tests for auth context

**Files to Create/Modify:**
- `frontend/apps/ppt-web/vitest.config.ts`
- `frontend/apps/ppt-web/src/setupTests.ts`
- `frontend/apps/ppt-web/src/contexts/__tests__/AuthContext.test.tsx`

---

##### Story 80.4: API Client Tests

As a **frontend developer**,
I want to **test API client wrappers**,
So that **API integration is reliable**.

**Acceptance Criteria:**

**Given** the API client package
**When** tests run
**Then**:
  - Request formation is tested
  - Error handling is tested
  - Auth header attachment is tested
  - Response parsing is tested
**And** mocks are used appropriately

**Technical Notes:**
- Add tests to `packages/api-client`
- Use MSW for API mocking
- Test error scenarios
- Test auth token handling

**Files to Create:**
- `frontend/packages/api-client/src/__tests__/client.test.ts`
- `frontend/packages/api-client/src/__tests__/auth.test.ts`

---

##### Story 80.5: Repository Layer Tests

As a **backend developer**,
I want to **test all repository operations**,
So that **database operations are reliable**.

**Acceptance Criteria:**

**Given** a repository module
**When** tests run
**Then**:
  - CRUD operations are tested
  - Constraint violations are handled
  - NULL handling is correct
  - Transaction rollback works
**And** tests use test database

**Technical Notes:**
- Create test fixtures for common data
- Test UserRepository operations
- Test OrganizationRepository operations
- Test BuildingRepository operations

**Files to Create:**
- `backend/crates/db/tests/user_repository_tests.rs`
- `backend/crates/db/tests/organization_repository_tests.rs`

---

##### Story 80.6: Mobile Unit Tests (iOS)

As a **mobile developer**,
I want to **add unit tests for iOS auth flows**,
So that **authentication is reliable**.

**Acceptance Criteria:**

**Given** the iOS app
**When** tests run
**Then**:
  - AuthManager is tested
  - Keychain operations are tested
  - SSO flow is tested
  - Error handling is tested
**And** tests pass in CI

**Technical Notes:**
- Add XCTest targets
- Mock SsoService for testing
- Test token storage/retrieval
- Test session restoration

**Files to Create:**
- `mobile-native/iosApp/iosAppTests/AuthManagerTests.swift`

---

### Phase 23: Frontend & Mobile Completion

#### Epic 81: Frontend API Integration
**Goal:** Wire all frontend pages to real API endpoints, removing mock data dependencies.

**Target Apps:** ppt-web
**Estimate:** 8 stories, ~3 weeks
**Dependencies:** Epic 79 (Security), Epic 80 (Tests)
**Priority:** P1 - HIGH

---

##### Story 81.1: Migration Pages API Integration

As a **manager**,
I want to **use real import/export functionality**,
So that **I can migrate data**.

**Acceptance Criteria:**

**Given** the ExportPage and ImportPage
**When** I use import/export features
**Then**:
  - Real API endpoints are called
  - Progress is tracked
  - Errors are displayed
  - Mock data is removed
**And** templates are downloadable

**Technical Notes:**
- Wire ExportPage to `/api/v1/migration/export`
- Wire ImportPage to `/api/v1/migration/import`
- Remove MOCK_CATEGORIES, MOCK_HISTORY
- Add loading and error states

**Files to Modify:**
- `frontend/apps/ppt-web/src/features/migration/pages/ExportPage.tsx`
- `frontend/apps/ppt-web/src/features/migration/pages/ImportPage.tsx`

---

##### Story 81.2: Packages & Visitors Pages API Integration

As a **manager**,
I want to **manage packages and visitors via real API**,
So that **operations are persisted**.

**Acceptance Criteria:**

**Given** the PackagesPage and VisitorsPage
**When** I use package/visitor features
**Then**:
  - CRUD operations call real API
  - Console.log handlers are replaced
  - Data is persisted
**And** real-time updates work

**Technical Notes:**
- Create TanStack Query hooks
- Replace console.log with API calls
- Add optimistic updates
- Add error handling

**Files to Modify:**
- `frontend/apps/ppt-web/src/features/packages/pages/PackagesPage.tsx`
- `frontend/apps/ppt-web/src/features/packages/pages/VisitorsPage.tsx`

---

##### Story 81.3: Registry Page API Integration

As a **manager**,
I want to **manage registrations via real API**,
So that **guest data is recorded**.

**Acceptance Criteria:**

**Given** the RegistryPage
**When** I manage registrations
**Then**:
  - All 16 handlers call real API
  - Data is fetched from server
  - Filters work with query params
**And** export functionality works

**Technical Notes:**
- Replace 16 console.log handlers
- Create registration API hooks
- Wire filters to API query params

**Files to Modify:**
- `frontend/apps/ppt-web/src/features/registry/pages/RegistryPage.tsx`

---

##### Story 81.4: Competitive Analysis API Integration

As a **manager**,
I want to **view real competitive data**,
So that **I can make informed decisions**.

**Acceptance Criteria:**

**Given** the CompetitiveAnalysisPage
**When** I view competitive data
**Then**:
  - Real market data is displayed
  - Mock data (250+ lines) is removed
  - Event handlers work
**And** data refreshes periodically

**Technical Notes:**
- Create competitive analysis API client
- Remove hardcoded SAMPLE_ data
- Add data fetching hooks

**Files to Modify:**
- `frontend/apps/ppt-web/src/features/competitive/pages/CompetitiveAnalysisPage.tsx`

---

##### Story 81.5: Developer Portal API Integration

As a **developer**,
I want to **manage API keys and webhooks via real API**,
So that **integrations work**.

**Acceptance Criteria:**

**Given** the DeveloperPortalPage
**When** I manage API keys/webhooks
**Then**:
  - Keys are created/revoked via API
  - Webhooks are configured via API
  - Usage data is real
**And** sandbox testing works

**Technical Notes:**
- Wire to `/api/v1/developer/keys`
- Wire to `/api/v1/developer/webhooks`
- Remove mock developer account data

**Files to Modify:**
- `frontend/apps/ppt-web/src/features/developer/pages/DeveloperPortalPage.tsx`
- `frontend/apps/ppt-web/src/features/developer/components/ApiDocumentation.tsx`

---

##### Story 81.6: Content Moderation API Integration

As a **moderator**,
I want to **moderate content via real API**,
So that **actions are recorded**.

**Acceptance Criteria:**

**Given** the ContentModerationPage
**When** I take moderation actions
**Then**:
  - Actions call real API
  - Content is fetched from server
  - Assignments are persisted
**And** audit trail is created

**Technical Notes:**
- Replace 6 console.log handlers
- Create moderation API hooks
- Add action confirmation dialogs

**Files to Modify:**
- `frontend/apps/ppt-web/src/features/compliance/pages/ContentModerationPage.tsx`

---

##### Story 81.7: WebSocket Preference Sync

As a **user**,
I want to **receive real-time preference updates**,
So that **my settings sync across devices**.

**Acceptance Criteria:**

**Given** the usePreferenceSync hook
**When** preferences change
**Then**:
  - WebSocket connection is established
  - isConnected returns true
  - lastSync is updated
**And** preferences sync in real-time

**Technical Notes:**
- Implement actual WebSocket connection
- Remove stub implementation
- Add reconnection logic
- Add connection status UI

**Files to Modify:**
- `frontend/packages/api-client/src/notification-preferences/sync.ts`

---

##### Story 81.8: Empty Handler Props Cleanup

As a **developer**,
I want to **remove all empty handler props**,
So that **UI buttons work**.

**Acceptance Criteria:**

**Given** components with `() => {}` handlers
**When** users click buttons
**Then**:
  - Actions are performed or buttons are hidden
  - No silent failures
**And** TypeScript strict mode passes

**Technical Notes:**
- Audit all `() => {}` usages
- Either implement handlers or remove props
- Add disabled state for unimplemented features

**Files to Modify:**
- `frontend/apps/ppt-web/src/features/migration/pages/ImportPage.tsx`
- `frontend/apps/ppt-web/src/features/migration/pages/ExportPage.tsx`

---

#### Epic 82: Mobile Native Completion
**Goal:** Fix broken functionality in the iOS Reality Portal app and complete API integration.

> **Note — story-number collision.** This Epic 82 (the authoritative one) is a
> *completion/fix* epic on an already-built iOS app. A separate, legacy *build*
> story set under `_bmad-output/implementation-artifacts/stories/82-*.md` reuses
> the same numbers (e.g. its "82.1" is "SwiftUI Project Setup", not "Remove
> Broken Direct Login Form" below). For the full reconciliation of both
> definitions against the actual `mobile-native/iosApp/` implementation, see
> [`docs/superpowers/plans/epic-82-story-mapping.md`](../docs/superpowers/plans/epic-82-story-mapping.md).

**Target Apps:** mobile-native (iosApp, shared)
**Estimate:** 6 stories, ~2 weeks
**Dependencies:** Epic 79 (Security)
**Priority:** P1 - HIGH

---

##### Story 82.1: Remove Broken Direct Login Form

As a **mobile user**,
I want to **not see a login form that doesn't work**,
So that **I'm not confused**.

**Acceptance Criteria:**

**Given** the LoginView
**When** it displays
**Then**:
  - Email/password form is removed OR
  - SSO-only messaging is clear
  - SSO button is prominent
**And** PM app installation is checked

**Technical Notes:**
- Either implement direct login OR
- Remove email/password fields
- Add "Install Property Management" fallback

**Files to Modify:**
- `mobile-native/iosApp/iosApp/Features/Auth/LoginView.swift`

---

##### Story 82.2: Fix Session Restoration Race Condition

As a **mobile user**,
I want to **not see authentication flicker on launch**,
So that **the app feels stable**.

**Acceptance Criteria:**

**Given** the app launches
**When** session is restored
**Then**:
  - Loading state is shown
  - Views wait for session check
  - No flicker between states
**And** restoration is complete before navigation

**Technical Notes:**
- Make restoreSession async
- Add loading state to MainTabView
- Await session before rendering

**Files to Modify:**
- `mobile-native/iosApp/iosApp/Core/AuthManager.swift`
- `mobile-native/iosApp/iosApp/App/RealityPortalApp.swift`

---

##### Story 82.3: Wire Category Filter Chips

As a **mobile user**,
I want to **filter listings by category**,
So that **I find relevant properties**.

**Acceptance Criteria:**

**Given** the HomeView category chips
**When** I tap a category
**Then**:
  - Navigation to SearchView occurs
  - Filter is pre-applied
**And** results are filtered

**Technical Notes:**
- Implement chip tap handlers
- Pass filter to SearchView
- Update SearchView to accept initial filters

**Files to Modify:**
- `mobile-native/iosApp/iosApp/Features/Home/HomeView.swift`
- `mobile-native/iosApp/iosApp/Features/Search/SearchView.swift`

---

##### Story 82.4: Persist Recent Searches

As a **mobile user**,
I want to **see my actual recent searches**,
So that **I can quickly repeat searches**.

**Acceptance Criteria:**

**Given** I perform searches
**When** I view search suggestions
**Then**:
  - My actual recent searches appear
  - Searches are persisted across sessions
  - Hardcoded suggestions are removed
**And** I can clear search history

**Technical Notes:**
- Store searches in UserDefaults
- Limit to last 10 searches
- Add clear history option

**Files to Modify:**
- `mobile-native/iosApp/iosApp/Features/Search/SearchView.swift`

---

##### Story 82.5: Add Map Integration

As a **mobile user**,
I want to **see property location on a map**,
So that **I can understand the location**.

**Acceptance Criteria:**

**Given** the ListingDetailView
**When** I view location section
**Then**:
  - MapKit map displays
  - Property marker is shown
  - Map is interactive
**And** directions can be requested

**Technical Notes:**
- Replace placeholder with MapKit
- Add pin annotation
- Add "Get Directions" button

**Files to Modify:**
- `mobile-native/iosApp/iosApp/Features/Listing/ListingDetailView.swift`

---

##### Story 82.6: Initialize ApiConfig at Startup

As a **mobile developer**,
I want to **ensure ApiConfig is initialized before use**,
So that **API calls don't crash**.

**Acceptance Criteria:**

**Given** the app launches
**When** any API call is made
**Then**:
  - ApiConfig.baseUrl is set
  - No crashes from missing config
**And** configuration matches environment

**Technical Notes:**
- Initialize in RealityPortalApp.configureApp()
- Use Configuration.shared.apiBaseUrl
- Add validation

**Files to Modify:**
- `mobile-native/iosApp/iosApp/App/RealityPortalApp.swift`
- `mobile-native/shared/src/commonMain/kotlin/three/two/bit/ppt/reality/api/ApiConfig.kt`

---

### Phase 24: Reliability & Observability

#### Epic 83: Silent Failure Remediation
**Goal:** Replace silent error swallowing with proper error handling and logging throughout the codebase.

**Target Apps:** api-server, ppt-web, mobile
**Estimate:** 5 stories, ~2 weeks
**Dependencies:** None
**Priority:** P1 - HIGH

---

##### Story 83.1: Backend Audit Log Failures

As a **administrator**,
I want to **all audit log failures to be visible**,
So that **compliance gaps are detected**.

**Acceptance Criteria:**

**Given** an audit log operation fails
**When** the error occurs
**Then**:
  - Error is logged at ERROR level
  - Metrics are incremented
  - Alert is triggered (if configured)
**And** original request still succeeds

**Technical Notes:**
- Already partially fixed
- Add metrics for audit failures
- Add structured logging fields
- Consider retry mechanism

**Files to Modify:**
- `backend/servers/api-server/src/routes/government_portal.rs`
- `backend/servers/api-server/src/routes/documents.rs`

---

##### Story 83.2: Backend Analytics Failures

As a **developer**,
I want to **analytics failures to be logged**,
So that **data loss is detectable**.

**Acceptance Criteria:**

**Given** view tracking or analytics fails
**When** the error occurs
**Then**:
  - Error is logged at WARN level
  - Failure count is tracked
**And** user experience is not affected

**Technical Notes:**
- Fix faults.rs AI suggestion logging (done)
- Fix listings view tracking
- Fix help article view tracking
- Add structured logging

**Files to Modify:**
- `backend/servers/reality-server/src/routes/listings.rs`
- `backend/servers/api-server/src/routes/help.rs`

---

##### Story 83.3: Backend Session/Auth Failures

As a **security engineer**,
I want to **auth-related failures to be logged**,
So that **security issues are visible**.

**Acceptance Criteria:**

**Given** token invalidation or login attempt recording fails
**When** the error occurs
**Then**:
  - Error is logged at ERROR level
  - Security team is notified
**And** rate limiting remains effective

**Technical Notes:**
- Fix token invalidation logging
- Fix login attempt recording
- Fail-closed for rate limiting

**Files to Modify:**
- `backend/servers/api-server/src/handlers/auth/mod.rs`
- `backend/servers/api-server/src/routes/auth.rs`

---

##### Story 83.4: Frontend Widget Error Handling

As a **mobile user**,
I want to **see error states when widgets fail**,
So that **I know something is wrong**.

**Acceptance Criteria:**

**Given** a widget API call fails
**When** the error occurs
**Then**:
  - Error UI is displayed (not empty)
  - Retry option is available
  - Error is logged to Sentry
**And** mock data is never used in production

**Technical Notes:**
- Fix WidgetDataProvider
- Add error state UI
- Remove mock data fallback
- Add Sentry integration

**Files to Modify:**
- `frontend/apps/mobile/src/widgets/WidgetDataProvider.ts`

---

##### Story 83.5: Frontend View Tracking Visibility

As a **developer**,
I want to **view tracking failures to be logged**,
So that **analytics accuracy is measurable**.

**Acceptance Criteria:**

**Given** view tracking fails
**When** the error occurs
**Then**:
  - Error is logged (not silently ignored)
  - Failure rate is measurable
**And** comment about silent failure is removed

**Technical Notes:**
- Add error logging to ArticleDetailPage
- Consider retry mechanism
- Add error tracking service integration

**Files to Modify:**
- `frontend/apps/ppt-web/src/features/news/pages/ArticleDetailPage.tsx`

---

#### Epic 84: TODO Cleanup
**Goal:** Address critical TODO items that mask unimplemented functionality.

**Target Apps:** api-server
**Estimate:** 6 stories, ~2 weeks
**Dependencies:** Varies by TODO
**Priority:** P2 - MEDIUM

---

##### Story 84.1: Implement Report Export

As a **manager**,
I want to **export reports**,
So that **I can share them externally**.

**Acceptance Criteria:**

**Given** the reports page
**When** I click export
**Then**:
  - PDF/Excel is generated
  - Download starts
**And** TODO in reports.rs is removed

**Technical Notes:**
- Implement actual export logic
- Support PDF and Excel formats
- Add async job for large reports

**Files to Modify:**
- `backend/servers/api-server/src/routes/reports.rs`

---

##### Story 84.2: Implement Airbnb/Booking OAuth

As a **property manager**,
I want to **connect Airbnb/Booking accounts**,
So that **listings sync**.

**Acceptance Criteria:**

**Given** the integrations page
**When** I connect Airbnb/Booking
**Then**:
  - OAuth flow completes
  - Tokens are stored securely
**And** TODO in rentals.rs is removed

**Technical Notes:**
- Implement OAuth 2.0 flows
- Store refresh tokens encrypted
- Add token refresh mechanism

**Files to Modify:**
- `backend/servers/api-server/src/routes/rentals.rs`

---

##### Story 84.3: Implement Listing Syndication

As a **real estate agent**,
I want to **syndicate listings to portals**,
So that **they appear on multiple sites**.

**Acceptance Criteria:**

**Given** a listing is published
**When** syndication is triggered
**Then**:
  - Status propagates to portals
  - External IDs are tracked
**And** TODO in listings.rs is removed

**Technical Notes:**
- Implement portal API clients
- Add syndication job queue
- Track sync status per portal

**Files to Modify:**
- `backend/servers/api-server/src/routes/listings.rs`

---

##### Story 84.4: Implement Reminder Sending

As a **user**,
I want to **receive integration reminders**,
So that **I don't miss deadlines**.

**Acceptance Criteria:**

**Given** a reminder is scheduled
**When** the time arrives
**Then**:
  - Email/push notification is sent
**And** TODO in integrations.rs is removed

**Technical Notes:**
- Wire to email service
- Add push notification support
- Add reminder scheduling

**Files to Modify:**
- `backend/servers/api-server/src/routes/integrations.rs`

---

##### Story 84.5: Implement RAG Vector Search

As a **user**,
I want to **search documents semantically**,
So that **I find relevant content**.

**Acceptance Criteria:**

**Given** documents are indexed
**When** I search with natural language
**Then**:
  - Semantic search returns results
  - pgvector is used for embeddings
**And** TODO in llm_document.rs is removed

**Technical Notes:**
- Add pgvector extension
- Implement embedding generation
- Add vector similarity search

**Files to Modify:**
- `backend/crates/db/src/repositories/llm_document.rs`

---

##### Story 84.6: Implement Price Change Tracking

As a **portal user**,
I want to **see price changes on favorites**,
So that **I know when prices drop**.

**Acceptance Criteria:**

**Given** I have favorited listings
**When** prices change
**Then**:
  - Price history is tracked
  - Notifications are sent
**And** TODO in portal.rs is removed

**Technical Notes:**
- Add price_history table
- Track original_price
- Add price drop notification

**Files to Modify:**
- `backend/crates/db/src/repositories/portal.rs`

---

#### Epic 85: Documentation & Cleanup
**Goal:** Remove dead code, update documentation, and ensure consistency.

**Target Apps:** All
**Estimate:** 4 stories, ~1 week
**Dependencies:** None
**Priority:** P3 - LOW

---

##### Story 85.1: Remove Sample Data from Production

As a **developer**,
I want to **remove sample data from production builds**,
So that **binary size is reduced**.

**Acceptance Criteria:**

**Given** sample data exists in views
**When** production builds are created
**Then**:
  - Sample data is excluded
  - No dead code in binary
**And** preview providers use test targets only

**Technical Notes:**
- Move sample data to #if DEBUG
- Use test targets for previews
- Audit all static sample arrays

**Files to Modify:**
- `mobile-native/iosApp/iosApp/Features/Home/HomeView.swift`
- `mobile-native/iosApp/iosApp/Features/Inquiries/InquiriesView.swift`

---

##### Story 85.2: Fix Hardcoded Version Numbers

As a **developer**,
I want to **version numbers to be dynamic**,
So that **they match releases**.

**Acceptance Criteria:**

**Given** version is displayed
**When** I view the app
**Then**:
  - Version matches Info.plist
  - Single source of truth
**And** hardcoded "0.1.0" is removed

**Technical Notes:**
- Read from Bundle.main.infoDictionary
- Create version utility
- Update AccountView

**Files to Modify:**
- `mobile-native/iosApp/iosApp/Features/Account/AccountView.swift`

---

##### Story 85.3: Fix Share Link Domain

As a **user**,
I want to **share links with real domain**,
So that **recipients can access them**.

**Acceptance Criteria:**

**Given** I share a listing
**When** the link is generated
**Then**:
  - Real domain is used
  - Link works for recipients
**And** example.com is removed

**Technical Notes:**
- Use Configuration.shared.webBaseUrl
- Ensure deep linking works

**Files to Modify:**
- `mobile-native/iosApp/iosApp/Features/Listing/ListingDetailView.swift`

---

##### Story 85.4: Update Root README

As a **new developer**,
I want to **understand the project quickly**,
So that **I can contribute**.

**Acceptance Criteria:**

**Given** the root README.md
**When** I read it
**Then**:
  - Project overview is clear
  - Setup instructions work
  - Architecture is explained
**And** empty README is replaced

**Technical Notes:**
- Add quick start guide
- Link to detailed docs
- Add architecture diagram

**Files to Modify:**
- `README.md`

---

## Summary

| Phase | Epics | Stories | Priority |
|-------|-------|---------|----------|
| 22: Security & Quality | 79-80 | 11 | P0 - CRITICAL |
| 23: Frontend & Mobile | 81-82 | 14 | P1 - HIGH |
| 24: Reliability | 83-85 | 15 | P1-P3 |

**Total:** 7 Epics, 40 Stories

### Implementation Order

1. **Epic 79** - Security Hardening (P0)
2. **Epic 80** - Test Infrastructure (P0)
3. **Epic 83** - Silent Failure Remediation (P1)
4. **Epic 81** - Frontend API Integration (P1)
5. **Epic 82** - Mobile Native Completion (P1)
6. **Epic 84** - TODO Cleanup (P2)
7. **Epic 85** - Documentation & Cleanup (P3)
