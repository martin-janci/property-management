# System Architecture

This document defines the architecture for the Property Management System (PPT) and Reality Portal, including service boundaries, technology decisions, API contracts, and database design.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Architecture Decision Records](#architecture-decision-records)
3. [Service Architecture](#service-architecture)
4. [Module Structure](#module-structure)
5. [API Contracts](#api-contracts)
6. [Database Architecture](#database-architecture)
7. [Infrastructure](#infrastructure)
8. [Security Architecture](#security-architecture)
9. [Observability](#observability)

---

## Architecture Overview

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                                    CLIENTS                                           │
├─────────────────┬─────────────────┬─────────────────┬───────────────────────────────┤
│   ppt-web       │  reality-web    │    mobile       │   mobile-native               │
│   (React SPA)   │  (Next.js SSR)  │ (React Native)  │   (KMP)                       │
└────────┬────────┴────────┬────────┴────────┬────────┴──────────────┬────────────────┘
         │                 │                 │                       │
         │ HTTPS           │ HTTPS           │ HTTPS                 │ HTTPS
         ▼                 ▼                 ▼                       ▼
┌─────────────────────────────────────────────────────────────────────────────────────┐
│                              API GATEWAY / LOAD BALANCER                             │
│                           (Nginx / AWS ALB / Cloudflare)                             │
├─────────────────────────────────────────────────────────────────────────────────────┤
│  • SSL Termination       • Rate Limiting        • Request Routing                    │
│  • DDoS Protection       • CORS Handling        • Health Checks                      │
└────────────────────────────────────┬────────────────────────────────────────────────┘
                                     │
         ┌───────────────────────────┴───────────────────────────┐
         │                                                       │
         ▼                                                       ▼
┌─────────────────────────────────┐         ┌─────────────────────────────────────────┐
│         API SERVER              │         │           REALITY SERVER                 │
│         (Port 8080)             │         │           (Port 8081)                    │
│                                 │         │                                          │
│  Property Management Backend    │         │  Reality Portal Backend                  │
│  ┌───────────────────────────┐  │         │  ┌─────────────────────────────────────┐ │
│  │ Modules:                  │  │         │  │ Modules:                            │ │
│  │ • Auth & IAM              │  │         │  │ • Portal Auth (OAuth)               │ │
│  │ • Organizations           │  │         │  │ • Listings                          │ │
│  │ • Buildings & Units       │  │         │  │ • Agencies                          │ │
│  │ • Faults                  │  │         │  │ • Realtors                          │ │
│  │ • Voting                  │  │         │  │ • Search                            │ │
│  │ • Documents               │  │         │  │ • Favorites                         │ │
│  │ • Messages                │  │         │  │ • Inquiries                         │ │
│  │ • Financial               │  │         │  │ • Import/Export                     │ │
│  │ • Rentals                 │  │         │  └─────────────────────────────────────┘ │
│  │ • Notifications           │  │         │                                          │
│  └───────────────────────────┘  │         │                                          │
└────────────────┬────────────────┘         └──────────────────┬──────────────────────┘
                 │                                              │
                 └──────────────────────┬───────────────────────┘
                                        │
         ┌──────────────────────────────┼──────────────────────────────┐
         │                              │                              │
         ▼                              ▼                              ▼
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────────────┐
│    PostgreSQL       │    │       Redis         │    │     Message Queue           │
│    (Primary DB)     │    │   (Cache/Sessions)  │    │   (RabbitMQ / SQS)          │
└─────────────────────┘    └─────────────────────┘    └─────────────────────────────┘
                                                                   │
                                        ┌──────────────────────────┴───────────────┐
                                        │                                          │
                                        ▼                                          ▼
                           ┌─────────────────────────┐            ┌─────────────────────────┐
                           │    WORKER SERVICES      │            │   EXTERNAL SERVICES     │
                           │                         │            │                         │
                           │  • Notification Worker  │            │  • Email (SendGrid)     │
                           │  • AI/ML Worker         │            │  • SMS Gateway          │
                           │  • Import Worker        │            │  • Push (FCM/APNs)      │
                           │  • Scheduler            │            │  • Payment (Stripe)     │
                           │                         │            │  • Storage (S3)         │
                           └─────────────────────────┘            │  • AI/ML APIs           │
                                                                  │  • Airbnb/Booking       │
                                                                  │  • OAuth Providers      │
                                                                  └─────────────────────────┘
```

### Design Principles

1. **Modular Monolith** - Start with well-structured monoliths, not microservices
2. **API-First** - OpenAPI specification as the contract
3. **Event-Driven** - Async communication via message queues
4. **Multi-Tenant** - Organization-level data isolation
5. **Offline-First** - Mobile apps work without connectivity
6. **Security by Design** - Zero-trust, encryption at rest and in transit

---

## Architecture Decision Records

### ADR-001: Modular Monolith over Microservices

**Status:** Accepted

**Context:**
The system has 508 use cases across 51 categories with complex domain logic. The team is small (< 10 developers initially).

**Decision:**
Implement as two modular monoliths (api-server, reality-server) rather than microservices.

**Rationale:**
- **Simplicity**: Easier deployment, debugging, and local development
- **Performance**: No network overhead for internal calls
- **Transactions**: ACID transactions across modules
- **Refactoring**: Easier to move boundaries as domain understanding improves
- **Team Size**: Microservices require dedicated teams per service

**Consequences:**
- Must maintain strict module boundaries within monolith
- Cannot scale individual modules independently
- All modules share same deployment lifecycle

**Migration Path:**
If needed, extract high-load modules (Notifications, AI/ML) to separate services later.

---

### ADR-002: Two Backend Servers

**Status:** Accepted

**Context:**
The system serves two distinct user bases with different requirements:
- PPT: Authenticated users (owners, tenants, managers) with multi-tenancy
- Reality Portal: Public users browsing listings, realtors managing properties

**Decision:**
Implement as two separate backend servers sharing a database.

**Rationale:**
- **Security Boundary**: Portal has different auth model (OAuth, public access)
- **Scaling**: Portal may need independent scaling during high traffic
- **SEO**: Reality Portal requires SSR for search engine optimization
- **Shared Data**: Listings created in PPT should appear on portal

**Consequences:**
- Shared database requires careful schema design
- Some code duplication for shared models
- Need to manage two deployment pipelines

---

### ADR-003: Rust for Backend Services

**Status:** Accepted

**Context:**
Need a performant, type-safe language for backend services.

**Decision:**
Use Rust with Axum framework for both api-server and reality-server.

**Rationale:**
- **Performance**: Near C performance with zero-cost abstractions
- **Safety**: Memory safety and thread safety guarantees
- **Type System**: Powerful type system catches errors at compile time
- **Async**: First-class async support with Tokio
- **Ecosystem**: Growing ecosystem for web services (Axum, SQLx, SeaORM)

**Consequences:**
- Steeper learning curve for new developers
- Longer compilation times
- Smaller talent pool than Go/Node.js

---

### ADR-004: PostgreSQL as Primary Database

**Status:** Accepted

**Context:**
Need a relational database that supports:
- Complex queries across related entities
- JSON for flexible data
- Full-text search
- Geospatial queries
- Row-level security for multi-tenancy

**Decision:**
Use PostgreSQL 16+ as the primary database.

**Rationale:**
- **Features**: JSONB, full-text search, PostGIS, RLS
- **Reliability**: Battle-tested, excellent durability
- **Extensions**: pgvector for AI embeddings, pg_cron for scheduling
- **Performance**: Excellent query optimizer, parallel queries
- **Hosting**: Available on all major cloud providers

**Consequences:**
- Single database may become bottleneck at scale
- Need to plan for read replicas
- Schema migrations require careful planning

---

### ADR-005: Event-Driven Architecture with Message Queue

**Status:** Accepted

**Context:**
Many operations require async processing:
- Sending notifications (email, SMS, push)
- AI/ML processing
- External API integrations
- Scheduled tasks

**Decision:**
Use RabbitMQ (self-hosted) or AWS SQS (cloud) for async messaging.

**Rationale:**
- **Decoupling**: Services don't need to know about consumers
- **Reliability**: Messages persist until processed
- **Scalability**: Can scale workers independently
- **Retry**: Built-in retry mechanisms

**Consequences:**
- Additional infrastructure to manage
- Eventual consistency for async operations
- Need to handle idempotency

---

### ADR-006: Redis for Caching and Sessions

**Status:** Accepted

**Context:**
Need fast access to:
- User sessions
- Cached API responses
- Rate limiting counters
- Real-time presence data

**Decision:**
Use Redis 7+ for caching, sessions, and ephemeral data.

**Rationale:**
- **Speed**: In-memory, sub-millisecond latency
- **Data Structures**: Strings, hashes, sorted sets, streams
- **Pub/Sub**: Real-time messaging for WebSocket
- **TTL**: Automatic expiration for sessions and cache

**Consequences:**
- Data loss on restart (acceptable for cache)
- Memory-bound, need to size appropriately
- Need Redis Cluster for high availability

---

### ADR-007: S3-Compatible Object Storage

**Status:** Accepted

**Context:**
Need to store:
- User uploads (documents, photos)
- Generated files (PDFs, exports)
- AI/ML model artifacts

**Decision:**
Use S3-compatible storage (AWS S3, MinIO for self-hosted).

**Rationale:**
- **Scalability**: Virtually unlimited storage
- **Cost**: Pay for what you use
- **CDN Integration**: Easy to put behind CDN
- **Compatibility**: S3 API is industry standard

**Consequences:**
- Files stored separately from database
- Need to handle orphaned files
- Presigned URLs for secure access

---

### ADR-008: WebSocket for Real-Time Features

**Status:** Implemented

**Owner:** pm-backend (Epic 2B — story 2B-C.1)

**Implemented:** 2026-05-25 — PR #472 (WebSocket realtime sync, merged to dev).
Ownership was formally confirmed by pm-tech-lead per action `pm-tech-lead-confirm-websocket-infra-ownership`; delivery was explicitly scheduled as story 2B-C.1 under DEC-001, not implicitly assumed.

**Context:**
Need real-time updates for:
- Live voting results
- Messaging and typing indicators
- Fault status updates
- Presence indicators
- Notification preference sync (8A.3)
- Direct messaging realtime (6-5)

**Decision:**
Implement WebSocket server using Axum's WebSocket support.

**Rationale:**
- **Performance**: Full-duplex, low latency
- **Efficiency**: No polling overhead
- **Browser Support**: Universal support

**Consequences:**
- Stateful connections require sticky sessions
- Need fallback for WebSocket-blocked networks
- Connection management complexity

**Delivery note:**
The Axum WS upgrade handler, connection registry, and pub/sub bridge were delivered in PR #472 as part of Epic 2B.
This unblocked: 8A.3 WS half (preference sync), 6-5 direct messaging realtime, and future DM slice.
Remaining 8A.3 work: mobile-push leg (FCM/APNs registration — tracked as `gap-8a-3-mobile-push`).

---

### ADR-009: TypeSpec for API Specification

**Status:** Accepted

**Context:**
Need to maintain API contracts between:
- Backend servers
- Frontend applications (React, React Native, KMP)
- Third-party integrations

**Decision:**
Use TypeSpec to define APIs, generate OpenAPI 3.1 specs.

**Rationale:**
- **DRY**: Single source of truth
- **Type Safety**: Compile-time validation
- **SDK Generation**: Auto-generate clients for TS, Kotlin
- **Documentation**: Auto-generate API docs

**Consequences:**
- Learning curve for TypeSpec
- Build step to generate OpenAPI
- Need to keep generated files in sync

---

### ADR-010: Multi-Tenancy with Row-Level Security

**Status:** Accepted

**Context:**
Organizations must be completely isolated:
- Data cannot leak between organizations
- Each organization manages their own users
- Super admin can access all organizations

**Decision:**
Implement multi-tenancy using PostgreSQL Row-Level Security (RLS).

**Rationale:**
- **Enforced at DB Level**: Cannot bypass in application code
- **Single Schema**: Simpler than schema-per-tenant
- **Performance**: Minimal overhead with proper indexing
- **Flexibility**: Policies can be complex

**Consequences:**
- RLS policies must be tested carefully
- Connection pooling must set tenant context
- Migrations apply to all tenants

---

## Service Architecture

### API Server (api-server)

**Port:** 8080
**Purpose:** Property Management backend for authenticated users

```
api-server/
├── src/
│   ├── main.rs              # Entry point
│   ├── config/              # Configuration
│   │   ├── mod.rs
│   │   ├── database.rs
│   │   ├── redis.rs
│   │   └── secrets.rs
│   │
│   ├── modules/             # Feature modules
│   │   ├── auth/            # Authentication & Authorization
│   │   │   ├── mod.rs
│   │   │   ├── handlers.rs
│   │   │   ├── service.rs
│   │   │   ├── models.rs
│   │   │   ├── middleware.rs
│   │   │   └── jwt.rs
│   │   │
│   │   ├── organizations/   # Multi-tenancy
│   │   │   ├── mod.rs
│   │   │   ├── handlers.rs
│   │   │   ├── service.rs
│   │   │   ├── models.rs
│   │   │   └── tenant.rs    # Tenant context
│   │   │
│   │   ├── buildings/       # Buildings & Units
│   │   ├── faults/          # Fault reporting
│   │   ├── voting/          # Voting system
│   │   ├── documents/       # Document management
│   │   ├── messages/        # Messaging
│   │   ├── financial/       # Invoices, payments
│   │   ├── rentals/         # Short-term rentals
│   │   ├── notifications/   # Push/email/SMS
│   │   ├── meters/          # Meter readings
│   │   ├── reports/         # Analytics & reports
│   │   └── admin/           # System admin
│   │
│   ├── shared/              # Shared utilities
│   │   ├── database/        # DB connection, migrations
│   │   ├── cache/           # Redis client
│   │   ├── queue/           # Message queue
│   │   ├── storage/         # S3 client
│   │   ├── errors/          # Error types
│   │   └── utils/           # Common utilities
│   │
│   └── websocket/           # WebSocket server
│       ├── mod.rs
│       ├── hub.rs
│       └── handlers.rs
│
├── migrations/              # Database migrations
├── tests/                   # Integration tests
└── Cargo.toml
```

### Reality Server (reality-server)

**Port:** 8081
**Purpose:** Reality Portal backend for public and realtor users

```
reality-server/
├── src/
│   ├── main.rs
│   ├── config/
│   │
│   ├── modules/
│   │   ├── auth/            # OAuth (Google, Apple, Facebook)
│   │   ├── listings/        # Property listings
│   │   ├── search/          # Search & filtering
│   │   ├── agencies/        # Agency management
│   │   ├── realtors/        # Realtor profiles
│   │   ├── favorites/       # User favorites
│   │   ├── inquiries/       # Contact inquiries
│   │   ├── imports/         # CRM/XML import
│   │   └── analytics/       # Listing analytics
│   │
│   ├── shared/              # Shared with api-server
│   └── websocket/
│
├── migrations/
├── tests/
└── Cargo.toml
```

### Worker Services

**Purpose:** Background job processing

```
workers/
├── notification-worker/     # Email, SMS, Push
│   ├── src/
│   │   ├── main.rs
│   │   ├── handlers/
│   │   │   ├── email.rs
│   │   │   ├── sms.rs
│   │   │   └── push.rs
│   │   └── templates/
│   └── Cargo.toml
│
├── ai-worker/               # AI/ML processing
│   ├── src/
│   │   ├── main.rs
│   │   ├── handlers/
│   │   │   ├── ocr.rs
│   │   │   ├── categorization.rs
│   │   │   ├── prediction.rs
│   │   │   └── chatbot.rs
│   │   └── models/
│   └── Cargo.toml
│
├── import-worker/           # External data import
│   ├── src/
│   │   ├── main.rs
│   │   ├── handlers/
│   │   │   ├── airbnb.rs
│   │   │   ├── booking.rs
│   │   │   ├── crm.rs
│   │   │   └── xml.rs
│   │   └── transformers/
│   └── Cargo.toml
│
└── scheduler/               # Cron-like scheduler
    ├── src/
    │   ├── main.rs
    │   └── jobs/
    │       ├── payment_reminders.rs
    │       ├── meter_reminders.rs
    │       ├── report_generation.rs
    │       └── maintenance_prediction.rs
    └── Cargo.toml
```

---

## Module Structure

### Module Boundaries (api-server)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              API SERVER MODULES                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐                  │
│  │      AUTH       │  │  ORGANIZATIONS  │  │    BUILDINGS    │                  │
│  │                 │  │                 │  │                 │                  │
│  │ • Login/Logout  │  │ • CRUD Orgs     │  │ • Buildings     │                  │
│  │ • Registration  │  │ • Branding      │  │ • Units         │                  │
│  │ • MFA           │  │ • Settings      │  │ • Entrances     │                  │
│  │ • OAuth/SSO     │  │ • Subscription  │  │ • Owners        │                  │
│  │ • Sessions      │  │ • Billing       │  │ • Tenants       │                  │
│  │ • Roles         │  │                 │  │ • Statistics    │                  │
│  │ • Delegations   │  │                 │  │                 │                  │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘                  │
│           │                    │                    │                            │
│           └────────────────────┼────────────────────┘                            │
│                                │                                                 │
│                    TENANT CONTEXT (Organization)                                 │
│                                │                                                 │
│  ┌─────────────────┐  ┌───────┴─────────┐  ┌─────────────────┐                  │
│  │     FAULTS      │  │     VOTING      │  │    DOCUMENTS    │                  │
│  │                 │  │                 │  │                 │                  │
│  │ • Report        │  │ • Create Vote   │  │ • Upload        │                  │
│  │ • Assign        │  │ • Cast Ballot   │  │ • Folders       │                  │
│  │ • Update Status │  │ • Results       │  │ • Versions      │                  │
│  │ • Comments      │  │ • Delegation    │  │ • Sharing       │                  │
│  │ • History       │  │ • Comments      │  │ • OCR           │                  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘                  │
│                                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐                  │
│  │    MESSAGES     │  │   FINANCIAL     │  │     METERS      │                  │
│  │                 │  │                 │  │                 │                  │
│  │ • Conversations │  │ • Invoices      │  │ • Readings      │                  │
│  │ • Send/Receive  │  │ • Payments      │  │ • Verification  │                  │
│  │ • Attachments   │  │ • Balance       │  │ • History       │                  │
│  │ • Read Receipts │  │ • Reports       │  │ • Reminders     │                  │
│  │ • Groups        │  │ • Budget        │  │ • OCR           │                  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘                  │
│                                                                                  │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐                  │
│  │    RENTALS      │  │ NOTIFICATIONS   │  │     ADMIN       │                  │
│  │                 │  │                 │  │                 │                  │
│  │ • Reservations  │  │ • Preferences   │  │ • User Roles    │                  │
│  │ • Guests        │  │ • Send Push     │  │ • Audit Log     │                  │
│  │ • Check-in/out  │  │ • Send Email    │  │ • Settings      │                  │
│  │ • Police Reg    │  │ • Send SMS      │  │ • Templates     │                  │
│  │ • Airbnb Sync   │  │ • History       │  │ • Statistics    │                  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘                  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Module Communication Rules

1. **Intra-Module**: Direct function calls within same module
2. **Inter-Module**: Via public service interfaces, not direct DB access
3. **Cross-Server**: Via message queue or shared database views
4. **External**: Via anti-corruption layer (adapters)

```rust
// Example: Module interface
// modules/faults/service.rs

pub struct FaultService {
    repo: FaultRepository,
    notification_service: Arc<dyn NotificationPort>,
    storage_service: Arc<dyn StoragePort>,
}

impl FaultService {
    pub async fn report_fault(&self, input: ReportFaultInput) -> Result<Fault> {
        // Validate input
        // Store photos via storage port
        // Create fault in database
        // Emit event for notifications
    }
}

// Ports for external dependencies
pub trait NotificationPort: Send + Sync {
    async fn send(&self, notification: Notification) -> Result<()>;
}

pub trait StoragePort: Send + Sync {
    async fn upload(&self, file: File) -> Result<FileUrl>;
}
```

---

## API Contracts

### OpenAPI Specification Structure

```
docs/api/
├── typespec/
│   ├── main.tsp              # Entry point
│   ├── tspconfig.yaml        # TypeSpec config
│   │
│   ├── domains/
│   │   ├── auth.tsp          # Authentication
│   │   ├── organizations.tsp # Multi-tenancy
│   │   ├── buildings.tsp     # Buildings & Units
│   │   ├── faults.tsp        # Fault reporting
│   │   ├── voting.tsp        # Voting
│   │   ├── documents.tsp     # Documents
│   │   ├── messages.tsp      # Messaging
│   │   ├── financial.tsp     # Financial
│   │   ├── rentals.tsp       # Short-term rentals
│   │   ├── meters.tsp        # Meter readings
│   │   ├── listings.tsp      # Reality listings
│   │   └── agencies.tsp      # Reality agencies
│   │
│   └── shared/
│       ├── models.tsp        # Common types
│       ├── errors.tsp        # Error responses
│       ├── pagination.tsp    # Pagination
│       └── tenant.tsp        # Multi-tenant context
│
└── generated/
    ├── openapi.yaml          # Full OpenAPI spec
    └── by-service/
        ├── api-server.yaml   # PPT API spec
        └── reality-server.yaml # Reality API spec
```

### API Versioning Strategy

```
/api/v1/...   # Current stable version
/api/v2/...   # Next version (when needed)
```

**Rules:**
- Maximum 2 active versions
- 12-month deprecation window
- Breaking changes only in new versions
- Additive changes allowed in current version

### Request/Response Standards

```yaml
# Standard successful response
{
  "data": { ... },
  "meta": {
    "requestId": "uuid",
    "timestamp": "ISO-8601"
  }
}

# Paginated response
{
  "data": [...],
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 100,
    "hasMore": true
  }
}

# Error response
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Human readable message",
    "details": [...],
    "requestId": "uuid"
  }
}
```

### Authentication Headers

```http
Authorization: Bearer <jwt_token>
X-Tenant-Id: <organization_uuid>
X-Request-Id: <uuid>
Accept-Language: sk-SK
```

### Rate Limiting Headers

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1699999999
```

---

## Database Architecture

### Schema Design

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              DATABASE SCHEMA                                     │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  PLATFORM SCHEMA (platform.*)                                                    │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  organizations, subscriptions, subscription_plans, audit_logs           │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│  IDENTITY SCHEMA (identity.*)                                                    │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  users, roles, user_roles, sessions, delegations, social_accounts       │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│  PROPERTY SCHEMA (property.*) - Uses RLS by organization_id                      │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  buildings, entrances, units, unit_ownerships, unit_occupancies         │    │
│  │  faults, fault_communications, fault_status_history                     │    │
│  │  announcements, announcement_comments                                    │    │
│  │  votes, vote_options, ballots, vote_comments                            │    │
│  │  conversations, messages, message_attachments                            │    │
│  │  documents, document_folders, document_versions                          │    │
│  │  meters, meter_readings, person_months                                   │    │
│  │  financial_accounts, invoices, invoice_items, payments, transactions    │    │
│  │  reservations, guests, police_registrations                             │    │
│  │  notifications, notification_preferences, device_tokens                  │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│  REALITY SCHEMA (reality.*)                                                      │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  listings, listing_photos, listing_features                             │    │
│  │  agencies, agency_realtors                                              │    │
│  │  realtors, realtor_licenses                                             │    │
│  │  portal_users, favorites, saved_searches                                │    │
│  │  inquiries, inquiry_responses, viewings                                 │    │
│  │  property_imports, import_runs                                          │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Row-Level Security (RLS)

```sql
-- Enable RLS on property tables
ALTER TABLE property.buildings ENABLE ROW LEVEL SECURITY;

-- Policy for organization isolation
CREATE POLICY org_isolation ON property.buildings
    USING (organization_id = current_setting('app.current_org')::uuid);

-- Policy for super admin
CREATE POLICY super_admin ON property.buildings
    USING (current_setting('app.is_super_admin')::boolean = true);

-- Set tenant context in application
SET app.current_org = 'organization-uuid';
SET app.is_super_admin = 'false';
```

### Key Tables

```sql
-- Organizations (multi-tenancy root)
CREATE TABLE platform.organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL, -- 'housing_cooperative' | 'property_management'
    contact_email VARCHAR(255) NOT NULL,
    contact_phone VARCHAR(50),
    branding JSONB DEFAULT '{}',
    settings JSONB DEFAULT '{}',
    status VARCHAR(50) DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Users
CREATE TABLE identity.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255),
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    phone VARCHAR(50),
    avatar_url TEXT,
    language VARCHAR(10) DEFAULT 'sk',
    mfa_enabled BOOLEAN DEFAULT FALSE,
    mfa_secret VARCHAR(255),
    status VARCHAR(50) DEFAULT 'active',
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Buildings (with RLS)
CREATE TABLE property.buildings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES platform.organizations(id),
    name VARCHAR(255),
    street VARCHAR(255) NOT NULL,
    street_number VARCHAR(50) NOT NULL,
    city VARCHAR(100) NOT NULL,
    postal_code VARCHAR(20) NOT NULL,
    country VARCHAR(2) DEFAULT 'SK',
    coordinates GEOGRAPHY(POINT),
    year_built INTEGER,
    total_units INTEGER DEFAULT 0,
    total_floors INTEGER,
    amenities JSONB DEFAULT '[]',
    status VARCHAR(50) DEFAULT 'active',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Listings (Reality Portal)
CREATE TABLE reality.listings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    realtor_id UUID NOT NULL REFERENCES reality.realtors(id),
    agency_id UUID REFERENCES reality.agencies(id),
    property_type VARCHAR(50) NOT NULL, -- 'apartment' | 'house' | 'commercial' | 'land'
    transaction_type VARCHAR(20) NOT NULL, -- 'sale' | 'rent'
    title VARCHAR(255) NOT NULL,
    description TEXT,
    description_translations JSONB DEFAULT '{}',
    street VARCHAR(255) NOT NULL,
    city VARCHAR(100) NOT NULL,
    district VARCHAR(100),
    postal_code VARCHAR(20),
    country VARCHAR(2) DEFAULT 'SK',
    coordinates GEOGRAPHY(POINT),
    price DECIMAL(15, 2) NOT NULL,
    currency VARCHAR(3) DEFAULT 'EUR',
    size_sqm DECIMAL(10, 2),
    rooms INTEGER,
    bathrooms INTEGER,
    floor INTEGER,
    total_floors INTEGER,
    year_built INTEGER,
    features JSONB DEFAULT '[]',
    status VARCHAR(50) DEFAULT 'draft',
    is_featured BOOLEAN DEFAULT FALSE,
    views_count INTEGER DEFAULT 0,
    favorites_count INTEGER DEFAULT 0,
    published_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes
CREATE INDEX idx_buildings_org ON property.buildings(organization_id);
CREATE INDEX idx_buildings_city ON property.buildings(city);
CREATE INDEX idx_listings_search ON reality.listings USING GIN (to_tsvector('simple', title || ' ' || COALESCE(description, '')));
CREATE INDEX idx_listings_location ON reality.listings USING GIST (coordinates);
CREATE INDEX idx_listings_price ON reality.listings(price);
CREATE INDEX idx_listings_status ON reality.listings(status) WHERE status = 'active';
```

### Database Migrations

Using SQLx migrations:

```
migrations/
├── 20240101000000_create_platform_schema.sql
├── 20240101000001_create_identity_schema.sql
├── 20240101000002_create_property_schema.sql
├── 20240101000003_create_reality_schema.sql
├── 20240101000004_enable_rls.sql
├── 20240101000005_create_indexes.sql
└── 20240101000006_seed_roles.sql
```

---

## Infrastructure

### Deployment Architecture

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              KUBERNETES CLUSTER                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌─────────────────────────────────────────────────────────────────────────┐    │
│  │  INGRESS CONTROLLER (nginx-ingress)                                      │    │
│  │  • SSL Termination    • Rate Limiting    • Routing                      │    │
│  └─────────────────────────────────────────────────────────────────────────┘    │
│                                                                                  │
│  ┌───────────────────┐  ┌───────────────────┐  ┌───────────────────────────┐    │
│  │  api-server       │  │  reality-server   │  │  Workers                  │    │
│  │  Deployment       │  │  Deployment       │  │                           │    │
│  │                   │  │                   │  │  notification-worker      │    │
│  │  replicas: 3      │  │  replicas: 3      │  │  ai-worker               │    │
│  │  resources:       │  │  resources:       │  │  import-worker           │    │
│  │    cpu: 500m      │  │    cpu: 500m      │  │  scheduler               │    │
│  │    memory: 512Mi  │  │    memory: 512Mi  │  │                           │    │
│  │                   │  │                   │  │  replicas: 1-3 each       │    │
│  │  HPA: 3-10        │  │  HPA: 3-10        │  │                           │    │
│  └───────────────────┘  └───────────────────┘  └───────────────────────────┘    │
│                                                                                  │
│  ┌───────────────────────────────────────────────────────────────────────────┐  │
│  │  STATEFUL SERVICES (Managed or Self-Hosted)                               │  │
│  │                                                                            │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │  │
│  │  │ PostgreSQL  │  │   Redis     │  │  RabbitMQ   │  │   MinIO     │       │  │
│  │  │ (RDS/self)  │  │ (Elasticache│  │ (Amazon MQ/ │  │ (S3/self)   │       │  │
│  │  │             │  │  /self)     │  │  self)      │  │             │       │  │
│  │  │ Primary +   │  │ Cluster     │  │ 3-node      │  │ HA mode     │       │  │
│  │  │ 2 Replicas  │  │ mode        │  │ cluster     │  │             │       │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘       │  │
│  │                                                                            │  │
│  └───────────────────────────────────────────────────────────────────────────┘  │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Environment Configuration

```yaml
# config/production.yaml
server:
  host: 0.0.0.0
  port: 8080
  workers: 4

database:
  host: ${DATABASE_HOST}
  port: 5432
  name: ppt
  user: ${DATABASE_USER}
  password: ${DATABASE_PASSWORD}
  max_connections: 100
  ssl_mode: require

redis:
  url: ${REDIS_URL}
  pool_size: 20

queue:
  url: ${RABBITMQ_URL}
  prefetch: 10

storage:
  endpoint: ${S3_ENDPOINT}
  bucket: ${S3_BUCKET}
  access_key: ${S3_ACCESS_KEY}
  secret_key: ${S3_SECRET_KEY}
  region: eu-central-1

jwt:
  secret: ${JWT_SECRET}
  access_token_ttl: 15m
  refresh_token_ttl: 7d

external:
  sendgrid_api_key: ${SENDGRID_API_KEY}
  twilio_account_sid: ${TWILIO_ACCOUNT_SID}
  twilio_auth_token: ${TWILIO_AUTH_TOKEN}
  stripe_secret_key: ${STRIPE_SECRET_KEY}
  openai_api_key: ${OPENAI_API_KEY}
```

### CI/CD Pipeline

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: cargo test --all

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Docker images
        run: |
          docker build -t api-server:${{ github.sha }} ./backend/api-server
          docker build -t reality-server:${{ github.sha }} ./backend/reality-server
      - name: Push to registry
        run: |
          docker push $REGISTRY/api-server:${{ github.sha }}
          docker push $REGISTRY/reality-server:${{ github.sha }}

  deploy:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Deploy to Kubernetes
        run: |
          kubectl set image deployment/api-server api-server=$REGISTRY/api-server:${{ github.sha }}
          kubectl set image deployment/reality-server reality-server=$REGISTRY/reality-server:${{ github.sha }}
          kubectl rollout status deployment/api-server
          kubectl rollout status deployment/reality-server
```

---

## Security Architecture

### Authentication Flow

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           AUTHENTICATION ARCHITECTURE                           │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  ┌──────────┐     ┌─────────────┐     ┌──────────────┐     ┌─────────────────┐  │
│  │  Client  │────►│ API Gateway │────►│ Auth Service │────►│    Database     │  │
│  └──────────┘     └─────────────┘     └──────────────┘     └─────────────────┘  │
│       │                  │                    │                                  │
│       │                  │                    │                                  │
│       │                  ▼                    ▼                                  │
│       │          ┌─────────────┐     ┌──────────────┐                           │
│       │          │ Rate Limit  │     │  JWT Tokens  │                           │
│       │          │   (Redis)   │     │   (Redis)    │                           │
│       │          └─────────────┘     └──────────────┘                           │
│       │                                                                          │
│       ▼                                                                          │
│  ┌──────────────────────────────────────────────────────────────────────────┐   │
│  │                          TOKEN STRUCTURE                                  │   │
│  │                                                                           │   │
│  │  Access Token (15 min):                                                   │   │
│  │  {                                                                        │   │
│  │    "sub": "user-uuid",                                                    │   │
│  │    "org": "organization-uuid",                                            │   │
│  │    "roles": ["owner", "manager"],                                         │   │
│  │    "permissions": ["faults:read", "faults:write"],                        │   │
│  │    "exp": 1699999999                                                      │   │
│  │  }                                                                        │   │
│  │                                                                           │   │
│  │  Refresh Token (7 days):                                                  │   │
│  │  {                                                                        │   │
│  │    "sub": "user-uuid",                                                    │   │
│  │    "jti": "token-id",                                                     │   │
│  │    "exp": 1700999999                                                      │   │
│  │  }                                                                        │   │
│  │                                                                           │   │
│  └──────────────────────────────────────────────────────────────────────────┘   │
│                                                                                  │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### Authorization Model

```rust
// Permission-based authorization
enum Permission {
    // Faults
    FaultsRead,
    FaultsWrite,
    FaultsAssign,
    FaultsDelete,

    // Voting
    VotesRead,
    VotesWrite,
    VotesCast,

    // Documents
    DocumentsRead,
    DocumentsWrite,
    DocumentsDelete,

    // Admin
    UsersManage,
    RolesManage,
    OrganizationManage,
}

// Role definitions
struct Role {
    name: String,
    permissions: HashSet<Permission>,
}

// Authorization check
async fn authorize(user: &User, permission: Permission) -> Result<()> {
    if user.has_permission(permission) {
        Ok(())
    } else {
        Err(AuthError::Forbidden)
    }
}
```

### Security Measures

| Layer | Measure | Implementation |
|-------|---------|----------------|
| Transport | TLS 1.3 | Nginx/ALB |
| API | Rate Limiting | Redis + middleware |
| API | Input Validation | Validator crate |
| Auth | Password Hashing | Argon2id |
| Auth | JWT Signing | EdDSA (Ed25519) |
| Auth | MFA | TOTP (RFC 6238) |
| Data | Encryption at Rest | PostgreSQL TDE / S3 SSE |
| Data | Row-Level Security | PostgreSQL RLS |
| Logs | PII Masking | Custom logger |
| Secrets | Management | Vault / AWS Secrets Manager |

---

## Observability

### Logging

```rust
// Structured logging with tracing
use tracing::{info, error, instrument};

#[instrument(skip(db))]
async fn create_fault(
    db: &Database,
    input: CreateFaultInput,
) -> Result<Fault> {
    info!(
        building_id = %input.building_id,
        category = %input.category,
        "Creating fault"
    );

    let fault = db.insert_fault(input).await?;

    info!(
        fault_id = %fault.id,
        "Fault created"
    );

    Ok(fault)
}
```

### Metrics

```
# Prometheus metrics

# HTTP metrics
http_requests_total{method, path, status}
http_request_duration_seconds{method, path}

# Business metrics
faults_created_total{organization, category}
votes_cast_total{organization}
payments_processed_total{organization, status}

# Infrastructure metrics
db_connections_active
db_query_duration_seconds{query}
cache_hits_total
cache_misses_total
queue_messages_pending{queue}
```

### Tracing

```
# OpenTelemetry configuration
OTEL_SERVICE_NAME=api-server
OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
OTEL_TRACES_SAMPLER=parentbased_traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1
```

### Health Checks

```rust
// Health check endpoints
GET /health         // Liveness probe
GET /health/ready   // Readiness probe

// Readiness checks
async fn readiness_check() -> impl IntoResponse {
    let db_ok = check_database().await;
    let redis_ok = check_redis().await;
    let queue_ok = check_queue().await;

    if db_ok && redis_ok && queue_ok {
        (StatusCode::OK, "ready")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "not ready")
    }
}
```

### Alerting

| Alert | Condition | Severity |
|-------|-----------|----------|
| High Error Rate | 5xx > 1% for 5min | Critical |
| High Latency | p99 > 2s for 5min | Warning |
| Database Down | health check fails | Critical |
| Queue Backlog | pending > 10000 | Warning |
| Disk Space | usage > 80% | Warning |
| Certificate Expiry | < 7 days | Critical |

---

## Summary

### Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Backend Language | Rust | Performance, safety |
| Web Framework | Axum | Modern, async |
| Database | PostgreSQL 16 | Features, reliability |
| Cache | Redis 7 | Speed, data structures |
| Queue | RabbitMQ | Reliability, routing |
| Storage | S3/MinIO | Scalability, cost |
| API Spec | TypeSpec → OpenAPI | DRY, SDK generation |
| Container | Docker | Portability |
| Orchestration | Kubernetes | Scaling, reliability |
| CI/CD | GitHub Actions | Integration |
| Observability | Prometheus, Jaeger | Metrics, tracing |

### Scalability Path

1. **Phase 1 (MVP)**: Single region, managed services
2. **Phase 2**: Read replicas, CDN for static assets
3. **Phase 3**: Multi-region, sharding by organization
4. **Phase 4**: Extract high-load modules (Notifications, AI)

### Cost Optimization

- Use spot instances for workers
- Reserved instances for databases
- S3 lifecycle policies for old documents
- Cache aggressively to reduce DB load
- Compress images before storage
