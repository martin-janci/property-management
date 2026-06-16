---
stepsCompleted: [1, 2, 3, 4]
inputDocuments:
  - _bmad-output/prd.md
  - _bmad-output/architecture.md
  - _bmad-output/ux-design-specification.md
workflowType: 'epics-and-stories'
lastStep: 4
status: 'complete'
project_name: 'Property Management System (PPT) & Reality Portal'
user_name: 'Martin Janci'
date: '2025-12-22'
elicitation_methods_applied:
  - Pre-mortem Analysis
  - Architecture Decision Records
  - Cross-Functional War Room
mvp_stories_generated: 74
mvp_acceptance_criteria: 222
phase2_stories_generated: 23
phase2_epics: 5
phase3_stories_generated: 13
phase3_epics: 2
phase4_stories_generated: 24
phase4_epics: 6
phase5_stories_generated: 25
phase5_epics: 6
total_stories_generated: 159
total_epics: 31
validation_passed: true
fr_coverage: '134/134 FRs (Phases 1-4 = FR1-101; Phase 5 backfill = FR102-134)'
phase5_note: 'Phase 5 (Epics 25-30) backfills already-shipped backend modules per PAP-17 Option C; documentation only, no feature code. Per-epic UI-wiring gaps are recorded where the surface is dead-code.'
---

# Property Management System (PPT) & Reality Portal - Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for Property Management System (PPT) & Reality Portal, decomposing the requirements from the PRD, UX Design, and Architecture requirements into implementable stories.

## Requirements Inventory

### Functional Requirements

**From PRD - 101 Functional Requirements across 15 Capability Areas:**

| ID | Capability Area | FR Count | Phase |
|----|-----------------|----------|-------|
| CA-01 | Identity & Access Management | 7 FRs | MVP |
| CA-02 | Organization & Multi-Tenancy | 7 FRs | MVP |
| CA-03 | Property & Resident Management | 7 FRs | MVP |
| CA-04 | Communication & Notifications | 8 FRs | MVP |
| CA-05 | Issue & Fault Management | 7 FRs | MVP |
| CA-06 | Voting & Decision Making | 8 FRs | MVP |
| CA-07 | Document Management | 7 FRs | MVP |
| CA-08 | Financial Management | 6 FRs | Phase 2 |
| CA-09 | Meter Readings & Utilities | 6 FRs | Phase 2 |
| CA-10 | AI & Automation | 7 FRs | Phase 3 |
| CA-11 | IoT & Smart Building | 5 FRs | Phase 3 |
| CA-12 | Real Estate & Listings | 8 FRs | Phase 4 |
| CA-13 | Rental Management | 6 FRs | Phase 4 |
| CA-14 | Compliance & Privacy | 6 FRs | MVP |
| CA-15 | Platform Operations | 6 FRs | MVP |
| CA-16 | Facilities & Operations | 6 FRs | Phase 5 |
| CA-17 | Disputes, Legal, Insurance & Compliance | 8 FRs | Phase 5 |
| CA-18 | Community, Marketplace & News | 5 FRs | Phase 5 |
| CA-19 | Investor & Portfolio Analytics | 7 FRs | Phase 5 |
| CA-20 | ESG & Building Certifications | 3 FRs | Phase 5 |
| CA-21 | Forms, Registry & Government Portal | 4 FRs | Phase 5 |

**Detailed Functional Requirements:**

**CA-01: Identity & Access Management (7 FRs)**
- FR1: Users can register accounts with email verification (UC-14.1, UC-14.2)
- FR2: Users can authenticate via email/password with session management (UC-14.3, UC-14.4)
- FR3: Users can reset forgotten passwords securely (UC-14.5, UC-14.6)
- FR4: Users can enable two-factor authentication (UC-23.1)
- FR5: Users can manage active sessions across devices (UC-14.11, UC-14.12)
- FR6: Platform operators can manage user lifecycle (invite, suspend, delete) (UC-14.7-14.10)
- FR7: Users can authenticate via SSO from Property Management to Reality Portal (UC-46.3)

**CA-02: Organization & Multi-Tenancy (7 FRs)**
- FR8: Organizations can be created with complete isolation from other tenants (UC-27.1)
- FR9: Organization admins can configure organization settings and branding (UC-27.2, UC-27.3)
- FR10: Organization admins can manage members and assign roles (UC-27.4, UC-27.5)
- FR11: Organizations can define custom role permissions within RBAC framework (UC-27.6)
- FR12: Super admins can view and manage all organizations (UC-18.1-18.10)
- FR13: Organizations can export all their data for migration (UC-27.7)
- FR14: Organizations can be deactivated with data retention per policy (UC-27.8)

**CA-03: Property & Resident Management (7 FRs)**
- FR15: Managers can create and configure buildings with address and metadata (UC-15.1)
- FR16: Managers can define units within buildings with ownership/rental status (UC-15.2)
- FR17: Managers can associate residents (owners, tenants) with units (UC-15.3, UC-15.4)
- FR18: Owners can delegate rights to other users (UC-28.1-28.4)
- FR19: Managers can track person-months for fee allocation (UC-10.1-10.6)
- FR20: Residents can view their unit details and associated information (UC-15.5)
- FR21: Managers can manage common areas and shared facilities (UC-15.6)

**CA-04: Communication & Notifications (8 FRs)**
- FR22: Managers can create announcements visible to specific buildings/units (UC-02.1-02.4)
- FR23: Residents can view, comment on, and acknowledge announcements (UC-02.5-02.8)
- FR24: Managers can pin important announcements (UC-02.9)
- FR25: Users can send direct messages to other users within their organization (UC-05.1-05.4)
- FR26: Users receive push notifications for relevant events (UC-01.1-01.3)
- FR27: Users can configure notification preferences by channel and category (UC-01.4-01.6)
- FR28: System can send email notifications for offline users (UC-01.2)
- FR29: Users can view neighbor information based on privacy settings (UC-06.1-06.4)

**CA-05: Issue & Fault Management (7 FRs)**
- FR30: Residents can report faults with description, category, and photos (UC-03.1-03.3)
- FR31: Managers can view, triage, and assign faults to technical staff (UC-03.4-03.6)
- FR32: Technical managers can update fault status through resolution workflow (UC-03.7-03.9)
- FR33: Residents can track status of their reported faults (UC-03.10)
- FR34: Residents can rate resolved faults (UC-03.11)
- FR35: System can suggest fault category and priority based on description (AI-assisted) (UC-20.3)
- FR36: Managers can generate fault reports and analytics (UC-17.4)

**CA-06: Voting & Decision Making (8 FRs)**
- FR37: Managers can create votes with multiple question types (yes/no, multiple choice, ranked) (UC-04.1-04.3)
- FR38: Owners can cast votes during voting period (UC-04.4)
- FR39: Owners can delegate voting rights (UC-28.2)
- FR40: Users can discuss votes in associated threads (UC-04.5)
- FR41: System calculates and displays results based on configured quorum (UC-04.6-04.7)
- FR42: System maintains immutable audit trail for all voting activity (UC-04.8)
- FR43: Managers can generate compliance reports for votes (UC-04.9)
- FR44: System supports hybrid attendance (in-person + remote) (UC-04.10)

**CA-07: Document Management (7 FRs)**
- FR45: Users can upload documents with metadata and categorization (UC-08.1-08.3)
- FR46: Managers can organize documents in folder structures (UC-08.4)
- FR47: Users can view documents based on their access permissions (UC-08.5)
- FR48: System maintains version history for documents (UC-08.6)
- FR49: Users can search documents by content and metadata (UC-08.7)
- FR50: Managers can share documents with specific users or groups (UC-08.8)
- FR51: System can extract text from uploaded documents (OCR) (UC-20.2)

**CA-08: Financial Management (6 FRs) - Phase 2**
- FR52: Owners can view their payment history and current balance (UC-16.1-16.3)
- FR53: Managers can record payments and generate invoices (UC-16.4-16.6)
- FR54: Managers can manage building maintenance fund (UC-16.7-16.9)
- FR55: System can generate financial reports by period, building, category (UC-17.1-17.3)
- FR56: System can export financial data to accounting systems (POHODA, Money S3) (UC-22.2)
- FR57: Managers can send payment reminders (UC-16.10)

**CA-09: Meter Readings & Utilities (6 FRs) - Phase 2**
- FR58: Residents can submit meter readings with photos (UC-11.1-11.3)
- FR59: System can extract meter values from photos (OCR) (UC-20.1)
- FR60: Managers can view and validate submitted readings (UC-11.4-11.5)
- FR61: System can detect anomalous readings (UC-11.6)
- FR62: Managers can generate utility reports (UC-11.7)
- FR63: System can track outages and service interruptions (UC-12.1-12.4)

**CA-10: AI & Automation (7 FRs) - Phase 3**
- FR64: Users can interact with AI chatbot for common questions (UC-20.4)
- FR65: System can analyze message sentiment for trend detection (UC-20.5)
- FR66: System can predict maintenance needs based on equipment age and history (UC-20.6)
- FR67: System can summarize long documents automatically (UC-20.7)
- FR68: System can provide smart search with natural language queries (UC-20.8)
- FR69: Managers can configure workflow automations (UC-26.1-26.4)
- FR70: System can trigger automated actions based on events (UC-26.5-26.8)

**CA-11: IoT & Smart Building (5 FRs) - Phase 3**
- FR71: System can ingest data from IoT sensors (UC-21.1-21.3)
- FR72: Users can view real-time sensor data dashboards (UC-21.4)
- FR73: System can alert on threshold violations (UC-21.5)
- FR74: System can correlate sensor data with fault reports (UC-21.6)
- FR75: Managers can configure sensor thresholds and alerts (UC-21.7)

**CA-12: Real Estate & Listings (8 FRs) - Phase 4**
- FR76: Owners can create property listings from existing unit data (UC-31.1-31.3)
- FR77: Realtors can manage listings with photos, descriptions, pricing (UC-31.4-31.6)
- FR78: System can syndicate listings to external portals (UC-32.1-32.4)
- FR79: Portal users can search and filter property listings (UC-44.1-44.4)
- FR80: Portal users can save favorite listings (UC-44.5-44.8)
- FR81: Portal users can contact listing agents (UC-45.1-45.4)
- FR82: Agencies can manage realtors and shared listings (UC-49.1-49.6)
- FR83: Realtors can import listings from external sources (UC-50.1-50.4)

**CA-13: Rental Management (6 FRs) - Phase 4**
- FR84: Property managers can sync with Airbnb/Booking.com (UC-29.1-29.4)
- FR85: Property managers can register guests for legal compliance (UC-30.1-30.4)
- FR86: System can generate guest reports for authorities (UC-30.5-30.6)
- FR87: Landlords can screen potential tenants (UC-33.1-33.4)
- FR88: Landlords can manage lease lifecycle (create, renew, terminate) (UC-34.1-34.6)
- FR89: System can track lease expirations and send reminders (UC-34.7-34.8)

**CA-14: Compliance & Privacy (6 FRs)**
- FR90: Users can export all their personal data (GDPR) (UC-23.4)
- FR91: Users can request deletion of their personal data (GDPR) (UC-23.5)
- FR92: Users can configure privacy settings for profile visibility (UC-23.1-23.3)
- FR93: System maintains audit logs for compliance-sensitive operations (UC-23.6)
- FR94: Managers can generate compliance reports (UC-23.7)
- FR95: System enforces data retention policies per regulation (UC-23.8)

**CA-15: Platform Operations (6 FRs)**
- FR96: Super admins can view platform health metrics (UC-18.1)
- FR97: Super admins can manage feature flags (UC-18.2)
- FR98: Super admins can broadcast system announcements (UC-18.3)
- FR99: Support staff can access organization data for troubleshooting (UC-18.4)
- FR100: System provides onboarding tour for new users (UC-42.1)
- FR101: System provides contextual help and documentation (UC-42.2)

> **Phase 5 (FR102-134) — backfill of already-shipped backend.** The following FRs document
> capabilities that already exist in code (route module + migration) but had no catalog entry.
> Authored per PAP-17 Option C; "shipped on backend" unless a UI-wiring gap is noted on the
> owning epic (Epics 25-30). UC references are indicative (the 508-UC taxonomy in
> `docs/functional-requirements.md` is a separate, unreconciled numbering — see
> `EPIC_STORY_STATUS.md` §6).

**CA-16: Facilities & Operations (6 FRs) - Phase 5** — *Epic 25*
- FR102: Managers can create and assign maintenance work orders with status lifecycle (`work_orders.rs`)
- FR103: Managers can manage a vendor/supplier directory with ratings and assignment (`vendors.rs`)
- FR104: Staff can run day-to-day building operations tasks and checklists (`operations.rs`)
- FR105: Front-desk staff can log packages and register/track visitors (`package_visitor.rs`)
- FR106: System tracks planned/unplanned outages and notifies affected residents (`outages.rs`)
- FR107: Managers can schedule recurring/preventive facility maintenance (`work_orders.rs` + `operations.rs`)

**CA-17: Disputes, Legal, Insurance & Compliance (8 FRs) - Phase 5** — *Epic 26*
- FR108: Residents/managers can open and mediate disputes with notes and resolution tracking (`disputes.rs`)
- FR109: Managers can record and enforce rule violations with escalation (`violations.rs`)
- FR110: Managers can store legal documents, contracts and case records (`legal.rs`)
- FR111: Managers can manage building/asset insurance policies and claims (`insurance.rs`)
- FR112: System tracks compliance obligations and generates compliance evidence (`compliance.rs`)
- FR113: System applies region-specific regulatory rules and reporting (`regional_compliance.rs`)
- FR114: Managers can run emergency/incident response workflows and broadcasts (`emergency.rs`)
- FR115: System performs AML/EDD screening and DSA obligations (`aml_dsa.rs`) — **flagged: needs a named owner + security review before this FR is treated as production-ready**

**CA-18: Community, Marketplace & News (5 FRs) - Phase 5** — *Epic 27*
- FR116: Residents can list, browse and trade items in a community marketplace (`marketplace.rs`)
- FR117: Residents can join community groups, events and discussions (`community.rs`)
- FR118: Managers can publish news articles and media to residents (`news_articles.rs`)
- FR119: Residents can connect with neighbours and resident directory (`neighbors.rs`)
- FR120: System moderates community/marketplace content (`marketplace.rs` + `community.rs`)

**CA-19: Investor & Portfolio Analytics (7 FRs) - Phase 5** — *Epic 28*
- FR121: Investors can view a portfolio investor portal with holdings and returns (`investor_portal.rs`)
- FR122: Owners can view per-property owner analytics (`owner_analytics.rs`)
- FR123: Managers can view cross-property portfolio analytics (`portfolio_analytics.rs`)
- FR124: System computes portfolio performance metrics over time (`portfolio_performance.rs`)
- FR125: System estimates and tracks property valuations (`property_valuation.rs`)
- FR126: Boards can schedule and minute board meetings (`board_meetings.rs`)
- FR127: Owners can manage platform subscriptions and billing plans (`subscriptions.rs`)

**CA-20: ESG & Building Certifications (3 FRs) - Phase 5** — *Epic 29*
- FR128: Managers can produce ESG/sustainability reports (`esg_reporting.rs`)
- FR129: Managers can track building certifications and their renewals (`building_certifications.rs`)
- FR130: System monitors energy/sustainability KPIs feeding ESG reports (`esg_reporting.rs`)

**CA-21: Forms, Registry & Government Portal (4 FRs) - Phase 5** — *Epic 30*
- FR131: Users can build, submit and process dynamic forms (`forms.rs`)
- FR132: System maintains official registries of records (`registry.rs`)
- FR133: System integrates with government portals for filings/lookups (`government_portal.rs`)
- FR134: System validates form/registry submissions against government schemas (`forms.rs` + `government_portal.rs`)

### Non-Functional Requirements

**From PRD - Critical Quality Attributes:**

**NFR-PERF: Performance**
- NFR1: API P95 Latency < 200ms (alert threshold > 500ms)
- NFR2: API P99 Latency < 500ms (alert threshold > 1s)
- NFR3: Database Query P95 < 100ms (alert threshold > 200ms)
- NFR4: Reality Portal LCP < 2.5s (alert threshold > 3s)
- NFR5: Reality Portal FCP < 1.5s (alert threshold > 2s)
- NFR6: Mobile App Launch < 3s (alert threshold > 5s)
- NFR7: Push Notification Delivery < 5s (alert threshold > 15s)

**NFR-CAP: Capacity**
- NFR8: MVP: 500 concurrent users, Year 1: 5,000, Year 3: 15,000
- NFR9: MVP: 100 RPS, Year 1: 1,000 RPS, Year 3: 5,000 RPS
- NFR10: MVP: 1,000 WebSocket connections, Year 1: 10,000, Year 3: 50,000

**NFR-SEC: Security**
- NFR11: Argon2id for password hashing
- NFR12: JWT with 15-minute access, 7-day refresh tokens
- NFR13: TOTP-based 2FA support
- NFR14: AES-256 encryption at rest for PII
- NFR15: TLS 1.3 for all API communication
- NFR16: Rate limiting: 100 req/min per user, 1000 req/min per org

**NFR-REL: Reliability**
- NFR17: 99.9% uptime SLA
- NFR18: Max 4h planned downtime/month
- NFR19: Error rate < 0.1%
- NFR20: MTTR < 30 minutes
- NFR21: RTO < 4 hours
- NFR22: RPO < 1 hour

**NFR-COMP: Compliance**
- NFR23: GDPR full compliance
- NFR24: Data export within 24 hours (UC-23.4)
- NFR25: Data deletion within 72 hours (UC-23.5)
- NFR26: Annual penetration testing
- NFR27: SOC 2 Type II by Year 2

**NFR-ACC: Accessibility**
- NFR28: WCAG 2.1 AA full compliance
- NFR29: Keyboard navigation for all features
- NFR30: Screen reader compatibility (NVDA, VoiceOver)
- NFR31: 4.5:1 minimum color contrast
- NFR32: 200% text scaling support

**NFR-I18N: Localization**
- NFR33: Languages: Slovak (sk), Czech (cs), German (de), English (en)
- NFR34: Locale-aware date/time formatting
- NFR35: Currency display: EUR, CZK

**NFR-MOB: Mobile**
- NFR36: Offline mode with background sync
- NFR37: Push notifications via FCM/APNs
- NFR38: Deep linking support
- NFR39: App size < 50MB
- NFR40: Battery drain < 3% per hour active use

### Additional Requirements

**From Architecture Document:**

**ARCH-TECH: Technology Stack**
- ARCH1: Backend: Rust with Axum 0.8.6 framework
- ARCH2: Database: PostgreSQL 16+ with RLS for tenant isolation
- ARCH3: Cache: Redis for sessions and rate limiting
- ARCH4: Storage: S3-compatible object storage
- ARCH5: ppt-web: React 19 + Vite 6 SPA
- ARCH6: reality-web: Next.js 15.5 SSR/SSG
- ARCH7: mobile: React Native 0.83
- ARCH8: mobile-native: Kotlin Multiplatform 2.3 (iOS/Android)

**ARCH-API: API & Communication**
- ARCH9: OpenAPI 3.1 specification (TypeSpec source)
- ARCH10: SDK generation: TypeScript (@hey-api/openapi-ts), Kotlin (openapi-generator)
- ARCH11: WebSocket for real-time (Axum + tokio-tungstenite)
- ARCH12: Redis pub/sub for cross-instance messaging

**ARCH-AUTH: Authentication**
- ARCH13: api-server as OAuth 2.0 provider
- ARCH14: reality-server as SSO consumer
- ARCH15: 12+ role types across platform/org/building/unit levels

**ARCH-DATA: Data Architecture**
- ARCH16: SQLx for compile-time query checking
- ARCH17: SeaORM for complex query building
- ARCH18: sqlx-cli for schema migrations

**ARCH-AI: AI/ML Architecture**
- ARCH19: Hybrid approach (local + external APIs)
- ARCH20: AI columns in data model (ocr_confidence, ai_category, sentiment_score)
- ARCH21: Confidence thresholds: >90% auto-apply, 70-90% suggest, <70% human review

**ARCH-TEST: Testing Strategy**
- ARCH22: Backend: 80% unit test coverage target
- ARCH23: Web: Vitest + React Testing Library (80% unit, 70% component)
- ARCH24: E2E: Playwright for critical flows
- ARCH25: Accessibility: axe-core in CI

**ARCH-STRUCT: Project Structure**
- ARCH26: Monorepo with pnpm workspaces (frontend/)
- ARCH27: Cargo workspace (backend/)
- ARCH28: Shared design tokens via Style Dictionary

**From UX Design Specification:**

**UX-CORE: Core Experience Principles**
- UX1: Action-First, Dashboard-Available design
- UX2: 60-Second Tasks: Notification to completion in under a minute
- UX3: Confirm, Not Assume: AI assists, user approves
- UX4: No Status Black Holes: Every item shows next expected step
- UX5: Reversible by Default: Undo window (30s), confirmation for high-stakes
- UX6: Design for Ján, Delight Michaela: Accessible first, density optional
- UX7: Notification IS Navigation: Deep-links replace menu hunting
- UX8: Platform-Native, Token-Unified: Consistent language, native feel

**UX-DESIGN: Design System**
- UX9: Token-first design system with Style Dictionary
- UX10: Radix UI + Tailwind CSS for web apps
- UX11: React Native Paper for PM mobile
- UX12: Compose/SwiftUI for Reality Portal mobile (KMP)
- UX13: 48px+ touch targets for accessibility

**UX-FLOW: User Journey Targets**
- UX14: Owner voting: < 60 seconds notification to completion
- UX15: Tenant fault report: < 30 seconds photo to submission
- UX16: Manager action processing: < 20 seconds per queue item
- UX17: Realtor listing publish: < 2 minutes to multi-portal live
- UX18: Portal user inquiry: ≤ 3 clicks from search to contact

**UX-PATTERN: Interaction Patterns**
- UX19: Command Palette (⌘K) for PM web power users
- UX20: Bottom navigation with labels for mobile
- UX21: Photo-first input for meter readings and fault reports
- UX22: Inline approval without drilling down
- UX23: Timeline views for fault status tracking

**UX-VISUAL: Visual Design**
- UX24: PM Palette: Professional blue (#2563EB primary)
- UX25: Reality Palette: Warm neutrals with orange accent (#F97316)
- UX26: Inter font family for all platforms
- UX27: 4px spacing grid system

### FR Coverage Map

| FR | Epic | Description |
|----|------|-------------|
| FR1 | 1 | User registration with email verification |
| FR2 | 1 | Email/password authentication with sessions |
| FR3 | 1 | Secure password reset |
| FR4 | 9 | Two-factor authentication (TOTP) |
| FR5 | 1 | Session management across devices |
| FR6 | 1 | User lifecycle management (invite, suspend, delete) |
| FR7 | 10A + 10A-SSO | SSO between PM and Reality Portal |
| FR8 | 2A | Organization creation with tenant isolation |
| FR9 | 2A | Organization settings and branding |
| FR10 | 2A | Member management and role assignment |
| FR11 | 2A | Custom role permissions within RBAC |
| FR12 | 10B | Super admin organization management |
| FR13 | 2A | Organization data export |
| FR14 | 2A | Organization deactivation with retention |
| FR15 | 3 | Building creation and configuration |
| FR16 | 3 | Unit definition with ownership/rental status |
| FR17 | 3 | Resident association with units |
| FR18 | 3 | Delegation of owner rights |
| FR19 | 3 | Person-month tracking for fees |
| FR20 | 3 | Resident unit view |
| FR21 | 3 | Common areas and facilities management |
| FR22 | 6 | Announcement creation for buildings/units |
| FR23 | 6 | Announcement viewing, commenting, acknowledgment |
| FR24 | 6 | Pinned announcements |
| FR25 | 6 | Direct messaging within organization |
| FR26 | 2B | Push notifications for events |
| FR27 | 2B + 8A + 8B | Notification preferences by channel/category |
| FR28 | 2B | Email notifications for offline users |
| FR29 | 6 | Neighbor information with privacy settings |
| FR30 | 4 | Fault reporting with description, category, photos |
| FR31 | 4 | Fault triage and assignment |
| FR32 | 4 | Fault status workflow |
| FR33 | 4 | Fault status tracking for residents |
| FR34 | 4 | Fault resolution rating |
| FR35 | 4 | AI-suggested fault category and priority |
| FR36 | 4 | Fault reports and analytics |
| FR37 | 5 | Vote creation with multiple question types |
| FR38 | 5 | Vote casting during voting period |
| FR39 | 5 | Delegated voting rights |
| FR40 | 5 | Vote discussion threads |
| FR41 | 5 | Quorum-based result calculation |
| FR42 | 5 | Immutable voting audit trail |
| FR43 | 5 | Voting compliance reports |
| FR44 | 5 | Hybrid attendance support |
| FR45 | 7A | Document upload with metadata |
| FR46 | 7A | Folder organization |
| FR47 | 7A | Permission-based document viewing |
| FR48 | 7B | Document version history |
| FR49 | 7B | Document search by content/metadata |
| FR50 | 7A | Document sharing |
| FR51 | 7B | Document OCR text extraction |
| FR52 | 11 | Payment history and balance viewing |
| FR53 | 11 | Payment recording and invoice generation |
| FR54 | 11 | Maintenance fund management |
| FR55 | 11 | Financial reports |
| FR56 | 11 | Accounting system export |
| FR57 | 11 | Payment reminders |
| FR58 | 12 | Meter reading submission with photos |
| FR59 | 12 | OCR meter value extraction |
| FR60 | 12 | Reading validation |
| FR61 | 12 | Anomaly detection |
| FR62 | 12 | Utility reports |
| FR63 | 12 | Outage tracking |
| FR64 | 13 | AI chatbot for common questions |
| FR65 | 13 | Sentiment analysis for trend detection |
| FR66 | 13 | Predictive maintenance |
| FR67 | 13 | Automatic document summarization |
| FR68 | 13 | Smart search with NLP |
| FR69 | 13 | Workflow automation configuration |
| FR70 | 13 | Event-triggered automated actions |
| FR71 | 14 | IoT sensor data ingestion |
| FR72 | 14 | Real-time sensor dashboards |
| FR73 | 14 | Threshold violation alerts |
| FR74 | 14 | Sensor-fault correlation |
| FR75 | 14 | Sensor threshold configuration |
| FR76 | 15 | Listing creation from unit data |
| FR77 | 15 | Listing management (photos, descriptions, pricing) |
| FR78 | 15 | Multi-portal syndication |
| FR79 | 16 | Property search and filtering |
| FR80 | 16 | Favorite listings |
| FR81 | 16 | Agent contact |
| FR82 | 17 | Agency and realtor management |
| FR83 | 17 | External listing import |
| FR84 | 18 | Airbnb/Booking.com sync |
| FR85 | 18 | Guest registration |
| FR86 | 18 | Guest reports for authorities |
| FR87 | 19 | Tenant screening |
| FR88 | 19 | Lease lifecycle management |
| FR89 | 19 | Lease expiration tracking and reminders |
| FR90 | 9 | GDPR data export |
| FR91 | 9 | GDPR data deletion |
| FR92 | 9 | Privacy settings configuration |
| FR93 | 9 | Compliance audit logs |
| FR94 | 9 | Compliance reports |
| FR95 | 9 | Data retention policy enforcement |
| FR96 | 10B | Platform health metrics |
| FR97 | 10B | Feature flag management |
| FR98 | 10B | System announcements |
| FR99 | 10B | Support data access |
| FR100 | 10B | User onboarding tour |
| FR101 | 10B | Contextual help and documentation |

## Epic List

### Phase 1: MVP (12 Epics)

#### Epic 1: User Authentication & Sessions
**Goal:** Users can register, login, manage sessions, and reset passwords. Token structure supports future organization context.

**FRs covered:** FR1, FR2, FR3, FR5, FR6
**Estimate:** 3 weeks
**Key Decisions:**
- JWT with extension points for org/role claims
- Multi-org session model
- Email template localization (SK, CS, DE, EN)

---

#### Epic 2A: Organizations & Tenant Isolation
**Goal:** Organizations can onboard with complete data isolation. RLS enforcement validated via penetration test framework.

**FRs covered:** FR8, FR9, FR10, FR11, FR13, FR14
**Estimate:** 2 weeks
**Key Decisions:**
- PostgreSQL RLS for tenant isolation
- RLS penetration test framework
- Organization data export capability

---

#### Epic 2B: Notification Infrastructure & Offline Foundation
**Goal:** Push, email, and in-app notifications work reliably. Event bus established. Offline sync patterns defined.

**FRs covered:** FR26, FR27 (basic), FR28
**Estimate:** 3 weeks
**Key Decisions:**
- Redis pub/sub event bus
- FCM/APNs push infrastructure
- Privacy-aware notification design
- Idempotency patterns for offline sync
- Conflict resolution strategy

---

#### Epic 3: Property & Building Management
**Goal:** Managers can create buildings and units with schema ready for Reality Portal. Residents associated with units.

**FRs covered:** FR15, FR16, FR17, FR18, FR19, FR20, FR21
**Estimate:** 2 weeks
**Key Decisions:**
- Nullable listing fields for Phase 4 readiness
- Public/private access patterns in schema
- No feature flags needed (just don't build listing UI)

---

#### Epic 4: Fault Reporting & Resolution
**Goal:** Tenants report faults with photos (works offline). Managers triage and resolve. AI training metadata captured.

**FRs covered:** FR30, FR31, FR32, FR33, FR34, FR35, FR36
**Estimate:** 3 weeks
**Key Decisions:**
- Offline: create fault + queue photos
- AI metadata columns (ai_category, confidence)
- Photo-first UX per UX specification

---

#### Epic 5: Building Voting & Decisions
**Goal:** Owners participate in remote voting with full audit trail. Online-only for integrity.

**FRs covered:** FR37, FR38, FR39, FR40, FR41, FR42, FR43, FR44
**Estimate:** 2 weeks
**Key Decisions:**
- **Online-only voting** (no offline to prevent conflicts)
- View cached votes offline
- Immutable audit log
- Delegated voting support

---

#### Epic 6: Announcements & Communication ⚠️
**Goal:** Managers broadcast announcements. Residents comment and interact. AI consent captured.

**FRs covered:** FR22, FR23, FR24, FR25, FR29
**Estimate:** 2 weeks
**Priority:** Can slip 2 weeks if needed
**Key Decisions:**
- Offline: view cached, queue comments
- AI consent flag on messages for Phase 3 training

---

#### Epic 7A: Basic Document Management
**Goal:** Users can upload, organize, and share documents with basic functionality.

**FRs covered:** FR45, FR46, FR47, FR50
**Estimate:** 1.5 weeks
**Key Decisions:**
- Basic upload/download/organize
- Permission-based viewing
- OCR and search deferred to Epic 7B

---

#### Epic 8A: Basic Notification Preferences
**Goal:** Users can toggle notifications on/off per channel.

**FRs covered:** FR27 (basic toggles only)
**Estimate:** 1 week
**Key Decisions:**
- Simple on/off per channel (push, email, in-app)
- Granular preferences deferred to Epic 8B

---

#### Epic 9: Privacy, Security & GDPR
**Goal:** Users can enable 2FA, export/delete data, and configure privacy. Audit logs available.

**FRs covered:** FR4, FR90, FR91, FR92, FR93, FR94, FR95
**Estimate:** 3 weeks
**Key Decisions:**
- TOTP-based 2FA (simpler, implement first)
- GDPR export/deletion (more complex, implement second)
- Audit trail for compliance operations

---

#### Epic 10A: OAuth Provider Foundation
**Goal:** OAuth 2.0 provider established on api-server for future SSO.

**FRs covered:** FR7 (partial - provider only)
**Estimate:** 1.5 weeks
**Key Decisions:**
- Build OAuth provider infrastructure
- SSO consumer on reality-server deferred to Phase 4
- No end-user UI in MVP

---

#### Epic 10B: Platform Administration
**Goal:** Super admins can manage organizations, feature flags, and platform health.

**FRs covered:** FR12, FR96, FR97, FR98, FR99, FR100, FR101
**Estimate:** 2 weeks
**Key Decisions:**
- Admin dashboard for org oversight
- Feature flag management
- Health metrics and monitoring
- User onboarding tour

---

### Phase 2: Financial & Enhanced Features (4 Epics)

#### Epic 11: Financial Management & Payments
**Goal:** Owners view balances. Managers track payments and manage funds.

**FRs covered:** FR52, FR53, FR54, FR55, FR56, FR57
**Estimate:** 3 weeks

---

#### Epic 12: Meter Readings & Utilities
**Goal:** Residents submit readings with OCR. Manual corrections stored as training data.

**FRs covered:** FR58, FR59, FR60, FR61, FR62, FR63
**Estimate:** 2.5 weeks
**Key Decisions:**
- OCR training pairs captured
- Anomaly detection

---

#### Epic 7B: Advanced Document Features
**Goal:** Full-text search, OCR extraction, and version history for documents.

**FRs covered:** FR48, FR49, FR51
**Estimate:** 2 weeks
**Key Decisions:**
- OCR text extraction
- Content indexed for AI/RAG
- Version history UI

---

#### Epic 8B: Granular Notification Preferences
**Goal:** Users configure per-category settings, quiet hours, and digest emails.

**FRs covered:** FR27 (advanced)
**Estimate:** 2 weeks

---

### Phase 3: Modern Technology (2 Epics)

#### Epic 13: AI Assistant & Automation
**Goal:** Users interact with AI chatbot. Managers configure workflow automations.

**FRs covered:** FR64, FR65, FR66, FR67, FR68, FR69, FR70
**Estimate:** 4 weeks
**Key Decisions:**
- Chat UI included
- RAG on documents (from Epic 7B)
- Sentiment analysis on messages (from Epic 6)
- Training data from Epics 4, 6, 12

---

#### Epic 14: IoT & Smart Building
**Goal:** Users view sensor data dashboards and receive threshold alerts.

**FRs covered:** FR71, FR72, FR73, FR74, FR75
**Estimate:** 3 weeks

---

### Phase 4: Reality Portal & Rental (6 Epics)

#### Epic 10A-SSO: Cross-Platform SSO Consumer
**Goal:** Complete SSO between PM and Reality Portal.

**FRs covered:** FR7 (complete)
**Estimate:** 1.5 weeks
**Key Decisions:**
- OIDC consumer on reality-server
- Mobile deep-link token sharing

---

#### Epic 15: Property Listings & Multi-Portal Sync
**Goal:** Owners/Realtors create listings from unit data and publish to multiple portals.

**FRs covered:** FR76, FR77, FR78
**Estimate:** 3 weeks

---

#### Epic 16: Portal Search & Discovery
**Goal:** Portal users search, filter, and save favorite listings.

**FRs covered:** FR79, FR80, FR81
**Estimate:** 2.5 weeks
**Key Decisions:**
- SSR/SSG for SEO
- Elasticsearch for search

---

#### Epic 17: Agency & Realtor Management
**Goal:** Agencies manage realtors and shared listings.

**FRs covered:** FR82, FR83
**Estimate:** 2 weeks

---

#### Epic 18: Short-Term Rental Integration
**Goal:** Property managers sync with Airbnb/Booking.com and register guests.

**FRs covered:** FR84, FR85, FR86
**Estimate:** 2.5 weeks

---

#### Epic 19: Lease Management & Tenant Screening
**Goal:** Landlords screen tenants and manage full lease lifecycle.

**FRs covered:** FR87, FR88, FR89
**Estimate:** 2.5 weeks

---

## Cross-Cutting Concerns

| Concern | Epic | Implementation |
|---------|------|----------------|
| JWT Token Foundation | 1 | Extension points for org/role claims |
| Multi-Org Sessions | 1 | User can belong to multiple orgs |
| Tenant Isolation (RLS) | 2A | Penetration test framework |
| Event Bus | 2B | Redis pub/sub for async operations |
| Notification Service | 2B | Push (FCM/APNs), email, in-app |
| Privacy-Aware Design | 2B | Notifications respect privacy settings |
| Offline Architecture | 2B | Patterns, sync queue, conflict resolution |
| Idempotency | 2B | Duplicate request handling for sync |
| Reality Portal Schema | 3 | Nullable listing fields for Phase 4 |
| Public Access Patterns | 3 | Schema ready for anonymous queries |
| AI Training Data | 4, 6, 12 | Metadata, consent flags, corrections |
| Online-Only Critical Actions | 5 | Voting requires connectivity |
| i18n Email Templates | 1 | SK, CS, DE, EN from day one |

## Summary

| Phase | Epics | FRs | Weeks (Parallel) |
|-------|-------|-----|------------------|
| MVP | 12 | 63 | ~16 weeks |
| Phase 2 | 4 | 12+ | ~5 weeks |
| Phase 3 | 2 | 12 | ~4 weeks |
| Phase 4 | 6 | 14 | ~8 weeks |
| **Total** | **24** | **101** | **~33 weeks** |

## MVP Critical Path

```
Week 1-3:   Epic 1 (Auth)
Week 2-4:   Epic 2A (Orgs) ──► Week 5-6: Epic 3 (Buildings)
Week 2-5:   Epic 2B (Notifications) [parallel]
Week 7-9:   Epic 4 (Faults) ──► Week 10-11: Epic 5 (Voting)
Week 10-11: Epic 6 (Announcements) [parallel, can slip]
Week 12:    Epic 7A (Docs) + Epic 8A (Prefs)
Week 13-15: Epic 9 (Privacy/2FA/GDPR)
Week 14-16: Epic 10A (OAuth) + Epic 10B (Admin)
```

**Critical Path:** 1 → 2A → 3 → 4 → 5 → 9 → 10B = **15 weeks minimum**

---

# Detailed Epic Stories

## Epic 1: User Authentication & Sessions

**Goal:** Users can register, login, manage sessions, and reset passwords. Token structure supports future organization context.

### Story 1.1: User Registration with Email Verification

As a **new user**,
I want to **create an account with my email address**,
So that **I can access the Property Management System**.

**Acceptance Criteria:**

**Given** a user is on the registration page
**When** they enter a valid email, password (min 8 chars, 1 uppercase, 1 number), and name
**Then** the system creates a pending user record
**And** sends a verification email with a unique token (valid 24 hours)
**And** displays "Check your email to verify your account"

**Given** a user clicks the verification link in the email
**When** the token is valid and not expired
**Then** the user account is activated
**And** the user is redirected to login page with success message

**Given** a user tries to register with an existing email
**When** they submit the registration form
**Then** the system displays "An account with this email already exists"
**And** does not reveal whether the account is verified or not

**Technical Notes:**
- Create `users` table with columns: id, email, password_hash, name, email_verified_at, created_at
- Use Argon2id for password hashing (NFR11)
- Email templates in SK, CS, DE, EN based on browser locale

---

### Story 1.2: Email/Password Login

As a **registered user**,
I want to **log in with my email and password**,
So that **I can access my account securely**.

**Acceptance Criteria:**

**Given** a verified user enters correct email and password
**When** they submit the login form
**Then** the system issues a JWT access token (15-minute expiry)
**And** issues a refresh token (7-day expiry, stored in HttpOnly cookie)
**And** redirects to the dashboard

**Given** a user enters incorrect credentials
**When** they submit the login form
**Then** the system displays "Invalid email or password"
**And** increments failed login counter
**And** after 5 failures, enforces a 15-minute lockout

**Given** an unverified user tries to log in
**When** they submit correct credentials
**Then** the system displays "Please verify your email first"
**And** offers to resend verification email

**Technical Notes:**
- JWT includes: user_id, email, iat, exp
- JWT extension point: `org_id` and `roles` claims (nullable for now)
- Rate limit: 10 login attempts per minute per IP (NFR16)

---

### Story 1.3: JWT Token Refresh

As an **authenticated user**,
I want to **stay logged in without re-entering credentials**,
So that **my session persists seamlessly**.

**Acceptance Criteria:**

**Given** a user has a valid refresh token
**When** their access token expires
**Then** the client automatically requests a new access token
**And** receives a new access token without user interaction

**Given** a user's refresh token is expired or invalid
**When** the client attempts to refresh
**Then** the system returns 401 Unauthorized
**And** the client redirects to login page

**Given** a refresh token is used
**When** a new access token is issued
**Then** the refresh token is rotated (old one invalidated)
**And** the new refresh token is set in HttpOnly cookie

**Technical Notes:**
- Create `refresh_tokens` table: id, user_id, token_hash, expires_at, revoked_at
- Token rotation prevents replay attacks

---

### Story 1.4: Password Reset Flow

As a **user who forgot my password**,
I want to **reset my password via email**,
So that **I can regain access to my account**.

**Acceptance Criteria:**

**Given** a user requests a password reset with a registered email
**When** they submit the request
**Then** the system sends a reset email with a unique token (valid 1 hour)
**And** displays "If an account exists, you'll receive reset instructions"

**Given** a user clicks a valid reset link
**When** they enter a new password meeting requirements
**Then** the password is updated
**And** all existing sessions are invalidated
**And** the user is redirected to login with success message

**Given** a user tries to use an expired or invalid reset token
**When** they submit the reset form
**Then** the system displays "This reset link has expired"
**And** offers to request a new reset email

**Technical Notes:**
- Create `password_reset_tokens` table: id, user_id, token_hash, expires_at, used_at
- Invalidate all refresh_tokens for user on password change

---

### Story 1.5: Active Session Management

As a **security-conscious user**,
I want to **view and revoke my active sessions**,
So that **I can protect my account from unauthorized access**.

**Acceptance Criteria:**

**Given** a user navigates to account security settings
**When** they view active sessions
**Then** they see a list of sessions with: device type, browser, IP, location (approximate), last active, current session indicator

**Given** a user clicks "Revoke" on a session
**When** the action is confirmed
**Then** that session's refresh token is invalidated
**And** any active requests from that session receive 401

**Given** a user clicks "Revoke all other sessions"
**When** the action is confirmed
**Then** all sessions except current are invalidated
**And** the user sees confirmation message

**Technical Notes:**
- Extend `refresh_tokens` with: user_agent, ip_address, created_at
- Use IP geolocation service for approximate location
- Multi-org support: sessions span organizations (user belongs to multiple)

---

### Story 1.6: User Lifecycle Management (Admin)

As a **platform operator**,
I want to **invite, suspend, and delete users**,
So that **I can manage user access to the platform**.

**Acceptance Criteria:**

**Given** an admin enters an email to invite a new user
**When** they submit the invitation
**Then** the system sends an invitation email with setup link
**And** creates a pending user record with `invited_at` timestamp

**Given** an admin suspends a user account
**When** the suspension is applied
**Then** all user sessions are immediately invalidated
**And** the user cannot log in
**And** the user sees "Account suspended. Contact support." on login attempt

**Given** an admin deletes a user account
**When** the deletion is confirmed (requires typing user email)
**Then** the user record is soft-deleted (retained per GDPR)
**And** all sessions are invalidated
**And** the email cannot be reused for 30 days

**Technical Notes:**
- Add to `users`: status (active, suspended, deleted), invited_at, suspended_at, deleted_at
- Soft delete with data retention per NFR25

---

### Story 1.7: Localized Email Templates

As a **user who prefers their native language**,
I want to **receive authentication emails in my language**,
So that **I can understand important account communications**.

**Acceptance Criteria:**

**Given** a user registers with browser locale set to Slovak
**When** the verification email is sent
**Then** the email is rendered in Slovak language
**And** uses Slovak date/time formatting

**Given** a user's browser locale is Czech, German, or English
**When** any authentication email is sent
**Then** the email is rendered in the detected language

**Given** a user's locale is not supported (e.g., French)
**When** an email is sent
**Then** the email defaults to English

**Technical Notes:**
- Create email templates: verification, password_reset, invitation, session_alert
- Each template in: sk, cs, de, en
- Store user's preferred_locale in users table (nullable, falls back to browser)

---

## Epic 2A: Organizations & Tenant Isolation

**Goal:** Organizations can onboard with complete data isolation. RLS enforcement validated via penetration test framework.

### Story 2A.1: Organization Creation

As a **platform administrator**,
I want to **create a new organization**,
So that **a group of users can manage their properties in isolation**.

**Acceptance Criteria:**

**Given** an admin submits organization creation form
**When** they provide name, slug (unique), and contact email
**Then** the system creates the organization record
**And** creates the organization owner user (or links existing user)
**And** the new org is completely isolated from other orgs

**Given** an organization slug is already taken
**When** the admin tries to create the org
**Then** the system displays "This URL slug is already in use"

**Technical Notes:**
- Create `organizations` table: id, name, slug, contact_email, created_at, status
- All tenant-scoped tables will have `organization_id` foreign key
- Enable PostgreSQL RLS on all tenant tables

---

### Story 2A.2: Row-Level Security Enforcement

As a **security architect**,
I want to **enforce tenant isolation at the database level**,
So that **data leakage between organizations is impossible**.

**Acceptance Criteria:**

**Given** RLS policies are applied to all tenant-scoped tables
**When** any query is executed without valid tenant context
**Then** the query returns zero rows (not an error)

**Given** a user with organization A context
**When** they query any tenant-scoped table
**Then** they only see rows where organization_id matches their context

**Given** a super admin context
**When** they query tenant-scoped tables
**Then** they can see all rows (bypasses RLS for admin functions)

**Technical Notes:**
- Create RLS policies using `current_setting('app.current_org_id')`
- Set tenant context in Axum middleware from JWT org_id claim
- Create `set_tenant_context(org_id)` function

---

### Story 2A.3: RLS Penetration Test Framework

As a **security team member**,
I want to **automated tests that verify tenant isolation**,
So that **we catch RLS misconfigurations before production**.

**Acceptance Criteria:**

**Given** the test suite runs with two test organizations
**When** Organization A's context is set
**Then** queries to all tenant tables return only Org A data

**Given** the test creates a record in Organization A
**When** Organization B's context is set and queries the table
**Then** the record is not visible

**Given** a new table is added without RLS policy
**When** the test suite runs
**Then** it fails with "Table X missing RLS policy"

**Technical Notes:**
- Integration tests using sqlx test fixtures
- Test helper: `with_tenant_context(org_id, async_fn)`
- CI gate: all new tables must have RLS test coverage

---

### Story 2A.4: Organization Settings & Branding

As an **organization administrator**,
I want to **configure my organization's settings and branding**,
So that **the platform reflects our identity**.

**Acceptance Criteria:**

**Given** an org admin navigates to organization settings
**When** they update name, logo, primary color
**Then** the settings are saved
**And** the branding is applied to the organization's interface

**Given** an org admin uploads a logo
**When** the file is a valid image (PNG, JPG, max 2MB)
**Then** the logo is stored in S3 and URL saved
**And** the logo appears in the org's header

**Technical Notes:**
- Add to `organizations`: logo_url, primary_color, settings (JSONB)
- S3 upload with presigned URLs
- Settings JSONB for future extensibility

---

### Story 2A.5: Organization Member Management

As an **organization administrator**,
I want to **add and remove members from my organization**,
So that **the right people have access**.

**Acceptance Criteria:**

**Given** an org admin invites a user by email
**When** the user doesn't exist
**Then** an invitation email is sent with org context
**And** a pending membership record is created

**Given** an org admin invites an existing platform user
**When** the invitation is sent
**Then** the user receives a "Join Organization" email
**And** upon acceptance, they're added to the org

**Given** an org admin removes a member
**When** the removal is confirmed
**Then** the membership is revoked
**And** the user's sessions for that org are invalidated

**Technical Notes:**
- Create `organization_members` table: id, organization_id, user_id, role, joined_at
- Users can belong to multiple organizations

---

### Story 2A.6: Role-Based Access Control (RBAC)

As an **organization administrator**,
I want to **assign roles with specific permissions**,
So that **users only access what they're authorized for**.

**Acceptance Criteria:**

**Given** an org admin assigns "Manager" role to a user
**When** that user logs in
**Then** their JWT includes the role in the org context
**And** they can perform manager-level actions

**Given** a user with "Owner" role tries to access manager settings
**When** the permission check is performed
**Then** the action is denied with 403 Forbidden

**Given** an org defines custom permissions for a role
**When** a user with that role attempts an action
**Then** the permission is checked against the custom configuration

**Technical Notes:**
- Create `roles` table: id, organization_id, name, permissions (JSONB)
- Default roles: Owner, Manager, Technical Manager, Tenant, Resident
- Permission format: `resource:action` (e.g., `faults:create`, `votes:manage`)

---

### Story 2A.7: Organization Data Export

As an **organization administrator**,
I want to **export all my organization's data**,
So that **I can migrate to another system or have a backup**.

**Acceptance Criteria:**

**Given** an org admin requests a full data export
**When** the export is initiated
**Then** a background job collects all org data
**And** generates a ZIP file with JSON/CSV files per table
**And** sends a download link via email when complete

**Given** the export is ready
**When** the admin clicks the download link (valid 7 days)
**Then** the ZIP file downloads
**And** contains: users, buildings, units, faults, votes, documents, etc.

**Technical Notes:**
- Async job using tokio background tasks
- Export to S3 presigned URL
- Respects GDPR: user data included only with consent

---

## Epic 2B: Notification Infrastructure & Offline Foundation

**Goal:** Push, email, and in-app notifications work reliably. Event bus established. Offline sync patterns defined.

### Story 2B.1: Event Bus Foundation

As a **system architect**,
I want to **establish an event-driven architecture**,
So that **services can communicate asynchronously**.

**Acceptance Criteria:**

**Given** an event is published (e.g., `fault.created`)
**When** subscribers are registered for that event
**Then** all subscribers receive the event payload

**Given** multiple api-server instances are running
**When** an event is published on one instance
**Then** all instances receive the event (via Redis pub/sub)

**Given** an event fails to process
**When** the subscriber throws an error
**Then** the event is retried up to 3 times with exponential backoff

**Technical Notes:**
- Redis pub/sub for cross-instance messaging
- Event schema: `{ event_type, payload, org_id, user_id, timestamp }`
- Create `EventBus` trait with `publish()` and `subscribe()` methods

---

### Story 2B.2: Push Notification Service (FCM/APNs)

As a **mobile user**,
I want to **receive push notifications on my device**,
So that **I'm alerted to important events in real-time**.

**Acceptance Criteria:**

**Given** a user has registered their device token
**When** an event relevant to them occurs
**Then** a push notification is sent to their device within 5 seconds (NFR7)

**Given** a user has multiple devices registered
**When** a notification is sent
**Then** all devices receive the notification
**And** tapping the notification deep-links to the relevant screen

**Given** a device token is invalid (uninstalled app)
**When** a push fails
**Then** the device token is marked as invalid
**And** removed from future notifications

**Technical Notes:**
- Create `device_tokens` table: id, user_id, token, platform (ios/android), created_at
- FCM for Android, APNs for iOS
- Batch notifications for efficiency

---

### Story 2B.3: Email Notification Service

As an **offline user**,
I want to **receive important notifications via email**,
So that **I don't miss critical updates**.

**Acceptance Criteria:**

**Given** a user has email notifications enabled
**When** they haven't been active for 24 hours and have pending notifications
**Then** an email digest is sent with summary of notifications

**Given** a high-priority event occurs (e.g., vote deadline)
**When** the event is triggered
**Then** an immediate email is sent regardless of digest settings

**Given** an email fails to send
**When** the delivery fails
**Then** the system retries up to 3 times
**And** logs the failure for monitoring

**Technical Notes:**
- Integration with email service (SendGrid/SES)
- Email templates: immediate alerts, daily digest
- Bounce handling and unsubscribe links

---

### Story 2B.4: In-App Notification Center

As an **active user**,
I want to **see my notifications in the app**,
So that **I can review and act on them**.

**Acceptance Criteria:**

**Given** a user logs into the app
**When** they view the notification center
**Then** they see a chronological list of notifications
**And** unread notifications are highlighted
**And** each notification links to its context

**Given** a user marks a notification as read
**When** the action is performed
**Then** the notification is no longer highlighted
**And** the unread count decrements

**Given** new notifications arrive while the user is active
**When** WebSocket connection is open
**Then** notifications appear in real-time without refresh

**Technical Notes:**
- Create `notifications` table: id, user_id, type, title, body, data (JSONB), read_at
- WebSocket channel per user for real-time updates
- Pagination for notification history

---

### Story 2B.5: Privacy-Aware Notification Design

As a **privacy-conscious user**,
I want to **notifications to respect my privacy settings**,
So that **sensitive information isn't exposed**.

**Acceptance Criteria:**

**Given** a user has privacy mode enabled
**When** a push notification is sent
**Then** the notification shows generic text ("New update available")
**And** doesn't expose content in lock screen

**Given** a user's neighbor has hidden their profile
**When** a notification involves that neighbor
**Then** the name is anonymized ("A neighbor reported...")

**Given** notification content contains PII
**When** the notification is rendered
**Then** PII is only shown within the authenticated app context

**Technical Notes:**
- Notification payload: `{ preview: "generic", full: "detailed" }`
- Push shows preview only; in-app shows full when authenticated
- Respect user's privacy_settings from their profile

---

### Story 2B.6: Offline Sync Queue Foundation

As a **mobile user in a low-connectivity area**,
I want to **queue my actions when offline**,
So that **they sync when I'm back online**.

**Acceptance Criteria:**

**Given** a user performs an action while offline
**When** the device has no network connection
**Then** the action is stored in local queue with timestamp
**And** the UI shows "Pending sync" indicator

**Given** the device regains connectivity
**When** the sync process runs
**Then** queued actions are sent to server in order
**And** successful syncs update local state
**And** the user sees "Synced" confirmation

**Given** a sync conflict occurs (server state changed)
**When** the conflict is detected
**Then** the user is prompted to resolve (keep local/server/merge)

**Technical Notes:**
- Client-side: AsyncStorage queue with retry logic
- Server-side: Idempotency keys to prevent duplicates
- Conflict resolution strategy documented per entity type

---

### Story 2B.7: Idempotency Pattern for API Operations

As a **backend developer**,
I want to **handle duplicate requests gracefully**,
So that **offline sync doesn't create duplicates**.

**Acceptance Criteria:**

**Given** a client sends a request with an idempotency key
**When** the same key is sent again
**Then** the server returns the cached response
**And** doesn't execute the operation twice

**Given** an idempotency key is older than 24 hours
**When** it's sent again
**Then** the key is treated as new (cache expired)

**Given** a request fails midway (network error)
**When** the client retries with the same idempotency key
**Then** the original operation is either completed or rolled back atomically

**Technical Notes:**
- Create `idempotency_keys` table: key, response, status, created_at
- Redis cache for fast lookups (TTL 24h)
- Idempotency header: `Idempotency-Key: uuid`

---

## Epic 3: Property & Building Management

**Goal:** Managers can create buildings and units with schema ready for Reality Portal. Residents associated with units.

### Story 3.1: Building Creation & Configuration

As a **property manager**,
I want to **create and configure buildings**,
So that **I can manage properties for my organization**.

**Acceptance Criteria:**

**Given** a manager navigates to "Add Building"
**When** they enter address, name, and building type
**Then** the building is created in the organization
**And** appears in the building list

**Given** a manager edits a building
**When** they update metadata (year built, floors, entrances)
**Then** the changes are saved
**And** reflected in the building detail view

**Given** a building has a valid address
**When** the address is saved
**Then** the system geocodes and stores coordinates
**And** the building can be displayed on a map

**Technical Notes:**
- Create `buildings` table: id, organization_id, name, address, coordinates, metadata (JSONB), created_at
- Include Reality Portal-ready fields: listing_status (nullable), public_description (nullable)
- Geocoding via external API (Google/Mapbox)

---

### Story 3.2: Unit Definition & Management

As a **property manager**,
I want to **define units within a building**,
So that **I can track individual apartments or spaces**.

**Acceptance Criteria:**

**Given** a manager is on a building detail page
**When** they add a unit with number, floor, and type (apartment, office, storage)
**Then** the unit is created and linked to the building

**Given** a manager configures a unit
**When** they set area (m²), room count, and ownership type (owner/rental)
**Then** the configuration is saved
**And** affects fee allocation calculations

**Given** multiple units exist in a building
**When** a manager views the unit list
**Then** units are sorted by floor and number
**And** show occupancy status (vacant, occupied, owner-occupied)

**Technical Notes:**
- Create `units` table: id, building_id, number, floor, type, area_m2, ownership_type, status
- Reality Portal fields: listing_metadata (JSONB, nullable), energy_certificate_id (FK, nullable)
- Schema supports future Phase 4 without migration

---

### Story 3.3: Resident Association with Units

As a **property manager**,
I want to **associate residents with units**,
So that **the right people receive relevant communications**.

**Acceptance Criteria:**

**Given** a manager adds a resident to a unit
**When** they specify the resident type (owner, tenant, family member)
**Then** the association is created
**And** the resident can access unit-specific content

**Given** a unit has multiple residents
**When** a manager views residents
**Then** they see all associated people with their roles
**And** can manage each individually

**Given** a resident moves out
**When** a manager ends their association
**Then** the end date is recorded
**And** the resident loses access to unit content
**And** historical records are preserved

**Technical Notes:**
- Create `unit_residents` table: id, unit_id, user_id, resident_type, start_date, end_date
- Support multiple residents per unit
- Historical associations for audit

---

### Story 3.4: Ownership Delegation

As a **property owner**,
I want to **delegate my rights to another person**,
So that **they can act on my behalf**.

**Acceptance Criteria:**

**Given** an owner initiates delegation
**When** they specify the delegate and scope (full, voting only, documents only)
**Then** an invitation is sent to the delegate

**Given** a delegate accepts the invitation
**When** they confirm the delegation
**Then** they gain the specified permissions
**And** can act on behalf of the owner

**Given** an owner revokes a delegation
**When** the revocation is processed
**Then** the delegate immediately loses access
**And** the revocation is logged for audit

**Technical Notes:**
- Create `delegations` table: id, owner_user_id, delegate_user_id, unit_id, scope, start_date, end_date
- Scope: all, voting, documents, faults
- Delegation appears in owner's and delegate's profiles

---

### Story 3.5: Person-Month Tracking

As a **property manager**,
I want to **track person-months for each unit**,
So that **fees can be allocated fairly based on occupancy**.

**Acceptance Criteria:**

**Given** a manager enters person-month data for a unit
**When** they specify month, year, and count of residents
**Then** the data is saved
**And** contributes to fee allocation calculations

**Given** a manager views person-month history
**When** they select a date range
**Then** they see monthly breakdown per unit
**And** totals for the building

**Given** the system needs to suggest person-months
**When** resident associations exist
**Then** the system pre-populates based on registered residents
**And** allows manual override

**Technical Notes:**
- Create `person_months` table: id, unit_id, year, month, count, source (manual/calculated)
- Aggregate view for building-level reports
- Supports historical queries

---

### Story 3.6: Unit Detail View for Residents

As a **resident**,
I want to **view my unit's details**,
So that **I can see relevant information about my home**.

**Acceptance Criteria:**

**Given** a resident logs in
**When** they navigate to "My Unit"
**Then** they see unit details (address, floor, area)
**And** see their association status (owner/tenant)

**Given** a resident has access to multiple units
**When** they view the unit selector
**Then** they can switch between units
**And** each unit shows its specific context

**Given** unit information is updated by a manager
**When** the resident views the unit
**Then** they see the current information

**Technical Notes:**
- Read-only view for residents
- Respects privacy settings (don't show other residents unless allowed)
- Mobile-optimized layout

---

### Story 3.7: Common Areas & Facilities

As a **property manager**,
I want to **manage common areas and shared facilities**,
So that **residents can book and use them**.

**Acceptance Criteria:**

**Given** a manager adds a common area
**When** they specify name, type (gym, laundry, meeting room), and rules
**Then** the facility is available for booking/viewing

**Given** a facility requires booking
**When** a resident views available slots
**Then** they see availability calendar
**And** can request a booking

**Given** a facility has usage rules
**When** displayed to residents
**Then** rules are clearly shown
**And** violations can be reported

**Technical Notes:**
- Create `facilities` table: id, building_id, name, type, rules, bookable, capacity
- Create `facility_bookings` table: id, facility_id, user_id, start_time, end_time, status
- Future: integrate with IoT access control

---

## Epic 4: Fault Reporting & Resolution

**Goal:** Tenants report faults with photos (works offline). Managers triage and resolve. AI training metadata captured.

### Story 4.1: Fault Creation with Photos

As a **tenant**,
I want to **report a fault with photos**,
So that **maintenance issues are documented and resolved**.

**Acceptance Criteria:**

**Given** a tenant opens "Report Fault"
**When** they take/select photos and add a description
**Then** the fault is created with pending status
**And** attached photos are uploaded
**And** notification is sent to managers

**Given** a tenant is offline
**When** they create a fault report
**Then** the report is queued locally
**And** syncs when connectivity is restored
**And** shows "Pending sync" indicator

**Given** photos are attached
**When** they exceed 10MB total
**Then** the system compresses images
**And** warns if quality is significantly reduced

**Technical Notes:**
- Create `faults` table: id, unit_id, reporter_id, title, description, status, priority, category, created_at
- Create `fault_attachments` table: id, fault_id, url, type, size
- AI columns: ai_category, ai_priority, ai_confidence (nullable, for Phase 3)
- Offline: use idempotency key from Story 2B.7

---

### Story 4.2: AI-Assisted Category Suggestion

As a **tenant**,
I want to **have fault category suggested automatically**,
So that **I don't have to guess the right classification**.

**Acceptance Criteria:**

**Given** a tenant enters fault description
**When** the description is analyzed
**Then** a category is suggested with confidence level
**And** the tenant can accept or override

**Given** AI confidence is > 90%
**When** the suggestion is made
**Then** the category is pre-selected
**And** highlighted as "AI suggested"

**Given** AI confidence is < 70%
**When** the suggestion is made
**Then** no pre-selection occurs
**And** manual selection is required

**Technical Notes:**
- Categories: plumbing, electrical, heating, structural, exterior, other
- Store ai_category, ai_confidence for training data
- Initial implementation: keyword matching (real ML in Phase 3)

---

### Story 4.3: Fault Triage by Manager

As a **property manager**,
I want to **triage incoming faults**,
So that **they're prioritized and assigned appropriately**.

**Acceptance Criteria:**

**Given** a new fault is reported
**When** a manager views the fault queue
**Then** they see faults sorted by creation date
**And** can filter by building, category, status

**Given** a manager triages a fault
**When** they set priority (low, medium, high, urgent) and category
**Then** the fault is updated
**And** relevant parties are notified

**Given** a manager assigns a fault to technical staff
**When** the assignment is made
**Then** the assignee receives notification
**And** the fault appears in their queue

**Technical Notes:**
- Add to `faults`: priority, assigned_to, triaged_at, triaged_by
- Manager dashboard with queue filters
- Keyboard shortcuts for quick triage (per UX spec)

---

### Story 4.4: Fault Status Workflow

As a **technical manager**,
I want to **update fault status through resolution**,
So that **everyone knows the current state**.

**Acceptance Criteria:**

**Given** a fault is assigned to technical staff
**When** they update status (in_progress, waiting_parts, scheduled, resolved)
**Then** the status changes
**And** timeline entry is created
**And** reporter is notified

**Given** a technician adds a work note
**When** they describe actions taken
**Then** the note is saved to timeline
**And** visible to managers and reporter

**Given** a fault is marked resolved
**When** the resolution is submitted
**Then** reporter is asked to confirm resolution
**And** satisfaction survey is triggered

**Technical Notes:**
- Create `fault_timeline` table: id, fault_id, user_id, action, note, created_at
- Status workflow: new → triaged → in_progress → resolved → closed
- Timeline visible to all stakeholders

---

### Story 4.5: Fault Status Tracking for Residents

As a **tenant**,
I want to **track the status of my reported faults**,
So that **I know when to expect resolution**.

**Acceptance Criteria:**

**Given** a tenant views their reported faults
**When** they access the fault list
**Then** they see all their faults with current status
**And** can tap to view details

**Given** a tenant views fault details
**When** timeline entries exist
**Then** they see chronological progress
**And** understand next expected step

**Given** a fault has an estimated completion date
**When** displayed to the tenant
**Then** the date is shown prominently
**And** updates trigger notifications

**Technical Notes:**
- Timeline view per UX specification
- Push notification on status changes
- "No status black holes" principle

---

### Story 4.6: Fault Resolution & Rating

As a **tenant**,
I want to **confirm fault resolution and provide feedback**,
So that **service quality can be tracked**.

**Acceptance Criteria:**

**Given** a fault is marked resolved
**When** the tenant is prompted to confirm
**Then** they can confirm resolution or reopen

**Given** a tenant confirms resolution
**When** prompted for feedback
**Then** they can rate (1-5 stars) and add comments
**And** the fault is closed

**Given** a tenant reopens a fault
**When** they provide reason
**Then** the fault returns to triaged status
**And** managers are notified

**Technical Notes:**
- Add to `faults`: resolved_at, confirmed_at, rating, feedback
- Reopen creates new timeline entry
- Aggregate ratings for reporting

---

### Story 4.7: Fault Reports & Analytics

As a **property manager**,
I want to **view fault reports and analytics**,
So that **I can identify patterns and improve service**.

**Acceptance Criteria:**

**Given** a manager accesses fault reports
**When** they select a date range and building
**Then** they see: fault count, average resolution time, ratings breakdown

**Given** analytics show category trends
**When** a category has high volume
**Then** it's highlighted for attention
**And** drill-down to individual faults is available

**Given** a manager exports report
**When** they select format (PDF, CSV)
**Then** the report is generated and downloadable

**Technical Notes:**
- Aggregate queries with date/building/category filters
- Charts: bar (by category), line (over time), pie (by status)
- Export includes raw data and summary

---

## Epic 5: Building Voting & Decisions

**Goal:** Owners participate in remote voting with full audit trail. Online-only for integrity.

### Story 5.1: Vote Creation with Question Types

As a **property manager**,
I want to **create votes with different question types**,
So that **owners can make decisions on building matters**.

**Acceptance Criteria:**

**Given** a manager creates a new vote
**When** they specify title, description, and end date
**Then** the vote is created in draft status
**And** questions can be added

**Given** a manager adds questions
**When** they select type (yes/no, multiple choice, ranked)
**Then** the question is configured appropriately
**And** preview shows expected ballot

**Given** a vote is ready
**When** the manager publishes it
**Then** eligible owners are notified
**And** voting period begins

**Technical Notes:**
- Create `votes` table: id, building_id, title, description, start_at, end_at, status, quorum_type
- Create `vote_questions` table: id, vote_id, question_text, type, options (JSONB)
- Question types: yes_no, single_choice, multiple_choice, ranked

---

### Story 5.2: Quorum Configuration

As a **property manager**,
I want to **configure quorum requirements**,
So that **votes are valid according to building bylaws**.

**Acceptance Criteria:**

**Given** a manager configures vote quorum
**When** they set type (simple majority, 2/3, weighted by ownership)
**Then** the quorum is saved
**And** displayed to voters

**Given** voting period ends
**When** quorum is not met
**Then** the vote is marked "Quorum not reached"
**And** results are still calculated but marked as advisory

**Given** weighted voting is enabled
**When** owners vote
**Then** their vote weight reflects ownership share

**Technical Notes:**
- Quorum types: simple_majority, two_thirds, weighted
- Weighted voting uses unit.ownership_share field
- Add ownership_share to units table

---

### Story 5.3: Vote Casting (Online Only)

As a **property owner**,
I want to **cast my vote during the voting period**,
So that **my voice is counted in building decisions**.

**Acceptance Criteria:**

**Given** an owner opens an active vote
**When** they are eligible (owner or delegate)
**Then** they see questions and can select answers

**Given** an owner submits their ballot
**When** they confirm their choices
**Then** the vote is recorded with timestamp
**And** receipt is shown with confirmation number

**Given** an owner is offline
**When** they try to vote
**Then** a message explains voting requires connectivity
**And** offers to save draft locally for later submission

**Technical Notes:**
- Create `vote_responses` table: id, vote_id, user_id, unit_id, answers (JSONB), submitted_at, signature
- **Online-only**: no offline voting to prevent conflicts
- Digital signature for non-repudiation

---

### Story 5.4: Delegated Voting

As a **property owner**,
I want to **delegate my voting rights**,
So that **someone can vote on my behalf when I'm unavailable**.

**Acceptance Criteria:**

**Given** an owner has delegated voting rights
**When** the delegate accesses a vote
**Then** they see voting option for the owner's unit
**And** can vote on behalf of the owner

**Given** both owner and delegate try to vote
**When** the owner votes first
**Then** the delegate is blocked with "Owner already voted"
**And** vice versa

**Given** a delegation is revoked mid-vote
**When** the delegate tries to vote
**Then** they receive "Delegation revoked" error

**Technical Notes:**
- Check delegations table with scope including 'voting'
- One vote per unit per question
- Audit trail shows who cast the vote

---

### Story 5.5: Vote Discussion Threads

As a **building resident**,
I want to **discuss votes with neighbors**,
So that **I can understand different perspectives**.

**Acceptance Criteria:**

**Given** a vote is active or recently closed
**When** a resident opens discussion
**Then** they see existing comments threaded by topic
**And** can add their own comments

**Given** a comment is posted
**When** relevant users have notification enabled
**Then** they receive notification of new comment

**Given** discussion gets heated
**When** a manager moderates
**Then** they can hide or delete comments
**And** warn or mute users

**Technical Notes:**
- Create `vote_comments` table: id, vote_id, user_id, parent_id, content, created_at, hidden
- AI consent flag for future sentiment analysis
- Moderation actions logged

---

### Story 5.6: Vote Results & Quorum Calculation

As a **property manager**,
I want to **see vote results with quorum status**,
So that **I can communicate outcomes to owners**.

**Acceptance Criteria:**

**Given** voting period ends
**When** the system calculates results
**Then** participation rate and quorum status are determined
**And** results per question are calculated

**Given** quorum is reached
**When** results are displayed
**Then** winning options are clearly indicated
**And** vote counts/percentages shown

**Given** results need to be shared
**When** manager generates report
**Then** official document is created with all details

**Technical Notes:**
- Automatic calculation on vote end_at
- Results: { participation_count, quorum_met, questions: [{ counts, winner }] }
- PDF generation for official records

---

### Story 5.7: Immutable Voting Audit Trail

As a **compliance officer**,
I want to **access immutable voting records**,
So that **vote integrity can be verified**.

**Acceptance Criteria:**

**Given** any voting action occurs
**When** the action is completed
**Then** an immutable audit entry is created
**And** includes timestamp, user, action, data hash

**Given** an audit is requested
**When** the officer reviews the trail
**Then** they see complete chronological log
**And** can verify no tampering occurred

**Given** a dispute arises
**When** audit is examined
**Then** original ballot data can be verified against hash

**Technical Notes:**
- Create `vote_audit_log` table: id, vote_id, user_id, action, data_hash, created_at
- Hash: SHA-256 of response data
- Append-only: no updates or deletes allowed
- Regular backup of audit table

---

### Story 5.8: Voting Compliance Reports

As a **property manager**,
I want to **generate compliance reports for votes**,
So that **I can satisfy legal requirements**.

**Acceptance Criteria:**

**Given** a vote is complete
**When** manager generates compliance report
**Then** document includes: vote details, participation, results, audit summary

**Given** report is generated
**When** exported as PDF
**Then** it includes digital timestamp
**And** can be officially submitted

**Given** multiple votes need reporting
**When** manager selects date range
**Then** summary report covers all included votes

**Technical Notes:**
- PDF generation with official formatting
- Digital timestamp via trusted service
- Archive reports in documents system

---

## Epic 6: Announcements & Communication

**Goal:** Managers broadcast announcements. Residents comment and interact. AI consent captured.

### Story 6.1: Announcement Creation & Targeting

As a **property manager**,
I want to **create announcements for specific audiences**,
So that **relevant information reaches the right people**.

**Acceptance Criteria:**

**Given** a manager creates an announcement
**When** they specify title, content, and target (all, building, specific units)
**Then** the announcement is created
**And** targeted users are notified

**Given** an announcement is scheduled for future
**When** the scheduled time arrives
**Then** the announcement is published automatically
**And** notifications are sent

**Given** rich content is needed
**When** manager uses editor
**Then** markdown formatting is supported
**And** images can be embedded

**Technical Notes:**
- Create `announcements` table: id, organization_id, title, content, target_type, target_ids, published_at, scheduled_at
- Target types: all, building, units, roles
- Markdown with sanitization

---

### Story 6.2: Announcement Viewing & Acknowledgment

As a **resident**,
I want to **view and acknowledge announcements**,
So that **I stay informed and managers know I've read them**.

**Acceptance Criteria:**

**Given** a resident has unread announcements
**When** they open the app
**Then** unread count is displayed
**And** announcements are accessible from dashboard

**Given** a resident views an announcement
**When** they scroll to the end
**Then** the announcement is marked as read
**And** read status is recorded

**Given** an announcement requires acknowledgment
**When** the resident clicks "Acknowledge"
**Then** their acknowledgment is recorded with timestamp
**And** managers can see acknowledgment status

**Technical Notes:**
- Create `announcement_reads` table: id, announcement_id, user_id, read_at, acknowledged_at
- Unread count in notification badge
- Acknowledgment required flag on announcement

---

### Story 6.3: Announcement Comments & Discussion

As a **resident**,
I want to **comment on announcements**,
So that **I can ask questions or provide feedback**.

**Acceptance Criteria:**

**Given** comments are enabled on an announcement
**When** a resident adds a comment
**Then** the comment is posted
**And** author and announcement creator are notified

**Given** a comment exists
**When** another resident replies
**Then** a threaded reply is created
**And** parent comment author is notified

**Given** comments are disabled
**When** a resident views the announcement
**Then** no comment input is shown
**And** reason is displayed ("Comments closed")

**Technical Notes:**
- Create `announcement_comments` table: id, announcement_id, user_id, parent_id, content, created_at
- AI consent flag: ai_training_consent (boolean)
- Moderation by managers

---

### Story 6.4: Pinned Announcements

As a **property manager**,
I want to **pin important announcements**,
So that **they remain visible at the top**.

**Acceptance Criteria:**

**Given** a manager pins an announcement
**When** residents view the announcement list
**Then** pinned items appear at the top
**And** are visually distinguished

**Given** multiple announcements are pinned
**When** displayed
**Then** they're sorted by pin date
**And** limit of 3 pinned per building enforced

**Given** a manager unpins an announcement
**When** the action is taken
**Then** it returns to chronological order
**And** changes immediately visible

**Technical Notes:**
- Add to `announcements`: pinned_at, pinned_by
- Client-side: pinned section + chronological section
- Auto-unpin after 30 days (configurable)

---

### Story 6.5: Direct Messaging

As a **resident**,
I want to **send messages to other users**,
So that **I can communicate privately about building matters**.

**Acceptance Criteria:**

**Given** a resident starts a conversation
**When** they select a recipient within their org
**Then** a direct message thread is created
**And** recipient is notified

**Given** messages are exchanged
**When** either party views the thread
**Then** messages appear in chronological order
**And** read receipts show when messages were seen

**Given** a user wants privacy
**When** they block another user
**Then** blocked user cannot send messages
**And** blocker doesn't see messages from blocked user

**Technical Notes:**
- Create `messages` table: id, sender_id, recipient_id, thread_id, content, read_at, created_at
- Create `message_threads` table: id, participant_ids, last_message_at
- Respect privacy settings from user profile

---

### Story 6.6: Neighbor Information (Privacy-Aware)

As a **resident**,
I want to **see information about my neighbors**,
So that **I can connect with my community**.

**Acceptance Criteria:**

**Given** a resident views neighbor list
**When** neighbors have public profiles
**Then** name and unit number are displayed
**And** contact options if enabled

**Given** a neighbor has hidden their profile
**When** their entry would be displayed
**Then** they appear as "Resident of Unit X"
**And** no contact information shown

**Given** a resident updates their visibility
**When** they toggle profile visibility
**Then** their display to neighbors updates immediately

**Technical Notes:**
- Neighbor = same building, different unit
- Privacy settings: visible, hidden, contacts_only
- Query respects privacy settings via view or application logic

---

## Epic 7A: Basic Document Management

**Goal:** Users can upload, organize, and share documents with basic functionality.

### Story 7A.1: Document Upload with Metadata

As a **user**,
I want to **upload documents with metadata**,
So that **important files are stored and categorized**.

**Acceptance Criteria:**

**Given** a user uploads a document
**When** they select a file and add title, category
**Then** the document is uploaded to storage
**And** metadata is saved
**And** the document appears in the list

**Given** a document exceeds size limit (50MB)
**When** upload is attempted
**Then** user sees error with size limit info
**And** upload is rejected

**Given** an unsupported file type is uploaded
**When** the file is submitted
**Then** user sees list of supported formats
**And** upload is rejected

**Technical Notes:**
- Create `documents` table: id, organization_id, folder_id, title, description, category, file_url, mime_type, size_bytes, created_by, created_at
- S3 presigned URLs for upload
- Supported: PDF, DOC(X), XLS(X), images, TXT

---

### Story 7A.2: Folder Organization

As a **property manager**,
I want to **organize documents in folders**,
So that **files are easy to find**.

**Acceptance Criteria:**

**Given** a manager creates a folder
**When** they specify name and parent folder
**Then** the folder is created
**And** appears in the folder tree

**Given** a manager moves a document to a folder
**When** the move is confirmed
**Then** the document's folder reference is updated
**And** it appears in the new location

**Given** a folder is deleted
**When** it contains documents
**Then** the user is warned
**And** must choose: move contents or delete all

**Technical Notes:**
- Create `document_folders` table: id, organization_id, parent_id, name, created_by
- Nested folders with max depth of 5
- Soft delete with recovery option

---

### Story 7A.3: Permission-Based Document Access

As a **user**,
I want to **view documents based on my permissions**,
So that **I only see what I'm authorized to access**.

**Acceptance Criteria:**

**Given** a document is uploaded with "Building X" scope
**When** a resident of Building X views documents
**Then** they can see and download the document

**Given** a document has "Owners only" permission
**When** a tenant tries to view it
**Then** the document is not visible in their list

**Given** a document is shared with specific users
**When** those users access documents
**Then** the shared document appears in their list

**Technical Notes:**
- Document access scopes: organization, building, unit, role, specific_users
- Permission check middleware
- Share via link with expiration

---

### Story 7A.4: Document Download & Preview

As a **user**,
I want to **preview and download documents**,
So that **I can view content without leaving the app**.

**Acceptance Criteria:**

**Given** a user opens a PDF document
**When** they click to view
**Then** an inline preview is displayed
**And** download button is available

**Given** a user opens an image
**When** they click to view
**Then** the image is displayed in a lightbox
**And** can be downloaded

**Given** a document type doesn't support preview
**When** user clicks to view
**Then** download is triggered directly

**Technical Notes:**
- PDF.js for inline PDF preview
- Image preview in modal
- S3 presigned URLs with 1h expiration for downloads

---

### Story 7A.5: Document Sharing

As a **property manager**,
I want to **share documents with specific users or groups**,
So that **relevant parties can access them**.

**Acceptance Criteria:**

**Given** a manager shares a document
**When** they select recipients (users, roles, buildings)
**Then** recipients receive notification
**And** the document appears in their shared documents

**Given** a share link is generated
**When** a valid link is accessed
**Then** the document is viewable/downloadable
**And** access is logged

**Given** a share is revoked
**When** the revocation is processed
**Then** recipients lose access immediately
**And** share links stop working

**Technical Notes:**
- Create `document_shares` table: id, document_id, share_type, target_id, shared_by, expires_at
- Share types: user, role, building, link
- Link shares with optional password

---

## Epic 8A: Basic Notification Preferences

**Goal:** Users can toggle notifications on/off per channel.

### Story 8A.1: Channel-Level Notification Toggles

As a **user**,
I want to **toggle notifications on/off per channel**,
So that **I control how I receive alerts**.

**Acceptance Criteria:**

**Given** a user opens notification settings
**When** they view channel options
**Then** they see toggles for: push, email, in-app

**Given** a user disables push notifications
**When** an event occurs
**Then** push is not sent
**And** email/in-app still work if enabled

**Given** a user disables all channels
**When** they're warned about missing important info
**Then** they can confirm or cancel the action

**Technical Notes:**
- Create `notification_preferences` table: id, user_id, channel, enabled, updated_at
- Channels: push, email, in_app
- Default: all enabled

---

### Story 8A.2: Critical Notification Override

As a **system administrator**,
I want to **send critical notifications that bypass preferences**,
So that **urgent information always reaches users**.

**Acceptance Criteria:**

**Given** a critical event occurs (system outage, security breach)
**When** admin triggers critical notification
**Then** all users receive it on all enabled channels
**And** an attempt is made on disabled channels with "critical" flag

**Given** a critical notification is sent
**When** user receives it
**Then** it's marked as critical/urgent
**And** cannot be dismissed without acknowledgment

**Technical Notes:**
- Critical flag on notification bypasses preferences
- Push: high priority mode
- Email: marked as important
- UI: modal that requires action

---

### Story 8A.3: Notification Preference Sync

As a **user with multiple devices**,
I want to **my preferences synced across devices**,
So that **settings are consistent everywhere**.

**Acceptance Criteria:**

**Given** a user updates preferences on mobile
**When** they open web app
**Then** the same preferences are applied

**Given** preferences change
**When** the change is saved
**Then** all active sessions receive update via WebSocket
**And** behavior changes immediately

**Technical Notes:**
- Preferences stored server-side
- WebSocket broadcast on change
- Mobile: respect OS-level notification settings too

---

## Epic 9: Privacy, Security & GDPR

**Goal:** Users can enable 2FA, export/delete data, and configure privacy. Audit logs available.

### Story 9.1: TOTP Two-Factor Authentication Setup

As a **security-conscious user**,
I want to **enable two-factor authentication**,
So that **my account is protected even if password is compromised**.

**Acceptance Criteria:**

**Given** a user enables 2FA
**When** they scan QR code with authenticator app
**Then** they enter verification code to confirm setup
**And** 2FA is activated for their account

**Given** a user with 2FA logs in
**When** they enter correct password
**Then** they're prompted for TOTP code
**And** login only succeeds with valid code

**Given** a user loses their authenticator
**When** they use backup codes
**Then** they can access their account
**And** are prompted to set up 2FA again

**Technical Notes:**
- Create `user_2fa` table: id, user_id, secret, enabled_at, backup_codes
- Generate 10 backup codes on setup
- TOTP implementation with 30s window

---

### Story 9.2: 2FA Recovery & Management

As a **user with 2FA enabled**,
I want to **manage and recover my 2FA settings**,
So that **I don't get locked out**.

**Acceptance Criteria:**

**Given** a user views their security settings
**When** 2FA is enabled
**Then** they see: last used, number of remaining backup codes

**Given** a user regenerates backup codes
**When** they confirm the action
**Then** old codes are invalidated
**And** new codes are shown once

**Given** a user disables 2FA
**When** they provide current TOTP code
**Then** 2FA is disabled
**And** confirmation email is sent

**Technical Notes:**
- Backup codes are hashed
- Regeneration requires current code or backup code
- Disable requires authentication proof

---

### Story 9.3: GDPR Data Export

As a **user**,
I want to **export all my personal data**,
So that **I can exercise my GDPR right to data portability**.

**Acceptance Criteria:**

**Given** a user requests data export
**When** they initiate the request
**Then** a background job collects their data
**And** they receive email when ready (within 24h per NFR24)

**Given** the export is complete
**When** user downloads the archive
**Then** it contains: profile, messages, documents, activities
**And** format is machine-readable (JSON)

**Given** an export is requested
**When** one is already in progress
**Then** user sees status of pending export
**And** cannot request another until complete

**Technical Notes:**
- Collect from all tables with user_id
- Include related data (faults reported, votes cast)
- S3 presigned URL, expires in 7 days

---

### Story 9.4: GDPR Data Deletion

As a **user**,
I want to **request deletion of my personal data**,
So that **I can exercise my GDPR right to be forgotten**.

**Acceptance Criteria:**

**Given** a user requests account deletion
**When** they confirm with password
**Then** a 72-hour deletion process begins (per NFR25)
**And** account is immediately deactivated

**Given** the deletion period ends
**When** user hasn't cancelled
**Then** personal data is deleted or anonymized
**And** only legally required records remain

**Given** a user cancels during 72-hour window
**When** they log in with password
**Then** deletion is cancelled
**And** account is reactivated

**Technical Notes:**
- Soft delete first, hard delete after 72h
- Anonymize: replace name with "Deleted User", hash email
- Retain: audit logs, financial records (legal requirement)

---

### Story 9.5: Privacy Settings Configuration

As a **user**,
I want to **configure my privacy settings**,
So that **I control what information is visible to others**.

**Acceptance Criteria:**

**Given** a user opens privacy settings
**When** they view options
**Then** they can set: profile visibility, contact sharing, neighbor visibility

**Given** a user hides their profile
**When** others view neighbor list
**Then** user appears as "Resident of Unit X"
**And** no personal details shown

**Given** a user enables "contacts only"
**When** someone not in contacts tries to message
**Then** message is blocked or flagged for approval

**Technical Notes:**
- Add to `users`: privacy_settings (JSONB)
- Settings: profile_visible, share_email, share_phone, neighbor_visible
- Immediate effect on all queries

---

### Story 9.6: Compliance Audit Logs

As a **compliance officer**,
I want to **access audit logs for sensitive operations**,
So that **I can verify regulatory compliance**.

**Acceptance Criteria:**

**Given** a sensitive operation occurs (data export, deletion, role change)
**When** the operation completes
**Then** an audit log entry is created
**And** includes: timestamp, user, action, affected data, IP

**Given** an officer queries audit logs
**When** they filter by date, action type, user
**Then** matching entries are returned
**And** can be exported for reporting

**Given** audit log integrity is checked
**When** verification runs
**Then** any tampering is detected
**And** alerts are raised

**Technical Notes:**
- Create `audit_logs` table: id, action_type, user_id, target_type, target_id, details (JSONB), ip_address, created_at
- Append-only with no update/delete
- Hash chain for integrity

---

### Story 9.7: Compliance Reports Generation

As a **organization administrator**,
I want to **generate compliance reports**,
So that **I can demonstrate GDPR adherence**.

**Acceptance Criteria:**

**Given** an admin requests compliance report
**When** they select report type and period
**Then** a report is generated with relevant data

**Given** report types include
**When** selected: data access requests, deletions, consent records
**Then** each type shows: count, status, timeline

**Given** a report is generated
**When** exported as PDF
**Then** it includes certification and timestamp

**Technical Notes:**
- Report types: data_requests, deletions, consent, breaches
- Aggregate statistics with drill-down
- Official formatting for regulators

---

## Epic 10A: OAuth Provider Foundation

**Goal:** OAuth 2.0 provider established on api-server for future SSO.

### Story 10A.1: OAuth 2.0 Authorization Server

As a **system architect**,
I want to **implement OAuth 2.0 authorization server**,
So that **future applications can authenticate via SSO**.

**Acceptance Criteria:**

**Given** an OAuth client is registered
**When** it initiates authorization flow
**Then** user is prompted to authorize
**And** authorization code is returned

**Given** a client exchanges authorization code
**When** the code is valid
**Then** access and refresh tokens are issued
**And** client can access protected resources

**Given** an access token is used
**When** it's valid and not expired
**Then** the protected resource is accessible
**And** token claims are available

**Technical Notes:**
- Implement OAuth 2.0 Authorization Code flow
- Create `oauth_clients` table: id, client_id, client_secret_hash, name, redirect_uris, scopes
- Create `oauth_authorizations` table: id, user_id, client_id, code, scopes, expires_at

---

### Story 10A.2: OAuth Client Registration

As a **platform administrator**,
I want to **register OAuth clients**,
So that **trusted applications can integrate**.

**Acceptance Criteria:**

**Given** an admin registers a new OAuth client
**When** they provide name, redirect URIs, and scopes
**Then** client_id and client_secret are generated
**And** the client is active

**Given** a client is registered
**When** it makes an authorization request
**Then** user sees client name and requested scopes
**And** can approve or deny

**Given** an admin revokes a client
**When** revocation is processed
**Then** all tokens for that client are invalidated
**And** new authorizations are blocked

**Technical Notes:**
- Client secret shown only once at creation
- Scopes: profile, email, org:read, full
- Audit log of client registrations

---

### Story 10A.3: OAuth Token Management

As an **OAuth client developer**,
I want to **manage access and refresh tokens**,
So that **my application stays authenticated**.

**Acceptance Criteria:**

**Given** a client has valid refresh token
**When** it requests token refresh
**Then** new access token is issued
**And** optionally new refresh token (rotation)

**Given** a user revokes client access
**When** they remove it from their authorized apps
**Then** all tokens for that user-client pair are invalidated

**Given** token introspection is requested
**When** a valid token is provided
**Then** token metadata is returned (user_id, scopes, expiry)

**Technical Notes:**
- Token rotation configurable per client
- Introspection endpoint for resource servers
- Token expiration: access 15m, refresh 7d

---

## Epic 10B: Platform Administration

**Goal:** Super admins can manage organizations, feature flags, and platform health.

### Story 10B.1: Organization Management Dashboard

As a **super administrator**,
I want to **view and manage all organizations**,
So that **I can oversee platform operations**.

**Acceptance Criteria:**

**Given** a super admin opens admin dashboard
**When** they view organizations list
**Then** they see all orgs with: name, user count, created date, status

**Given** a super admin drills into an organization
**When** they view details
**Then** they see: members, buildings, usage metrics, billing status

**Given** a super admin suspends an organization
**When** the suspension is applied
**Then** all org users are logged out
**And** org appears as suspended

**Technical Notes:**
- Cross-tenant queries bypass RLS
- Suspension cascades to all org members
- Audit log for admin actions

---

### Story 10B.2: Feature Flag Management

As a **platform administrator**,
I want to **manage feature flags**,
So that **I can control feature rollout**.

**Acceptance Criteria:**

**Given** an admin views feature flags
**When** the list is displayed
**Then** they see all flags with: name, status, affected entities

**Given** an admin enables a flag for specific orgs
**When** the flag is updated
**Then** those orgs immediately see the feature
**And** others don't

**Given** a flag is globally enabled
**When** the change is saved
**Then** all users see the feature
**And** the change is logged

**Technical Notes:**
- Create `feature_flags` table: id, name, description, default_value, overrides (JSONB)
- Overrides: by org_id, user_id, role
- Client polls or WebSocket update

---

### Story 10B.3: Platform Health Monitoring

As a **super administrator**,
I want to **monitor platform health metrics**,
So that **I can detect and respond to issues**.

**Acceptance Criteria:**

**Given** an admin views health dashboard
**When** the page loads
**Then** they see: API latency, error rates, active users, queue depth

**Given** a metric exceeds threshold
**When** the threshold is crossed
**Then** visual alert is shown
**And** optional notification sent

**Given** an admin views historical data
**When** they select time range
**Then** trends are displayed in charts

**Technical Notes:**
- Metrics from: Prometheus, application logs, database stats
- Thresholds configurable per metric
- Retention: 30 days detailed, 1 year aggregated

---

### Story 10B.4: System Announcements

As a **platform administrator**,
I want to **broadcast system announcements**,
So that **all users are informed of platform changes**.

**Acceptance Criteria:**

**Given** an admin creates a system announcement
**When** they specify message and severity (info, warning, critical)
**Then** the announcement is queued for display

**Given** an announcement is active
**When** users open the app
**Then** they see the banner at top of screen
**And** can dismiss (for info) or must acknowledge (for critical)

**Given** an admin schedules maintenance
**When** the maintenance window approaches
**Then** countdown is displayed
**And** reminder notifications sent

**Technical Notes:**
- Create `system_announcements` table: id, message, severity, start_at, end_at, dismissible
- Banner with severity-based styling
- Maintenance mode: read-only access

---

### Story 10B.5: Support Data Access

As a **support staff member**,
I want to **access organization data for troubleshooting**,
So that **I can help users resolve issues**.

**Acceptance Criteria:**

**Given** support staff is granted access to an org
**When** they view org data
**Then** they see user-facing data (not internal metadata)
**And** access is read-only

**Given** support access is requested
**When** org admin approves (or auto-approved for support tier)
**Then** access is granted for limited time (24h)
**And** all actions are logged

**Given** support access expires
**When** the time limit is reached
**Then** access is automatically revoked
**And** support staff is notified

**Technical Notes:**
- Temporary tenant context for support
- Read-only mode enforced at API level
- Full audit trail of support access

---

### Story 10B.6: User Onboarding Tour

As a **new user**,
I want to **see an interactive onboarding tour**,
So that **I understand how to use the system**.

**Acceptance Criteria:**

**Given** a user logs in for the first time
**When** the dashboard loads
**Then** an onboarding tour starts automatically
**And** highlights key features step by step

**Given** a user is in the tour
**When** they complete a step
**Then** the next step is shown
**And** progress is tracked

**Given** a user skips or completes the tour
**When** they want to revisit
**Then** they can restart from help menu
**And** progress is reset

**Technical Notes:**
- Add to `users`: onboarding_completed_at
- Tour steps: dashboard, notifications, faults, votes, documents
- Role-specific tours (manager vs resident)

---

### Story 10B.7: Contextual Help & Documentation

As a **user**,
I want to **access contextual help**,
So that **I can learn features without leaving the app**.

**Acceptance Criteria:**

**Given** a user clicks help icon on a screen
**When** help panel opens
**Then** relevant documentation is displayed
**And** links to full docs if needed

**Given** a user searches in help
**When** they enter a query
**Then** matching help articles are returned
**And** can be viewed inline

**Given** documentation is updated
**When** the change is published
**Then** in-app help reflects updates immediately

**Technical Notes:**
- Help content in CMS or markdown files
- Context: route + feature key
- Search via client-side or API

---

## MVP Stories Summary

| Epic | Stories | Total ACs |
|------|---------|-----------|
| 1 - Auth | 7 | 21 |
| 2A - Organizations | 7 | 21 |
| 2B - Notifications | 7 | 21 |
| 3 - Buildings | 7 | 21 |
| 4 - Faults | 7 | 21 |
| 5 - Voting | 8 | 24 |
| 6 - Announcements | 6 | 18 |
| 7A - Documents | 5 | 15 |
| 8A - Notif Prefs | 3 | 9 |
| 9 - Privacy/GDPR | 7 | 21 |
| 10A - OAuth | 3 | 9 |
| 10B - Admin | 7 | 21 |
| **MVP Total** | **74** | **222** |

---

# Phase 2: Enhanced Features & Infrastructure

Phase 2 builds on MVP by completing deferred items and adding financial/utility management capabilities.

**FRs Covered:** FR27 (complete), FR51, FR52-FR63
**Deferred Items Addressed:** WebSocket real-time sync, mobile OS-specific push integration

---

## Epic 7B: Advanced Document Features

**Goal:** Complete document management with versioning, templates, and e-signature integration (FR51).

**Prerequisites:** Epic 7A (Basic Documents)

---

### Story 7B.1: Document Versioning & History

As a **property manager**,
I want to **track document versions and view history**,
So that **I can see changes over time and restore previous versions**.

**Acceptance Criteria:**

**Given** a user uploads a new version of an existing document
**When** the upload completes
**Then** the new version is saved with version number incremented
**And** previous versions remain accessible in version history
**And** the document list shows the latest version by default

**Given** a user views document version history
**When** they select a previous version
**Then** they can view or download that specific version
**And** see who uploaded it and when

**Given** a user restores a previous version
**When** they confirm the restoration
**Then** the old version becomes the new current version
**And** a new version entry is created (not destructive)

**Technical Notes:**
- Extend `documents`: add version_number (default 1), parent_document_id (nullable for version chains)
- Create `document_versions` view for history queries
- S3: each version is a separate object (no overwrite)

---

### Story 7B.2: Document Templates & Generation

As a **property manager**,
I want to **create documents from templates**,
So that **I can quickly generate standardized documents with pre-filled data**.

**Acceptance Criteria:**

**Given** a manager selects a document template
**When** they choose a building/unit context
**Then** the template is populated with relevant data (addresses, names, dates)
**And** a preview is shown before saving

**Given** an organization admin creates a custom template
**When** they define placeholders (e.g., {{unit.address}}, {{owner.name}})
**Then** the template is saved and available for document generation
**And** placeholders are validated against available data fields

**Given** a generated document is created
**When** saved
**Then** it's stored as a regular document with template_id reference
**And** can be edited independently of the template

**Technical Notes:**
- Create `document_templates` table: id, organization_id, name, content (HTML/markdown), placeholders (JSONB)
- Template engine: Handlebars or similar for placeholder substitution
- Default templates: lease agreement, meeting minutes, rules

---

### Story 7B.3: E-Signature Integration

As a **property manager**,
I want to **send documents for electronic signature**,
So that **I can get legally binding signatures without paper**.

**Acceptance Criteria:**

**Given** a manager initiates e-signature on a document
**When** they specify signers (email addresses)
**Then** an e-signature request is created
**And** signers receive email invitations to sign

**Given** a signer completes their signature
**When** all signers have signed
**Then** the document is marked as fully signed
**And** a signed copy is stored in documents
**And** all parties receive confirmation

**Given** a signature request is pending
**When** the manager checks status
**Then** they see which signers have/haven't signed
**And** can send reminders to pending signers

**Technical Notes:**
- Integration: DocuSign or similar (API abstracted)
- Create `signature_requests` table: id, document_id, status, signers (JSONB), completed_at
- Webhook handling for signature events
- Signed documents stored with audit trail

---

## Epic 8B: Granular Notification Preferences

**Goal:** Complete notification preferences with per-event and per-channel controls (FR27 complete).

**Prerequisites:** Epic 8A (Basic Notification Preferences), Epic 2B (Notification Infrastructure)

---

### Story 8B.1: Per-Event Type Preferences

As a **user**,
I want to **control notifications for each event type individually**,
So that **I only receive notifications I care about**.

**Acceptance Criteria:**

**Given** a user opens notification preferences
**When** they view the event type list
**Then** they see all event categories (faults, votes, announcements, documents, etc.)
**And** can toggle each independently

**Given** a user disables "New Fault" notifications
**When** a new fault is created in their building
**Then** they don't receive push/email/in-app for that event
**And** other users with notifications enabled still receive them

**Given** a new event type is added to the system
**When** the user views preferences
**Then** the new type appears with sensible defaults (enabled)
**And** user can adjust as needed

**Technical Notes:**
- Extend `notification_preferences`: event_types (JSONB with per-type settings)
- Default schema: { "fault.created": true, "vote.started": true, ... }
- Migrate existing preferences (all true by default)

---

### Story 8B.2: Per-Channel Delivery Preferences

As a **user**,
I want to **control which channels each notification uses**,
So that **I can get urgent items via push but routine items only in-app**.

**Acceptance Criteria:**

**Given** a user configures channel preferences for an event type
**When** they select channels (push, email, in-app)
**Then** notifications for that event use only selected channels
**And** unselected channels are skipped

**Given** a user sets "Vote Deadline" to push + email
**When** a vote deadline approaches
**Then** they receive both push notification and email
**And** the in-app notification is still created (always on)

**Given** a user sets "New Announcement" to in-app only
**When** an announcement is posted
**Then** they only see it in the notification center
**And** no push or email is sent

**Technical Notes:**
- Extend event_types JSONB: { "fault.created": { enabled: true, channels: ["push", "email", "in_app"] } }
- In-app always on (can be hidden but not disabled)
- UI: checkbox matrix (event types vs channels)

---

### Story 8B.3: Notification Schedule (Do Not Disturb)

As a **user**,
I want to **set quiet hours when notifications are silenced**,
So that **I'm not disturbed during sleep or meetings**.

**Acceptance Criteria:**

**Given** a user enables quiet hours
**When** they set start/end times (e.g., 22:00-07:00)
**Then** push notifications are held during those hours
**And** delivered when quiet hours end

**Given** a notification is high priority (e.g., emergency)
**When** it occurs during quiet hours
**Then** it is delivered immediately regardless of schedule
**And** is marked as priority override

**Given** a user is in a different timezone than their building
**When** quiet hours are evaluated
**Then** the user's configured timezone is used
**And** not the building or server timezone

**Technical Notes:**
- Add to `notification_preferences`: quiet_hours (JSONB: { enabled, start, end, timezone })
- Hold queue in Redis with scheduled delivery
- Priority flag on notification types (configurable by admin)

---

### Story 8B.4: Role-Based Default Preferences

As an **organization admin**,
I want to **set default notification preferences by role**,
So that **new users get appropriate settings automatically**.

**Acceptance Criteria:**

**Given** an admin configures default preferences for "Manager" role
**When** a new manager joins the organization
**Then** their notification preferences are initialized with manager defaults
**And** they can customize from there

**Given** an admin updates role defaults
**When** they save changes
**Then** new users get updated defaults
**And** existing users retain their customized preferences

**Given** a user changes roles (promoted to manager)
**When** the role change is applied
**Then** they keep their existing preferences
**And** are notified of recommended settings for their new role

**Technical Notes:**
- Create `role_notification_defaults` table: id, organization_id, role, preferences (JSONB)
- On user creation: copy from role defaults
- On role change: optional "apply defaults" action

---

## Epic 2B-Complete: WebSocket & Mobile Notification Infrastructure

**Goal:** Complete notification infrastructure with WebSocket real-time sync and mobile OS-specific push integration.

**Prerequisites:** Epic 2B (Notification Infrastructure - foundation)

---

### Story 2B-C.1: WebSocket Real-Time Sync

As a **user with the app open**,
I want to **see real-time updates without refreshing**,
So that **I always have the latest information**.

**Acceptance Criteria:**

**Given** a user has the app open with WebSocket connected
**When** a new notification/fault/vote is created in their context
**Then** the update appears immediately (within 1 second)
**And** no manual refresh is needed

**Given** the WebSocket connection drops
**When** it reconnects
**Then** missed updates are fetched via REST delta sync
**And** the UI is updated accordingly

**Given** multiple browser tabs are open
**When** an update arrives
**Then** all tabs receive and display the update
**And** read states sync across tabs

**Technical Notes:**
- Axum WebSocket with per-user channels
- Redis pub/sub for cross-instance message routing
- Client: reconnect with exponential backoff
- Delta sync: last_sync_at timestamp comparison

---

### Story 2B-C.2: Mobile OS-Specific Push Integration

As a **mobile user**,
I want to **receive native push notifications**,
So that **I'm alerted even when the app is closed**.

**Acceptance Criteria:**

**Given** an Android user installs the app
**When** they grant notification permissions
**Then** their FCM token is registered with the server
**And** push notifications work reliably

**Given** an iOS user installs the app
**When** they grant notification permissions
**Then** their APNs token is registered
**And** push notifications display correctly (even with app killed)

**Given** a user updates/reinstalls the app
**When** a new token is generated
**Then** the old token is invalidated
**And** the new token is registered seamlessly

**Technical Notes:**
- React Native: @react-native-firebase/messaging for both platforms
- Token refresh handling in app lifecycle
- Badge count sync with unread notification count
- Notification grouping (collapse similar notifications)

---

### Story 2B-C.3: Notification Analytics & Delivery Tracking

As a **platform operator**,
I want to **track notification delivery and engagement**,
So that **I can optimize notification effectiveness**.

**Acceptance Criteria:**

**Given** notifications are sent
**When** the operator views analytics dashboard
**Then** they see delivery rates by channel (push/email/in-app)
**And** open/click rates where trackable

**Given** push notifications fail to deliver
**When** the failure rate exceeds threshold (>5%)
**Then** an alert is triggered
**And** the operator can investigate (invalid tokens, service issues)

**Given** a specific notification type has low engagement
**When** compared to baseline
**Then** the operator can identify and adjust content/timing
**And** track improvement over time

**Technical Notes:**
- Create `notification_events` table: id, notification_id, event (sent, delivered, opened, clicked), timestamp
- Push: FCM/APNs delivery receipts
- Email: tracking pixels, link tracking
- Dashboard: aggregations by type, channel, time period

---

## Epic 11: Financial Management & Payments

**Goal:** Enable financial record keeping, fee management, and payment processing (FR52-FR57).

**Prerequisites:** Epic 3 (Buildings/Units), Epic 2A (Organizations)

---

### Story 11.1: Financial Account Structure

As a **property manager**,
I want to **set up financial accounts for buildings and units**,
So that **I can track income, expenses, and balances**.

**Acceptance Criteria:**

**Given** a manager views a building's financial section
**When** no accounts exist
**Then** they can create accounts (operating, reserve, utilities)
**And** each account has opening balance and currency

**Given** a unit is created
**When** it's associated with a building
**Then** a unit ledger account is automatically created
**And** linked to the building's main accounts

**Given** an account has transactions
**When** the manager views the account
**Then** they see running balance, recent transactions
**And** can filter by date range and category

**Technical Notes:**
- Create `financial_accounts` table: id, organization_id, building_id, unit_id (nullable), account_type, currency, balance
- Create `account_transactions` table: id, account_id, amount, type (debit/credit), category, description, date, reference
- Double-entry bookkeeping pattern (each transaction affects 2 accounts)

---

### Story 11.2: Fee Schedule Management

As a **property manager**,
I want to **define recurring fees for units**,
So that **monthly charges are automatically calculated**.

**Acceptance Criteria:**

**Given** a manager opens fee schedule for a building
**When** they add a fee type (maintenance, utilities, parking)
**Then** they can specify amount, frequency, and which units it applies to
**And** the fee is added to the schedule

**Given** a fee schedule exists
**When** the billing period starts
**Then** fees are automatically calculated for each unit
**And** owner/tenant can see their upcoming charges

**Given** a manager adjusts a fee mid-period
**When** the change is saved
**Then** it applies from the next billing period
**And** current period charges remain unchanged

**Technical Notes:**
- Create `fee_schedules` table: id, building_id, name, amount, frequency (monthly/quarterly/annual), unit_filter (JSONB)
- Create `unit_fees` table: id, unit_id, fee_schedule_id, effective_from, effective_to
- Cron job: generate charges on billing period start

---

### Story 11.3: Invoice Generation

As a **property manager**,
I want to **generate invoices for unit owners/tenants**,
So that **they have official payment requests**.

**Acceptance Criteria:**

**Given** a billing period closes
**When** invoices are generated
**Then** each unit with charges receives an invoice
**And** the invoice contains all line items with due date

**Given** an invoice is generated
**When** the owner/tenant views it
**Then** they see itemized charges, total, due date
**And** can download as PDF

**Given** a manager generates ad-hoc invoice
**When** they add custom line items
**Then** the invoice is created immediately
**And** the owner/tenant is notified

**Technical Notes:**
- Create `invoices` table: id, unit_id, billing_period, status (draft, sent, paid, overdue), due_date, total
- Create `invoice_items` table: id, invoice_id, description, amount, fee_schedule_id (nullable)
- PDF generation: template-based (organization branding)

---

### Story 11.4: Payment Recording

As a **property manager**,
I want to **record payments received**,
So that **account balances are accurate**.

**Acceptance Criteria:**

**Given** a payment is received (bank transfer, cash)
**When** the manager records it
**Then** it's linked to the relevant invoice(s)
**And** the invoice status updates (paid/partial)
**And** account balances update accordingly

**Given** a partial payment is made
**When** recorded
**Then** the invoice shows remaining balance
**And** the payment history shows all payments

**Given** an overpayment occurs
**When** recorded
**Then** the excess is credited to the unit's account
**And** can be applied to future invoices

**Technical Notes:**
- Create `payments` table: id, unit_id, invoice_id (nullable), amount, method, reference, recorded_at, recorded_by
- Auto-allocation: oldest invoice first
- Credit balance tracking per unit

---

### Story 11.5: Online Payment Integration

As a **unit owner/tenant**,
I want to **pay invoices online**,
So that **I can pay conveniently without bank transfers**.

**Acceptance Criteria:**

**Given** a user views an unpaid invoice
**When** they click "Pay Now"
**Then** they're directed to payment gateway
**And** can pay with card or bank transfer

**Given** a payment is successful
**When** the gateway confirms
**Then** the invoice is marked paid
**And** the user receives confirmation
**And** a receipt is generated

**Given** a payment fails
**When** the gateway reports failure
**Then** the user sees error message
**And** can retry or choose different method
**And** the invoice remains unpaid

**Technical Notes:**
- Integration: Stripe or local payment provider
- Webhook handling for async confirmations
- PCI compliance: no card data stored locally
- Fee handling: who pays transaction fees (configurable)

---

### Story 11.6: Payment Reminders & Overdue Handling

As a **property manager**,
I want to **automate payment reminders and track overdue payments**,
So that **collection is efficient and consistent**.

**Acceptance Criteria:**

**Given** an invoice due date approaches (configurable days before)
**When** the reminder schedule triggers
**Then** the owner/tenant receives reminder notification
**And** the reminder is logged

**Given** an invoice is past due
**When** the grace period expires
**Then** the invoice status changes to "overdue"
**And** escalation notifications are sent (configurable)

**Given** a unit has overdue balance
**When** the manager views unit details
**Then** overdue amount is highlighted
**And** payment history shows aging breakdown

**Technical Notes:**
- Create `reminder_schedules` table: id, organization_id, days_before_due, days_after_due, template_id
- Cron job: process reminders daily
- Late fee calculation (optional, configurable)

---

### Story 11.7: Financial Reports

As a **property manager**,
I want to **generate financial reports**,
So that **I can review financial health and share with stakeholders**.

**Acceptance Criteria:**

**Given** a manager requests income statement
**When** they select date range
**Then** report shows all income and expenses
**And** categorized by type with totals

**Given** a manager requests accounts receivable report
**When** generated
**Then** report shows all outstanding balances by unit
**And** aging buckets (current, 30, 60, 90+ days)

**Given** a manager exports a report
**When** they choose format (PDF, Excel)
**Then** the report is generated and downloadable
**And** includes organization branding

**Technical Notes:**
- Report types: income statement, balance sheet, AR aging, cash flow
- Filtering: building, date range, account type
- Export: PDF (template), Excel (raw data)
- Scheduled reports: email to stakeholders (configurable)

---

## Epic 12: Meter Readings & Utilities

**Goal:** Enable meter reading submission, validation, and utility cost distribution (FR58-FR63).

**Prerequisites:** Epic 3 (Buildings/Units), Epic 11 (Financial Management)

---

### Story 12.1: Meter Registration

As a **property manager**,
I want to **register meters for units and common areas**,
So that **readings can be tracked and validated**.

**Acceptance Criteria:**

**Given** a manager adds a meter to a unit
**When** they enter meter details (type, ID, location, initial reading)
**Then** the meter is registered
**And** appears in the unit's meter list

**Given** a building has common area meters
**When** registered
**Then** they're marked as shared
**And** costs can be distributed to units

**Given** a meter is replaced
**When** the manager records replacement
**Then** old meter is archived with final reading
**And** new meter starts fresh with initial reading

**Technical Notes:**
- Create `meters` table: id, unit_id (nullable for common), building_id, type (electricity, gas, water, heat), meter_id, location, active
- Create `meter_readings` table: id, meter_id, reading, date, source (manual, photo, automatic), submitted_by
- Meter types: electricity, gas, water, heat (extensible)

---

### Story 12.2: Self-Reading Submission

As a **unit owner/tenant**,
I want to **submit meter readings myself**,
So that **my utility charges are accurate**.

**Acceptance Criteria:**

**Given** a user is prompted to submit readings
**When** they enter the reading value
**Then** the reading is recorded with timestamp
**And** basic validation is performed (not less than previous)

**Given** a user takes a photo of the meter
**When** they upload it
**Then** the photo is stored as evidence
**And** OCR extracts the reading (with manual correction option)

**Given** the submission window is open
**When** a user submits readings
**Then** they see confirmation
**And** can edit until the window closes

**Technical Notes:**
- Submission windows: configurable dates per building
- Photo storage: S3 with reference in reading
- OCR: optional integration (Azure/Google Vision)
- Validation: reading >= previous, within reasonable range

---

### Story 12.3: Reading Validation & Approval

As a **property manager**,
I want to **review and approve submitted readings**,
So that **erroneous readings are caught before billing**.

**Acceptance Criteria:**

**Given** readings are submitted
**When** the manager views the approval queue
**Then** they see all pending readings with validation status
**And** flagged readings are highlighted (anomalies)

**Given** a reading is flagged as anomaly
**When** the manager reviews it
**Then** they can approve with note, request re-submission, or enter corrected value
**And** the action is logged

**Given** all readings for a period are approved
**When** the manager finalizes
**Then** readings are locked for billing
**And** consumption is calculated (current - previous)

**Technical Notes:**
- Validation rules: configurable thresholds per meter type
- Anomaly detection: >2x or <0.5x of historical average
- Approval states: pending, approved, rejected, estimated
- Estimation: use average if reading missing

---

### Story 12.4: Utility Cost Distribution

As a **property manager**,
I want to **distribute utility costs to units based on consumption**,
So that **each unit pays their fair share**.

**Acceptance Criteria:**

**Given** meter readings are finalized
**When** the utility bill arrives
**Then** the manager enters total cost
**And** system calculates per-unit charges based on consumption

**Given** a common area meter exists
**When** costs are distributed
**Then** the manager can choose distribution method (equal, by area, by occupants)
**And** charges are allocated accordingly

**Given** distribution is complete
**When** approved
**Then** charges are added to unit accounts
**And** appear on next invoice

**Technical Notes:**
- Create `utility_bills` table: id, building_id, meter_type, period_start, period_end, total_amount, distribution_method
- Distribution methods: consumption-based, area-based, equal split, hybrid
- Auto-generation of unit charges after distribution

---

### Story 12.5: Consumption History & Analytics

As a **unit owner/tenant**,
I want to **view my consumption history**,
So that **I can track usage trends and identify waste**.

**Acceptance Criteria:**

**Given** a user views their consumption
**When** they select a meter type
**Then** they see historical readings as chart
**And** month-over-month comparison

**Given** consumption data spans multiple years
**When** viewed
**Then** year-over-year comparison is available
**And** seasonal patterns are visible

**Given** a user's consumption exceeds building average
**When** they view their report
**Then** a comparison indicator is shown
**And** tips for reduction are suggested

**Technical Notes:**
- Charts: line graph (time series), bar chart (comparison)
- Aggregations: daily, monthly, yearly
- Building averages: calculated from all unit readings
- Privacy: only show own data + anonymized building average

---

### Story 12.6: Automatic Meter Reading Integration

As a **property manager**,
I want to **receive readings from smart meters automatically**,
So that **manual submission isn't needed**.

**Acceptance Criteria:**

**Given** a smart meter is configured
**When** it transmits a reading
**Then** the reading is automatically recorded
**And** marked as source: automatic

**Given** automatic readings are received
**When** they differ from expected pattern
**Then** anomaly detection still applies
**And** manager is notified of potential issues

**Given** automatic meter fails to report
**When** the expected reading is missing
**Then** an alert is generated
**And** manual reading can be entered as fallback

**Technical Notes:**
- API endpoint for meter data ingestion
- Authentication: API key per meter provider
- Supported protocols: REST (others via adapters)
- Reliability: retry logic, missing data alerts

---

## Phase 2 Stories Summary

| Epic | Stories | FRs Covered |
|------|---------|-------------|
| 7B - Advanced Documents | 3 | FR51 |
| 8B - Granular Notifications | 4 | FR27 (complete) |
| 2B-Complete - WebSocket & Mobile | 3 | Deferred items |
| 11 - Financial Management | 7 | FR52-FR57 |
| 12 - Meter Readings | 6 | FR58-FR63 |
| **Phase 2 Total** | **23** | **13 FRs** |

---

## Phase 2 Sprint Plan

| Sprint | Epics | Stories | Rationale |
|--------|-------|---------|-----------|
| 2A | Epic 7B + Epic 8B | 7 | Complete deferred MVP items (low dependencies) |
| 2B | Epic 2B-Complete | 3 | Real-time infrastructure (enables future features) |
| 2C | Epic 11 | 7 | Financial management (high business value) |
| 2D | Epic 12 | 6 | Utilities (depends on Epic 11 for billing integration) |

**Dependency Chain:** MVP → 7B/8B (parallel) → 2B-Complete → 11 → 12

---

# Phase 3 Stories: Modern Technology

## Epic 13: AI Assistant & Automation

**Goal:** Users interact with AI chatbot. Managers configure workflow automations. System provides intelligent insights.

**FRs covered:** FR64, FR65, FR66, FR67, FR68, FR69, FR70
**Estimate:** 4 weeks

### Story 13.1: AI Chatbot Interface

As a **resident or manager**,
I want to **interact with an AI chatbot for common questions**,
So that **I can get quick answers without searching through documentation**.

**Acceptance Criteria:**

**Given** a user opens the AI assistant
**When** they type a question in natural language
**Then** the chatbot processes the query
**And** returns a relevant response within 3 seconds

**Given** a question about building operations
**When** the chatbot responds
**Then** it references relevant documents, announcements, or FAQs
**And** provides links to source materials

**Given** the chatbot cannot answer a question
**When** confidence is below 70%
**Then** it acknowledges uncertainty
**And** offers to connect with a human manager
**And** logs the query for training improvement

**Technical Notes:**
- RAG (Retrieval Augmented Generation) on indexed documents from Epic 7B
- Confidence threshold: >90% auto-respond, 70-90% respond with disclaimer, <70% escalate
- Chat history stored per user for context
- LLM integration: external API (OpenAI/Claude) with local fallback

---

### Story 13.2: Sentiment Analysis for Messages

As a **property manager**,
I want to **see sentiment trends in resident communications**,
So that **I can identify emerging issues before they escalate**.

**Acceptance Criteria:**

**Given** messages and comments are submitted
**When** they have AI training consent enabled
**Then** sentiment is analyzed and scored (-1 to +1)
**And** score is stored with the message

**Given** a manager views the sentiment dashboard
**When** they select a date range
**Then** they see sentiment trends over time
**And** can identify spikes in negative sentiment

**Given** sentiment suddenly drops for a building
**When** the threshold is breached
**Then** an alert is generated for the manager
**And** recent negative messages are highlighted

**Technical Notes:**
- Add `sentiment_score` column to messages, comments, fault reports
- Only analyze content with `ai_training_consent = true`
- Dashboard: line chart over time, building comparison
- Alert threshold: configurable per organization

---

### Story 13.3: Predictive Maintenance

As a **technical manager**,
I want to **receive predictions about equipment maintenance needs**,
So that **I can schedule preventive maintenance before failures occur**.

**Acceptance Criteria:**

**Given** equipment is tracked in the system with maintenance history
**When** the prediction engine analyzes patterns
**Then** it estimates time until next maintenance needed
**And** generates recommendations

**Given** a prediction indicates high failure risk
**When** the threshold is exceeded
**Then** a proactive maintenance task is suggested
**And** the technical manager is notified

**Given** equipment age and fault history are available
**When** combined with manufacturer guidelines
**Then** maintenance schedules are optimized
**And** budget implications are estimated

**Technical Notes:**
- Input: equipment type, age, fault history, manufacturer data
- ML model: trained on historical fault patterns
- Output: risk score (0-100), days until recommended service
- Integration with fault reporting system

---

### Story 13.4: Automatic Document Summarization

As a **busy manager**,
I want to **get automatic summaries of long documents**,
So that **I can quickly understand key points without reading everything**.

**Acceptance Criteria:**

**Given** a document longer than 1000 words is uploaded
**When** it finishes processing
**Then** an AI summary is generated
**And** displayed alongside the full document

**Given** a user views a document
**When** a summary exists
**Then** they see "Summary" toggle option
**And** can switch between summary and full view

**Given** meeting minutes or reports are uploaded
**When** summarized
**Then** key decisions and action items are extracted
**And** highlighted in the summary

**Technical Notes:**
- LLM summarization with extractive + abstractive approach
- Max summary length: 500 words or 20% of original
- Store in `documents` table: ai_summary, summary_generated_at
- Re-generate option if document updated

---

### Story 13.5: Smart Search with NLP

As a **user searching for information**,
I want to **use natural language queries**,
So that **I can find relevant content without knowing exact keywords**.

**Acceptance Criteria:**

**Given** a user enters a natural language query
**When** they search across the system
**Then** results are ranked by semantic relevance
**And** include documents, announcements, faults, and messages

**Given** a query like "problems with heating last winter"
**When** processed
**Then** faults about heating from winter months are returned
**And** related announcements about heating are included

**Given** search results are displayed
**When** the user views them
**Then** relevant snippets are highlighted
**And** source type is indicated

**Technical Notes:**
- Vector embeddings for semantic search
- Index: documents, announcements, faults, comments
- Combine keyword + semantic scoring
- Re-index on content changes

---

### Story 13.6: Workflow Automation Configuration

As a **property manager**,
I want to **configure automated workflows**,
So that **routine tasks are handled automatically**.

**Acceptance Criteria:**

**Given** a manager opens workflow configuration
**When** they create a new automation
**Then** they select trigger, conditions, and actions
**And** can preview the workflow

**Given** trigger types available
**When** configuring
**Then** options include: fault created, payment due, document uploaded, time-based

**Given** action types available
**When** configuring
**Then** options include: send notification, create task, update status, send email

**Technical Notes:**
- Create `workflows` table: id, organization_id, name, trigger_type, trigger_config, enabled
- Create `workflow_actions` table: id, workflow_id, action_type, action_config, order
- Workflow engine: event-driven execution
- Audit all workflow executions

---

### Story 13.7: Event-Triggered Automated Actions

As a **system**,
I want to **execute automated actions based on events**,
So that **workflows run reliably without manual intervention**.

**Acceptance Criteria:**

**Given** an event matches a workflow trigger
**When** conditions are met
**Then** the configured actions execute in sequence
**And** execution is logged

**Given** an action fails
**When** retry policy is configured
**Then** the action is retried up to 3 times
**And** failure is logged with error details

**Given** a workflow execution completes
**When** reviewing history
**Then** full execution trace is visible
**And** performance metrics are tracked

**Technical Notes:**
- Create `workflow_executions` table: id, workflow_id, triggered_at, status, error
- Create `workflow_execution_steps` table: id, execution_id, action_id, status, output
- Event bus integration (Redis pub/sub)
- Dead letter queue for failed executions

---

## Epic 14: IoT & Smart Building

**Goal:** Users view sensor data dashboards and receive threshold alerts. System correlates sensor data with building operations.

**FRs covered:** FR71, FR72, FR73, FR74, FR75
**Estimate:** 3 weeks

### Story 14.1: IoT Sensor Registration

As a **technical manager**,
I want to **register IoT sensors in the system**,
So that **sensor data can be collected and monitored**.

**Acceptance Criteria:**

**Given** a manager adds a new sensor
**When** they provide type, location, and connection details
**Then** the sensor is registered in the system
**And** appears in the sensor list

**Given** a sensor is being configured
**When** they specify the data type (temperature, humidity, motion, etc.)
**Then** appropriate data handling is configured
**And** units of measurement are set

**Given** a sensor has authentication requirements
**When** credentials are provided
**Then** they are stored securely
**And** connection is verified

**Technical Notes:**
- Create `sensors` table: id, building_id, name, type, location, connection_config, status
- Sensor types: temperature, humidity, motion, co2, water_leak, energy, smoke
- Encrypted storage for credentials
- Health check endpoint for each sensor

---

### Story 14.2: Sensor Data Ingestion

As a **system**,
I want to **ingest data from IoT sensors**,
So that **real-time monitoring is possible**.

**Acceptance Criteria:**

**Given** a sensor sends data
**When** the ingestion endpoint receives it
**Then** data is validated and stored
**And** timestamp is recorded

**Given** data arrives in various formats
**When** processed
**Then** it's normalized to a standard schema
**And** stored in time-series optimized format

**Given** a sensor goes offline
**When** no data is received for the expected interval
**Then** an offline alert is generated
**And** sensor status is updated

**Technical Notes:**
- Create `sensor_readings` table: id, sensor_id, value, unit, timestamp (partitioned by time)
- Ingestion: REST API with API key auth, optional MQTT adapter
- Time-series storage: TimescaleDB extension or partitioned PostgreSQL
- Retention: raw data 1 year, aggregated data 5 years

---

### Story 14.3: Real-time Sensor Dashboards

As a **building manager**,
I want to **view real-time sensor data on dashboards**,
So that **I can monitor building conditions at a glance**.

**Acceptance Criteria:**

**Given** a manager opens the sensor dashboard
**When** sensors are active
**Then** current values are displayed with last update time
**And** historical graphs show trends

**Given** the dashboard is open
**When** new data arrives
**Then** values update in real-time (WebSocket)
**And** graphs animate new data points

**Given** a manager selects a date range
**When** viewing historical data
**Then** aggregated data is displayed
**And** can be zoomed/panned

**Technical Notes:**
- WebSocket for real-time updates
- Charts: line (time series), gauge (current value)
- Aggregation levels: minute, hour, day
- Dashboard layout saved per user

---

### Story 14.4: Threshold Violation Alerts

As a **building manager**,
I want to **receive alerts when sensor values exceed thresholds**,
So that **I can respond to abnormal conditions quickly**.

**Acceptance Criteria:**

**Given** a threshold is configured for a sensor
**When** the value exceeds the threshold
**Then** an alert is generated
**And** notification is sent per user preferences

**Given** multiple thresholds exist
**When** configuring
**Then** warning and critical levels can be set
**And** different actions triggered for each

**Given** an alert condition resolves
**When** value returns to normal range
**Then** a resolution notification is sent
**And** alert status is updated

**Technical Notes:**
- Create `sensor_thresholds` table: id, sensor_id, metric, warning_value, critical_value, comparison
- Create `sensor_alerts` table: id, sensor_id, threshold_id, value, triggered_at, resolved_at
- Alert debouncing: minimum 5 minutes between same alerts
- Integration with notification service

---

### Story 14.5: Sensor-Fault Correlation

As a **technical manager**,
I want to **see correlations between sensor data and fault reports**,
So that **I can identify root causes of issues**.

**Acceptance Criteria:**

**Given** a fault is reported
**When** sensor data exists for that location
**Then** relevant sensor readings are linked to the fault
**And** displayed in fault details

**Given** sensor anomalies are detected
**When** a related fault exists
**Then** the correlation is highlighted
**And** suggested as potential cause

**Given** historical analysis is performed
**When** patterns emerge
**Then** predictive alerts are generated
**And** recommendations made

**Technical Notes:**
- Link faults to nearby sensors via location
- Correlation engine: analyze sensor data ±24 hours of fault creation
- Display: sensor chart embedded in fault timeline
- ML: pattern detection for recurring correlations

---

### Story 14.6: Sensor Threshold Configuration

As a **building manager**,
I want to **configure sensor thresholds**,
So that **alerts are meaningful for my buildings**.

**Acceptance Criteria:**

**Given** a manager configures thresholds
**When** they set values for a sensor type
**Then** warning and critical levels are saved
**And** applied to selected sensors

**Given** default thresholds exist
**When** a sensor is registered
**Then** defaults are applied based on type
**And** can be customized

**Given** thresholds are updated
**When** saved
**Then** change is logged
**And** existing alerts are re-evaluated

**Technical Notes:**
- Default thresholds per sensor type (e.g., temperature: warning 25°C, critical 30°C)
- Bulk configuration for multiple sensors
- Threshold presets: comfort, energy-saving, safety
- Audit trail for threshold changes

---

# Phase 4 Stories: Reality Portal & Rental

## Epic 10A-SSO: Cross-Platform SSO Consumer

**Goal:** Complete SSO between Property Management and Reality Portal. Users authenticate once and access both platforms.

**FRs covered:** FR7 (complete)
**Estimate:** 1.5 weeks

### Story 10A-SSO.1: OIDC Consumer on Reality Server

As a **Reality Portal user**,
I want to **log in using my Property Management account**,
So that **I don't need separate credentials for both platforms**.

**Acceptance Criteria:**

**Given** a user clicks "Login with Property Management" on Reality Portal
**When** they are redirected to api-server OAuth
**Then** they authenticate with their PM credentials
**And** are redirected back with authorization code

**Given** the authorization code is received
**When** reality-server exchanges it for tokens
**Then** access and refresh tokens are issued
**And** user session is created on Reality Portal

**Given** user is already logged into Property Management
**When** they access Reality Portal
**Then** SSO occurs automatically
**And** no re-authentication required

**Technical Notes:**
- OIDC authorization code flow
- reality-server: OIDC consumer (using existing OAuth provider from Epic 10A)
- Shared user identity via user_id
- Token claims: user_id, email, name

---

### Story 10A-SSO.2: Mobile Deep-Link Token Sharing

As a **mobile user**,
I want to **switch between PM and Reality Portal apps seamlessly**,
So that **I stay logged in across both apps**.

**Acceptance Criteria:**

**Given** a user is logged into PM mobile app
**When** they tap a link to Reality Portal
**Then** the app opens with SSO token passed via deep link
**And** user is automatically authenticated

**Given** a deep link token is received
**When** Reality Portal app validates it
**Then** a new session is created
**And** user sees their authenticated state

**Given** the SSO token is expired
**When** deep link is followed
**Then** user is prompted to log in
**And** can do so via PM credentials

**Technical Notes:**
- Deep link scheme: `reality://sso?token=xxx`
- Token: short-lived (5 minutes), one-time use
- Validate against api-server before creating session
- Fallback: redirect to login if token invalid

---

### Story 10A-SSO.3: Unified Account Management

As a **user with accounts on both platforms**,
I want to **manage my account in one place**,
So that **changes apply everywhere**.

**Acceptance Criteria:**

**Given** a user updates their profile on PM
**When** they access Reality Portal
**Then** their updated name, email, and preferences are reflected
**And** no manual sync required

**Given** a user changes their password on PM
**When** their Reality Portal session is active
**Then** the session remains valid until expiry
**And** next login uses new password

**Given** a user deletes their PM account
**When** they try to access Reality Portal
**Then** access is denied
**And** local data is cleared

**Technical Notes:**
- Single source of truth: users table on api-server
- Reality Portal: fetch user details from api-server on each session
- Event: user_updated, user_deleted propagated via event bus
- Cache user details with 5-minute TTL

---

## Epic 15: Property Listings & Multi-Portal Sync

**Goal:** Owners and Realtors create listings from unit data and publish to multiple portals.

**FRs covered:** FR76, FR77, FR78
**Estimate:** 3 weeks

### Story 15.1: Listing Creation from Unit Data

As a **property owner**,
I want to **create a listing from my existing unit data**,
So that **I don't have to re-enter property information**.

**Acceptance Criteria:**

**Given** an owner accesses their unit
**When** they click "Create Listing"
**Then** unit data (address, size, type) is pre-populated
**And** they can add listing-specific details

**Given** unit data is imported
**When** the owner reviews
**Then** they can edit any pre-filled fields
**And** add photos, description, and price

**Given** the listing is ready
**When** saved as draft
**Then** it's visible only to the owner
**And** can be published later

**Technical Notes:**
- Create `listings` table: id, unit_id, organization_id, status, price, description, created_at
- Status: draft, active, paused, sold, rented, archived
- Copy unit data but allow override
- Photos stored in S3 with listing_id prefix

---

### Story 15.2: Listing Management (Photos, Description, Pricing)

As a **realtor**,
I want to **manage listing details including photos and pricing**,
So that **the listing is attractive to potential buyers/renters**.

**Acceptance Criteria:**

**Given** a realtor edits a listing
**When** they upload photos
**Then** photos are processed (resized, optimized)
**And** can be reordered via drag-and-drop

**Given** a description is entered
**When** it includes formatting (bold, lists)
**Then** markdown is supported
**And** preview shows rendered output

**Given** pricing is set
**When** type is selected (sale, rent)
**Then** appropriate fields are shown
**And** currency options available (EUR, CZK)

**Technical Notes:**
- Create `listing_photos` table: id, listing_id, url, order, alt_text
- Image processing: thumbnail (300x200), medium (800x600), large (1600x1200)
- Rich text: markdown with sanitization
- Price fields: price, price_type (sale/rent), currency, negotiable flag

---

### Story 15.3: Multi-Portal Syndication

As a **realtor**,
I want to **publish listings to multiple portals simultaneously**,
So that **I can maximize exposure**.

**Acceptance Criteria:**

**Given** a listing is ready for publishing
**When** the realtor selects target portals
**Then** options include: Reality Portal, Sreality, Bezrealitky, Nehnutelnosti
**And** portal-specific requirements are shown

**Given** a listing is published
**When** syndication runs
**Then** listing data is sent to each selected portal
**And** status per portal is tracked

**Given** a portal sync fails
**When** the error is logged
**Then** the realtor is notified
**And** can retry or troubleshoot

**Technical Notes:**
- Create `listing_syndications` table: id, listing_id, portal, external_id, status, synced_at
- Portal adapters: abstract interface, concrete implementations
- Initial: Reality Portal (native), others via API
- Cron job: sync changes every 15 minutes

---

### Story 15.4: Listing Status Management

As a **realtor**,
I want to **update listing status (paused, sold, rented)**,
So that **portals reflect current availability**.

**Acceptance Criteria:**

**Given** a property is sold
**When** the realtor marks it as "Sold"
**Then** listing is removed from active portals
**And** marked with sold date

**Given** a listing should be temporarily hidden
**When** status is set to "Paused"
**Then** it's hidden from public view
**And** can be reactivated

**Given** status changes
**When** syndicated portals exist
**Then** update is propagated to all portals
**And** sync status is confirmed

**Technical Notes:**
- Status transitions: draft → active → paused/sold/rented → archived
- Syndication propagates status changes
- Sold/rented listings visible in history for 90 days
- Archive after 90 days

---

## Epic 16: Portal Search & Discovery

**Goal:** Portal users search, filter, and save favorite listings.

**FRs covered:** FR79, FR80, FR81
**Estimate:** 2.5 weeks

### Story 16.1: Property Search & Filtering

As a **portal user**,
I want to **search properties with filters**,
So that **I can find listings matching my criteria**.

**Acceptance Criteria:**

**Given** a user opens the search page
**When** they enter search criteria
**Then** filters include: location, price range, property type, size, rooms
**And** results update as filters change

**Given** search is performed
**When** results are returned
**Then** listings are sorted by relevance
**And** displayed with photo, price, and key details

**Given** no results match
**When** displayed
**Then** suggestions are shown (expand range, nearby areas)
**And** save search option offered

**Technical Notes:**
- Elasticsearch for search indexing
- Filters: location (geo), price_min/max, type, size_min/max, rooms_min/max
- SSR/SSG for SEO (Next.js)
- URL reflects search state for sharing/bookmarking

---

### Story 16.2: Favorite Listings

As a **portal user**,
I want to **save favorite listings**,
So that **I can review them later**.

**Acceptance Criteria:**

**Given** a user views a listing
**When** they click the heart icon
**Then** the listing is saved to favorites
**And** the icon fills in

**Given** a user views their favorites
**When** they open the favorites page
**Then** all saved listings are displayed
**And** can be removed

**Given** a favorited listing is updated
**When** price or status changes
**Then** user is notified (if opted in)
**And** change is highlighted in favorites

**Technical Notes:**
- Create `favorites` table: id, user_id, listing_id, created_at
- Anonymous users: store in local storage, prompt to create account
- Notifications: price drop, back on market
- Max favorites: 100 per user

---

### Story 16.3: Saved Searches & Alerts

As a **portal user**,
I want to **save my search criteria and get alerts**,
So that **I'm notified when matching properties are listed**.

**Acceptance Criteria:**

**Given** a user performs a search
**When** they click "Save Search"
**Then** the search criteria are stored
**And** they can name the search

**Given** a new listing matches saved criteria
**When** it's published
**Then** user receives notification
**And** can click through to listing

**Given** a user manages saved searches
**When** viewing their list
**Then** they can edit, delete, or toggle notifications
**And** see last match date

**Technical Notes:**
- Create `saved_searches` table: id, user_id, name, criteria (JSONB), alerts_enabled, last_matched_at
- Matching: run on new listing publication
- Email digest option: daily or instant
- Max saved searches: 10 per user

---

### Story 16.4: Agent Contact Inquiry

As a **portal user**,
I want to **contact the listing agent**,
So that **I can ask questions or schedule a viewing**.

**Acceptance Criteria:**

**Given** a user views a listing
**When** they click "Contact Agent"
**Then** a contact form is shown
**And** pre-filled with their info if logged in

**Given** the inquiry is submitted
**When** the agent receives it
**Then** it includes: message, contact details, listing reference
**And** agent is notified via preferred channel

**Given** the agent replies
**When** using the system
**Then** user receives the response
**And** conversation thread is maintained

**Technical Notes:**
- Create `inquiries` table: id, listing_id, user_id, agent_id, message, status, created_at
- Create `inquiry_messages` table: id, inquiry_id, sender_id, content, created_at
- Lead tracking: inquiry → viewing → offer → closed
- GDPR: contact info only visible to listing agent

---

## Epic 17: Agency & Realtor Management

**Goal:** Agencies manage realtors and shared listings.

**FRs covered:** FR82, FR83
**Estimate:** 2 weeks

### Story 17.1: Agency Registration & Setup

As an **agency owner**,
I want to **register my agency on the platform**,
So that **my team can manage listings together**.

**Acceptance Criteria:**

**Given** an agency owner registers
**When** they provide agency name, address, and contact
**Then** the agency is created
**And** owner becomes agency admin

**Given** agency is set up
**When** admin configures branding
**Then** logo and colors can be uploaded
**And** appear on agency profile and listings

**Given** agency profile exists
**When** portal users view it
**Then** they see agency info, team, and active listings

**Technical Notes:**
- Create `agencies` table: id, name, slug, address, logo_url, primary_color, status
- Agency admin role: manage team, all listings, billing
- Public profile page: /agencies/{slug}
- Verification process: manual review for premium features

---

### Story 17.2: Realtor Team Management

As an **agency admin**,
I want to **manage my team of realtors**,
So that **they can create and manage listings**.

**Acceptance Criteria:**

**Given** an admin invites a realtor
**When** they enter email
**Then** invitation is sent
**And** realtor can join the agency

**Given** a realtor is part of an agency
**When** they create a listing
**Then** it's associated with both realtor and agency
**And** agency branding is applied

**Given** a realtor leaves the agency
**When** removed by admin
**Then** their listings can be reassigned or archived
**And** they lose agency access

**Technical Notes:**
- Create `agency_members` table: id, agency_id, user_id, role, joined_at
- Realtor roles: agent (own listings), senior (all listings), admin (full control)
- Listing ownership: realtor_id + agency_id
- Reassignment on departure

---

### Story 17.3: Shared Listing Management

As a **realtor**,
I want to **share listings with agency colleagues**,
So that **anyone can handle inquiries**.

**Acceptance Criteria:**

**Given** a realtor creates a listing
**When** they set visibility to "Agency"
**Then** all agency members can view and edit
**And** inquiries can be handled by any team member

**Given** an inquiry comes in
**When** any realtor responds
**Then** response is visible to all team members
**And** inquiry status is updated

**Given** multiple realtors collaborate
**When** changes are made
**Then** edit history shows who made changes
**And** conflicts are prevented via optimistic locking

**Technical Notes:**
- Listing visibility: personal, agency, public
- Inquiry assignment: pool, round-robin, or claimed
- Edit history in audit log
- Real-time collaboration indicators

---

### Story 17.4: External Listing Import

As a **realtor**,
I want to **import listings from external sources**,
So that **I don't have to manually re-enter data**.

**Acceptance Criteria:**

**Given** a realtor initiates import
**When** they select source (CSV, XML, or portal API)
**Then** the import wizard guides them through mapping
**And** preview shows parsed data

**Given** import mapping is configured
**When** import runs
**Then** listings are created in draft status
**And** import log shows success/failures

**Given** photos are included
**When** imported
**Then** images are downloaded and processed
**And** stored in system

**Technical Notes:**
- Import sources: CSV template, Sreality XML, custom API
- Field mapping: source field → system field
- Duplicate detection: address matching
- Batch processing for large imports

---

## Epic 18: Short-Term Rental Integration

**Goal:** Property managers sync with Airbnb/Booking.com and register guests.

**FRs covered:** FR84, FR85, FR86
**Estimate:** 2.5 weeks

### Story 18.1: Airbnb/Booking.com Calendar Sync

As a **property manager**,
I want to **sync calendars with Airbnb and Booking.com**,
So that **availability is always accurate**.

**Acceptance Criteria:**

**Given** a manager connects Airbnb account
**When** OAuth authorization completes
**Then** property mapping is initiated
**And** calendars start syncing

**Given** calendars are synced
**When** a booking is made on Airbnb
**Then** it appears in the system within 15 minutes
**And** availability is blocked

**Given** availability is changed in system
**When** sync runs
**Then** changes propagate to connected platforms
**And** double-booking is prevented

**Technical Notes:**
- Create `rental_connections` table: id, unit_id, platform, external_id, access_token, status
- Create `rental_bookings` table: id, connection_id, guest_name, check_in, check_out, external_booking_id
- iCal fallback if API not available
- Sync frequency: every 15 minutes

---

### Story 18.2: Guest Registration

As a **property manager**,
I want to **register guests for legal compliance**,
So that **I meet local regulations**.

**Acceptance Criteria:**

**Given** a booking arrives
**When** guest details are available
**Then** registration form is pre-filled
**And** manager can complete required fields

**Given** ID document is required
**When** guest provides it
**Then** OCR extracts details
**And** manager verifies

**Given** registration is complete
**When** saved
**Then** guest is registered in system
**And** ready for authority reporting

**Technical Notes:**
- Create `guest_registrations` table: id, booking_id, name, nationality, id_type, id_number, birth_date, registered_at
- ID document types: passport, ID card, driver's license
- OCR integration from Epic 7B/12
- Compliance with local hotel registration laws

---

### Story 18.3: Guest Report Generation

As a **property manager**,
I want to **generate guest reports for authorities**,
So that **I comply with reporting requirements**.

**Acceptance Criteria:**

**Given** reporting period ends
**When** manager generates report
**Then** all registered guests are included
**And** format matches authority requirements

**Given** report format varies by region
**When** generating
**Then** appropriate template is used
**And** data is validated against requirements

**Given** report is generated
**When** submitted
**Then** submission is logged
**And** confirmation stored

**Technical Notes:**
- Report formats: Slovak police XML, Czech UbyPort, generic CSV
- Automatic generation on configurable schedule
- Submission: manual download or direct API where available
- Retention: 5 years per regulations

---

### Story 18.4: Booking Calendar View

As a **property manager**,
I want to **view all bookings in a calendar**,
So that **I can manage occupancy across properties**.

**Acceptance Criteria:**

**Given** a manager opens the calendar view
**When** properties are selected
**Then** bookings are displayed on timeline
**And** color-coded by source (Airbnb, Booking, direct)

**Given** multiple units are managed
**When** viewing calendar
**Then** all units can be seen together
**And** filtered as needed

**Given** a gap exists in bookings
**When** identified
**Then** quick-block option is available
**And** can sync to platforms

**Technical Notes:**
- Calendar component: week/month view
- Multi-property timeline (Gantt-style)
- Drag-and-drop for manual bookings
- Quick actions: block, unblock, add maintenance

---

## Epic 19: Lease Management & Tenant Screening

**Goal:** Landlords screen tenants and manage full lease lifecycle.

**FRs covered:** FR87, FR88, FR89
**Estimate:** 2.5 weeks

### Story 19.1: Tenant Application Processing

As a **landlord**,
I want to **receive and review tenant applications**,
So that **I can select qualified tenants**.

**Acceptance Criteria:**

**Given** a listing allows applications
**When** a prospective tenant applies
**Then** they fill out application form
**And** application is submitted to landlord

**Given** an application is received
**When** landlord reviews
**Then** they see applicant details, references, income info
**And** can request additional documents

**Given** multiple applications exist
**When** comparing
**Then** side-by-side view is available
**And** scoring/ranking can be applied

**Technical Notes:**
- Create `tenant_applications` table: id, listing_id, applicant_id, status, submitted_at
- Create `application_documents` table: id, application_id, type, file_url
- Application status: submitted, reviewing, approved, rejected
- Privacy: applicant data visible only to listing owner

---

### Story 19.2: Tenant Screening

As a **landlord**,
I want to **screen potential tenants**,
So that **I can make informed decisions**.

**Acceptance Criteria:**

**Given** a landlord initiates screening
**When** applicant consents
**Then** background check is requested
**And** results returned within 24-48 hours

**Given** screening results are available
**When** reviewed
**Then** they include: credit check, reference verification, income verification
**And** risk assessment summary

**Given** screening indicates concerns
**When** displayed
**Then** specific items are highlighted
**And** landlord can request clarification

**Technical Notes:**
- Third-party screening service integration
- Consent required: GDPR compliant
- Results stored encrypted
- Retention: 30 days after decision (deleted if rejected)

---

### Story 19.3: Lease Creation & Signing

As a **landlord**,
I want to **create and sign leases digitally**,
So that **the process is efficient and documented**.

**Acceptance Criteria:**

**Given** a landlord creates a lease
**When** they select template and fill details
**Then** lease document is generated
**And** preview is available

**Given** lease is ready for signing
**When** sent to tenant
**Then** they receive notification
**And** can sign digitally

**Given** both parties sign
**When** signatures complete
**Then** lease is finalized
**And** PDF stored in documents

**Technical Notes:**
- Create `leases` table: id, unit_id, tenant_id, landlord_id, start_date, end_date, rent, status, document_id
- Lease templates: configurable per organization
- E-signature integration: DocuSign or built-in
- Legal validity per jurisdiction

---

### Story 19.4: Lease Lifecycle Management

As a **landlord**,
I want to **manage lease renewals and terminations**,
So that **I can maintain proper records**.

**Acceptance Criteria:**

**Given** a lease is approaching expiry
**When** 90 days before end date
**Then** landlord and tenant are notified
**And** renewal options are presented

**Given** landlord initiates renewal
**When** new terms are set
**Then** renewal document is generated
**And** sent for signing

**Given** lease is terminated
**When** termination is processed
**Then** reason is recorded
**And** unit status updated to available

**Technical Notes:**
- Lease status: draft, active, renewing, expired, terminated
- Renewal: creates new lease linked to original
- Termination types: end of term, early (mutual), early (breach)
- Move-out checklist integration

---

### Story 19.5: Lease Expiration Tracking & Reminders

As a **landlord**,
I want to **receive reminders about expiring leases**,
So that **I don't miss important dates**.

**Acceptance Criteria:**

**Given** leases exist in the system
**When** viewing dashboard
**Then** expiring leases (next 90 days) are highlighted
**And** count badge shows

**Given** expiration approaches
**When** configurable thresholds hit (90, 60, 30, 14 days)
**Then** reminders are sent
**And** action options included

**Given** multiple properties are managed
**When** generating report
**Then** all expiring leases are listed
**And** can be exported

**Technical Notes:**
- Reminder schedule: configurable per organization
- Notification channels: email, push, in-app
- Dashboard widget: expiring leases list
- Batch actions: bulk renewal, bulk notification

---

## Phase 3 & 4 Stories Summary

| Epic | Stories | FRs Covered |
|------|---------|-------------|
| 13 - AI Assistant & Automation | 7 | FR64-FR70 |
| 14 - IoT & Smart Building | 6 | FR71-FR75 |
| 10A-SSO - Cross-Platform SSO | 3 | FR7 (complete) |
| 15 - Property Listings | 4 | FR76-FR78 |
| 16 - Portal Search & Discovery | 4 | FR79-FR81 |
| 17 - Agency & Realtor Management | 4 | FR82-FR83 |
| 18 - Short-Term Rental | 4 | FR84-FR86 |
| 19 - Lease Management | 5 | FR87-FR89 |
| **Phase 3 Total** | **13** | **12 FRs** |
| **Phase 4 Total** | **24** | **14 FRs** |

---

## Phase 3 Sprint Plan

| Sprint | Epics | Stories | Rationale |
|--------|-------|---------|-----------|
| 3A | Epic 13 (Stories 1-4) | 4 | AI foundation (chatbot, sentiment, prediction, summarization) |
| 3B | Epic 13 (Stories 5-7) | 3 | AI completion (search, workflows) |
| 3C | Epic 14 (Stories 1-3) | 3 | IoT foundation (sensors, data, dashboards) |
| 3D | Epic 14 (Stories 4-6) | 3 | IoT completion (alerts, correlation, config) |

**Dependency Chain:** Phase 2 → Epic 7B (for RAG) → Epic 13 → Epic 14

---

## Phase 4 Sprint Plan

| Sprint | Epics | Stories | Rationale |
|--------|-------|---------|-----------|
| 4A | Epic 10A-SSO + Epic 15 (1-2) | 5 | SSO completion + listing foundation |
| 4B | Epic 15 (3-4) + Epic 16 (1-2) | 4 | Syndication + search foundation |
| 4C | Epic 16 (3-4) + Epic 17 (1-2) | 4 | Search completion + agency foundation |
| 4D | Epic 17 (3-4) + Epic 18 (1-2) | 4 | Agency completion + rental integration start |
| 4E | Epic 18 (3-4) + Epic 19 (1-2) | 4 | Rental completion + lease start |
| 4F | Epic 19 (3-5) | 3 | Lease completion |

**Dependency Chain:** Epic 10A-SSO → Epic 15 → Epic 16 → Epic 17 (parallel: Epic 18, 19)

---

# Phase 5 Stories: Facilities, Compliance, Community & Analytics (Backfill)

> **Backfill — documentation only, no feature code.** Phase 5 catalogs six "keeper" clusters
> of backend that already ship in `backend/servers/api-server/src/routes/` with real migrations
> under `backend/crates/db/migrations/`, but had no epic/story/FR entry. Authored per the board's
> **Option C** decision on **PAP-17** (see the *Phase-5 Plan — Hybrid (Option C)* plan doc).
> Selection rule: a module is a **keeper** if it is substantial, backed by a real migration, and
> either already reachable (routed web group or mobile screen) or a clear product roadmap area.
> Pure 501 scaffolds (`competitive.rs`, `public_api.rs`, `vendor_portal.rs`) and platform-infra
> modules are **quarantined**, not cataloged (tracked separately under the Option-C "Child B" issue).
>
> Each epic records: **Goal**, **FRs covered**, **Backed by** (route module line-counts + migrations),
> a **UI status** line (with the wiring gap where the surface is dead-code), and stories. The
> flagship UI-wiring gaps already have dedicated issues (PAP-18…26); epics here reference them
> rather than duplicate them.

## Epic 25: Facilities & Operations

**Goal:** Managers run building operations end-to-end — work orders, vendor management, daily
operations tasks, package/visitor handling, and outage tracking.

**FRs covered:** FR102, FR103, FR104, FR105, FR106, FR107
**Estimate:** backend shipped; Phase-5 backfill (UI partial)

**Backed by (code evidence):**
- Routes: `work_orders.rs` (935 L), `vendors.rs` (989 L), `operations.rs` (1443 L),
  `package_visitor.rs` (1175 L), `outages.rs` (932 L).
- Migrations: `00053_create_work_orders.sql`, `00054_create_vendors.sql`,
  `00075_create_operations.sql`, `00068_create_package_visitor_management.sql`,
  `00086_create_outages.sql`.

**UI status:** `outages/` (373 L group) and `facilities/` ppt-web groups are **routed**;
mobile has `outages/` screens. The standalone `vendors`/`package_visitor` surfaces are
thin/back-office only. No new dead-code gap to flag here beyond the routed surfaces.

### Story 25.1: Work Order Lifecycle

As a **building manager**, I want to **create, assign and track maintenance work orders**,
So that **repairs move through a clear status lifecycle to completion**.

**Acceptance Criteria:**
**Given** a fault or planned task **When** a manager raises a work order with priority and assignee
**Then** it is persisted with an open status **And** transitions (assigned → in-progress → done) are auditable.

**Technical Notes:** `work_orders.rs` handlers; `00053_create_work_orders.sql` schema (status, priority, assignee, building_id).

### Story 25.2: Vendor & Supplier Directory

As a **manager**, I want to **maintain a directory of vendors with ratings**,
So that **work orders can be assigned to qualified suppliers**.

**Acceptance Criteria:**
**Given** a vendor record **When** a manager assigns it to a work order **Then** the assignment and contact details are linked **And** the vendor's history is visible.

**Technical Notes:** `vendors.rs`; `00054_create_vendors.sql`. (Distinct from the 501-stub `vendor_portal.rs` external facade, which is quarantined.)

### Story 25.3: Daily Operations Tasks & Checklists

As **operations staff**, I want to **run recurring operational tasks and checklists**,
So that **routine building upkeep is tracked and not missed**.

**Acceptance Criteria:**
**Given** a recurring operations task **When** staff complete checklist items **Then** completion is timestamped and reportable.

**Technical Notes:** `operations.rs` (1443 L); `00075_create_operations.sql`.

### Story 25.4: Package & Visitor Management

As **front-desk staff**, I want to **log incoming packages and register visitors**,
So that **deliveries and access are tracked per resident**.

**Acceptance Criteria:**
**Given** a package arrives **When** logged against a unit/resident **Then** the resident is notified and pickup is recorded; **Given** a visitor **When** registered **Then** entry/exit is tracked.

**Technical Notes:** `package_visitor.rs` (1175 L); `00068_create_package_visitor_management.sql`.

### Story 25.5: Outage Tracking & Resident Notification

As a **manager**, I want to **record planned/unplanned outages and notify affected residents**,
So that **residents have advance notice of service interruptions**.

**Acceptance Criteria:**
**Given** an outage **When** created with scope and window **Then** affected residents are notified **And** the outage status is shown in the routed `outages/` UI.

**Technical Notes:** `outages.rs`; `00086_create_outages.sql`; ppt-web `outages/` group routed; mobile `outages/` screens.

---

## Epic 26: Disputes, Legal, Insurance & Compliance

**Goal:** Managers handle the regulatory and contentious side of property management — disputes,
rule violations, legal records, insurance, compliance obligations, regional rules, emergency
response, and AML/DSA screening.

**FRs covered:** FR108, FR109, FR110, FR111, FR112, FR113, FR114, FR115
**Estimate:** backend shipped; Phase-5 backfill — **`aml_dsa.rs` flagged for owner + security review**

**Backed by (code evidence):**
- Routes: `disputes.rs` (1581 L), `violations.rs`, `legal.rs` (1134 L), `insurance.rs`,
  `compliance.rs`, `regional_compliance.rs`, `emergency.rs` (1634 L), **`aml_dsa.rs` (1963 L)**.
- Migrations: `00076_create_disputes.sql`, `00115_rls_disputes.sql`,
  `00160_disputes_mediation_notes.sql`, `00098_create_violations.sql`,
  `00058_create_legal_compliance.sql`, `00055_create_insurance.sql`,
  `00056_create_emergency.sql`.

**UI status:** `disputes/` (482 L group) and `emergency/` ppt-web groups are **routed**.
`insurance/` and `compliance/` ppt-web dirs exist but are **dead code (unrouted)** — wiring
gap recorded; this is part of the broad UI-wiring backlog (see `EPIC_STORY_STATUS.md` §4.B),
not a duplicate of a flagship gap issue.

> ⚠️ **Security/ownership flag — `aml_dsa.rs` (1963 L).** This is a large AML/EDD + DSA module
> with **no named owner and no security review on record**. It is folded into this epic for
> catalog completeness only. **Action:** assign a named owner and run a security review (least
> privilege, untrusted-input validation at the boundary, audit logging) **before** FR115 is
> treated as production-ready. Owner: **Atlas to assign** (CTO recommends a dedicated child issue).

### Story 26.1: Dispute Intake & Mediation

As a **resident or manager**, I want to **open and mediate disputes with notes**,
So that **conflicts are tracked to resolution**.

**Acceptance Criteria:**
**Given** a dispute **When** opened and mediation notes are added **Then** status and notes persist and are auditable.

**Technical Notes:** `disputes.rs`; `00076_create_disputes.sql` + `00160_disputes_mediation_notes.sql`; routed `disputes/` group.

### Story 26.2: Violation Tracking & Enforcement

As a **manager**, I want to **record rule violations and escalate enforcement**,
So that **community rules are enforced consistently**.

**Acceptance Criteria:**
**Given** a violation **When** logged against a unit/resident **Then** escalation steps and resolution are tracked.

**Technical Notes:** `violations.rs`; `00098_create_violations.sql`.

### Story 26.3: Legal Records & Insurance

As a **manager**, I want to **store legal documents and manage insurance policies/claims**,
So that **contracts and coverage are centrally tracked**.

**Acceptance Criteria:**
**Given** a legal document or policy **When** recorded **Then** it is retrievable with metadata; **Given** an insurance claim **When** filed **Then** its status is tracked.

**Technical Notes:** `legal.rs`, `insurance.rs`; `00058_create_legal_compliance.sql`, `00055_create_insurance.sql`.

### Story 26.4: Compliance & Regional Rules

As a **manager**, I want to **track compliance obligations and apply region-specific rules**,
So that **the platform meets local regulatory requirements**.

**Acceptance Criteria:**
**Given** a compliance obligation **When** due **Then** evidence/reporting is generated; **Given** a region **When** rules apply **Then** region-specific handling is enforced.

**Technical Notes:** `compliance.rs`, `regional_compliance.rs`.

### Story 26.5: Emergency Response & AML/DSA Screening

As a **manager/compliance officer**, I want to **run emergency workflows and AML/EDD screening**,
So that **incidents are handled and legal screening obligations are met**.

**Acceptance Criteria:**
**Given** an emergency **When** triggered **Then** response workflow + broadcast run; **Given** an entity **When** AML/EDD screening runs **Then** a screening result is recorded.

**Technical Notes:** `emergency.rs` (`00056_create_emergency.sql`); `aml_dsa.rs` — **gated on the security/ownership flag above**.

---

## Epic 27: Community, Marketplace & News

**Goal:** Residents engage with each other and the building — community marketplace, groups/events,
news/media, and a neighbour directory. **Strongest keeper: fully reachable on web + mobile.**

**FRs covered:** FR116, FR117, FR118, FR119, FR120
**Estimate:** backend + UI shipped; Phase-5 backfill (catalog only)

**Backed by (code evidence):**
- Routes: `marketplace.rs` (1933 L), `community.rs`, `news_articles.rs` (1060 L), `neighbors.rs`.
- Migrations: `00103_create_marketplace.sql`, `00121_rls_marketplace.sql`,
  `00064_community_features.sql`, `00114_rls_community_features.sql`, `00069_create_news_media.sql`.

**UI status:** **Fully reachable** — ppt-web `/community/marketplace|events|groups`, `news/`,
`neighbors/` are routed; mobile has `news/`, `neighbors/` screens. No wiring gap.

### Story 27.1: Community Marketplace

As a **resident**, I want to **list, browse and trade items in a community marketplace**,
So that **neighbours can exchange goods within the building**.

**Acceptance Criteria:**
**Given** a listing **When** posted **Then** it appears in the marketplace with moderation; **Given** interest **When** a resident responds **Then** a conversation is started.

**Technical Notes:** `marketplace.rs` (1933 L); `00103_create_marketplace.sql` + RLS `00121`.

### Story 27.2: Community Groups & Events

As a **resident**, I want to **join groups and events**,
So that **I can participate in building community life**.

**Acceptance Criteria:**
**Given** a group/event **When** a resident joins/RSVPs **Then** membership/attendance is tracked.

**Technical Notes:** `community.rs`; `00064_community_features.sql` + RLS `00114`; routed `/community/*`.

### Story 27.3: News & Media Publishing

As a **manager**, I want to **publish news articles and media to residents**,
So that **building announcements reach the community**.

**Acceptance Criteria:**
**Given** an article **When** published **Then** residents see it in the routed `news/` UI with media attachments.

**Technical Notes:** `news_articles.rs` (1060 L); `00069_create_news_media.sql`.

### Story 27.4: Neighbour Directory & Connections

As a **resident**, I want to **connect with neighbours via a directory**,
So that **residents can find and contact each other (privacy-respecting)**.

**Acceptance Criteria:**
**Given** opt-in **When** a resident appears in the directory **Then** connections respect privacy settings (ties to FR92).

**Technical Notes:** `neighbors.rs`; routed `neighbors/` web + mobile.

---

## Epic 28: Investor & Portfolio Analytics

**Goal:** Owners, investors and boards get a financial/analytics layer above Epic 11 — investor
portal, owner/portfolio analytics, performance metrics, valuations, board meetings, and
subscription billing.

**FRs covered:** FR121, FR122, FR123, FR124, FR125, FR126, FR127
**Estimate:** backend shipped; **UI is dead-code → wiring gap recorded**

**Backed by (code evidence):**
- Routes: `investor_portal.rs`, `owner_analytics.rs`, `portfolio_analytics.rs`,
  `portfolio_performance.rs`, `property_valuation.rs`, `board_meetings.rs`, `subscriptions.rs` (1681 L).
- Migrations: `00096_create_investor_portal.sql`, `00077_create_owner_analytics.sql`,
  `00116_rls_owner_analytics.sql`, `00091_create_portfolio_analytics.sql`,
  `00100_create_portfolio_performance.sql`, `00120_rls_portfolio_performance.sql`,
  `00146_fix_perf_portfolios_super_admin_bypass.sql`, `00095_create_property_valuation.sql`,
  `00099_create_board_meetings.sql`, `00059_create_subscription_billing.sql`.

> **UI-wiring gap (recorded).** The ppt-web `subscription/`, `portfolio-performance/` dirs and
> related investor surfaces are **dead code — built but not mounted** in `AppRoutes.tsx`. Backend
> + schema are complete; the web surface is unreachable. This is a wiring gap, not a backend gap
> (see `EPIC_STORY_STATUS.md` §4.B). Wiring is out of scope for this docs-only backfill.

### Story 28.1: Investor Portal

As an **investor**, I want to **view my portfolio holdings and returns**,
So that **I can monitor my property investments**.

**Acceptance Criteria:**
**Given** investor holdings **When** the portal loads **Then** holdings and returns are shown (backend ready; web surface unrouted — gap recorded).

**Technical Notes:** `investor_portal.rs`; `00096_create_investor_portal.sql`.

### Story 28.2: Owner & Portfolio Analytics

As an **owner/manager**, I want to **view per-property and cross-property analytics**,
So that **I understand performance across the portfolio**.

**Acceptance Criteria:**
**Given** properties **When** analytics run **Then** owner-level and portfolio-level metrics are computed with RLS isolation.

**Technical Notes:** `owner_analytics.rs`, `portfolio_analytics.rs`; `00077`/`00116`, `00091`.

### Story 28.3: Portfolio Performance Metrics

As a **manager**, I want to **track portfolio performance over time**,
So that **trends and benchmarks are visible**.

**Acceptance Criteria:**
**Given** historical data **When** performance is computed **Then** time-series metrics are available; super-admin bypass is correctly scoped.

**Technical Notes:** `portfolio_performance.rs`; `00100`/`00120` + `00146` (super-admin bypass fix).

### Story 28.4: Property Valuation & Board Meetings

As a **manager/board**, I want to **track valuations and run board meetings**,
So that **asset values and governance decisions are recorded**.

**Acceptance Criteria:**
**Given** a property **When** valued **Then** the estimate is tracked; **Given** a board meeting **When** scheduled **Then** agenda/minutes persist.

**Technical Notes:** `property_valuation.rs` (`00095`), `board_meetings.rs` (`00099`).

### Story 28.5: Subscription & Billing

As an **owner**, I want to **manage platform subscription plans and billing**,
So that **organisations can pay for the tier they use**.

**Acceptance Criteria:**
**Given** a plan **When** subscribed **Then** billing state is tracked (backend ready; web `subscription/` dir unrouted — gap recorded).

**Technical Notes:** `subscriptions.rs` (1681 L); `00059_create_subscription_billing.sql`.

---

## Epic 29: ESG & Building Certifications

**Goal:** Managers meet sustainability and certification obligations — ESG reporting and tracking
of building certifications and their renewals.

**FRs covered:** FR128, FR129, FR130
**Estimate:** backend shipped; Phase-5 backfill

**Backed by (code evidence):**
- Routes: `esg_reporting.rs`, `building_certifications.rs`.
- Migrations: `00093_create_esg_reporting.sql`, `00094_create_building_certifications.sql`.

**UI status:** Back-office/reporting surface; no dedicated routed ppt-web group yet — wiring is a
future ticket only if the epic is prioritised (no dead-code dir to flag).

### Story 29.1: ESG Reporting Dashboard

As a **manager**, I want to **produce ESG/sustainability reports**,
So that **the organisation meets increasingly-mandated ESG disclosure**.

**Acceptance Criteria:**
**Given** building/energy data **When** an ESG report is generated **Then** sustainability KPIs are aggregated into a report.

**Technical Notes:** `esg_reporting.rs`; `00093_create_esg_reporting.sql`.

### Story 29.2: Building Certification Tracking

As a **manager**, I want to **track building certifications and renewal dates**,
So that **certifications stay valid and renewals are not missed**.

**Acceptance Criteria:**
**Given** a certification **When** recorded with an expiry **Then** renewal reminders can be raised.

**Technical Notes:** `building_certifications.rs`; `00094_create_building_certifications.sql`.

### Story 29.3: Energy/Sustainability KPI Monitoring

As a **manager**, I want to **monitor energy and sustainability KPIs**,
So that **ESG reports are backed by tracked metrics**.

**Acceptance Criteria:**
**Given** sustainability metrics **When** captured over time **Then** they feed the ESG report (ties to Epic 14 IoT energy data where available).

**Technical Notes:** `esg_reporting.rs` (consumes energy/sensor inputs).

---

## Epic 30: Forms, Registry & Government Portal

**Goal:** Users build and submit dynamic forms, the system maintains official registries, and it
integrates with government portals for filings and lookups.

**FRs covered:** FR131, FR132, FR133, FR134
**Estimate:** backend shipped; Phase-5 backfill

**Backed by (code evidence):**
- Routes: `forms.rs` (1873 L), `registry.rs` (1093 L), `government_portal.rs` (987 L).
- Migrations: `00066_create_forms.sql`, `00062_government_portal_integration.sql`.

**UI status:** `forms/` is **mobile-reachable** (RN `forms/` screens). `registry/` and
`government-portal/` ppt-web dirs exist but are **dead code (unrouted)** — wiring gap recorded
(see `EPIC_STORY_STATUS.md` §4.B). Gov/registry integrations are real external surfaces.

### Story 30.1: Dynamic Form Builder & Submission

As a **user**, I want to **build, submit and process dynamic forms**,
So that **structured intake (applications, requests) is captured**.

**Acceptance Criteria:**
**Given** a form definition **When** a user submits **Then** the submission is validated and stored; mobile `forms/` screens consume the API.

**Technical Notes:** `forms.rs` (1873 L); `00066_create_forms.sql`.

### Story 30.2: Official Registry

As a **system/manager**, I want to **maintain official registries of records**,
So that **regulated record-keeping is supported**.

**Acceptance Criteria:**
**Given** a registry record **When** created/updated **Then** it is persisted with audit history (web `registry/` dir unrouted — gap recorded).

**Technical Notes:** `registry.rs` (1093 L).

### Story 30.3: Government Portal Integration

As a **system**, I want to **integrate with government portals for filings/lookups**,
So that **regulatory submissions and validations are automated**.

**Acceptance Criteria:**
**Given** a filing/lookup **When** sent to the gov portal **Then** the response is recorded and submissions are validated against government schemas.

**Technical Notes:** `government_portal.rs` (987 L); `00062_government_portal_integration.sql`. Secure-by-default: validate all untrusted gov-portal responses at the boundary.

---

## Phase 5 Stories Summary

| Epic | Name | Stories | FRs | UI status |
|------|------|---------|-----|-----------|
| 25 | Facilities & Operations | 5 | FR102-107 | 🟡 partial (outages/facilities routed) |
| 26 | Disputes, Legal, Insurance & Compliance | 5 | FR108-115 | 🟡 partial (disputes/emergency routed; insurance/compliance dead-code) — **aml_dsa.rs flagged** |
| 27 | Community, Marketplace & News | 4 | FR116-120 | ✅ fully reachable (web + mobile) |
| 28 | Investor & Portfolio Analytics | 5 | FR121-127 | ❌ web dead-code (wiring gap recorded) |
| 29 | ESG & Building Certifications | 3 | FR128-130 | 🟡 back-office (no routed group yet) |
| 30 | Forms, Registry & Government Portal | 3 | FR131-134 | 🟡 partial (forms mobile-reachable; registry/gov dead-code) |
| **Phase 5 Total** | | **25** | **33 FRs (FR102-134)** | |

**Quarantined (NOT cataloged, per Option C):** pure 501 scaffolds `competitive.rs` (removed),
`public_api.rs`, `vendor_portal.rs`; and platform/infra modules (`infrastructure`,
`feature_packages`, `reports`, `migration`, `tenant_config`, `api_ecosystem`, `multi_currency`,
`data_residency`). These are not product epics — they carry on the Option-C "Child B" cleanup issue.

---

## Complete Summary

| Phase | Epics | Stories | FRs | Weeks |
|-------|-------|---------|-----|-------|
| MVP (Phase 1) | 12 | 74 | 63 | ~16 |
| Phase 2 | 5 | 23 | 13 | ~5 |
| Phase 3 | 2 | 13 | 12 | ~4 |
| Phase 4 | 6 | 24 | 14 | ~8 |
| Phase 5 (backfill) | 6 | 25 | 33 | shipped (BE) |
| **Total** | **31** | **159** | **134** | **~33+** |

Phases 1-4 cover all 101 original PRD Functional Requirements. **Phase 5 (Epics 25-30)** is a
documentation-only backfill of already-shipped backend modules (FR102-134) authored per PAP-17
Option C — see the "Phase 5 Stories" section above and the reconciliation in
`docs/EPIC_STORY_STATUS.md` §2.

---

## Continuation

This document is the **authoritative catalog**: Epics 1-19 (Phases 1-4) **plus the Phase 5
backfill (Epics 25-30, FR102-134)** authored inline above per PAP-17 Option C.

> **Superseded aspirational backlog.** The sibling files `epics-002.md` … `epics-015.md`
> contain an earlier, never-delivered planning backlog that reuses epic numbers (e.g. a
> *different* "Epic 25 — Legal Document & Compliance" in `epics-002.md`, and epics up to
> ~150 across `epics-015.md`) and a conflicting "Phase 5: Epics 20-23 / FR102-FR125" scheme.
> Those numberings are **not** authoritative and are not tracked in
> `docs/EPIC_STORY_STATUS.md`. Where this document and a continuation file disagree, **this
> document wins.** The continuation files are retained as raw brainstorming, pending a
> separate cleanup decision.
