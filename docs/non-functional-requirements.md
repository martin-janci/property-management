# Non-Functional Requirements

This document defines the non-functional requirements (NFRs) for the Property Management System (PPT) and Reality Portal, covering performance, scalability, security, logging, monitoring, SEO, caching, and web rendering strategies.

> **Delivery-surface baselines:** per-surface NFR baselines (current state vs target) live alongside this doc. See [`integrations/push-voice-webhooks-nfr-baseline.md`](./integrations/push-voice-webhooks-nfr-baseline.md) for push fan-out (FCM/APNs) and voice webhooks — metrics catalogue, retry/dead-letter policy, and token-refresh failure alerting.

## Table of Contents

1. [Performance Requirements](#performance-requirements)
2. [Scalability Requirements](#scalability-requirements)
3. [Security Requirements](#security-requirements)
4. [Logging Requirements](#logging-requirements)
5. [Monitoring & Observability](#monitoring--observability)
6. [SEO Requirements](#seo-requirements)
7. [Caching Strategy](#caching-strategy)
8. [Web Rendering Strategy (SSR/SSG)](#web-rendering-strategy-ssrssg)
9. [Reliability & Availability](#reliability--availability)
10. [Compliance Requirements](#compliance-requirements)

---

## Performance Requirements

### Response Time SLAs

| Endpoint Category | P50 | P95 | P99 | Max |
|-------------------|-----|-----|-----|-----|
| Health checks | < 10ms | < 20ms | < 50ms | 100ms |
| Authentication | < 100ms | < 200ms | < 500ms | 1s |
| Simple reads (GET single resource) | < 50ms | < 100ms | < 200ms | 500ms |
| List queries (paginated) | < 100ms | < 200ms | < 500ms | 1s |
| Complex queries (search, filters) | < 200ms | < 500ms | < 1s | 2s |
| Write operations (POST/PUT/DELETE) | < 100ms | < 200ms | < 500ms | 1s |
| File uploads (< 5MB) | < 500ms | < 1s | < 2s | 5s |
| File uploads (5-50MB) | < 2s | < 5s | < 10s | 30s |
| Report generation | < 2s | < 5s | < 10s | 30s |
| AI/ML operations | < 3s | < 5s | < 10s | 30s |

### Throughput Requirements

| Metric | Target | Burst |
|--------|--------|-------|
| API requests/second (api-server) | 1,000 | 5,000 |
| API requests/second (reality-server) | 2,000 | 10,000 |
| WebSocket connections | 10,000 | 20,000 |
| Concurrent users | 5,000 | 15,000 |
| Database connections (pool) | 100 | 200 |
| Background jobs/minute | 1,000 | 5,000 |

### Frontend Performance

| Metric | Target | Mobile |
|--------|--------|--------|
| First Contentful Paint (FCP) | < 1.5s | < 2s |
| Largest Contentful Paint (LCP) | < 2.5s | < 3s |
| First Input Delay (FID) | < 100ms | < 100ms |
| Cumulative Layout Shift (CLS) | < 0.1 | < 0.1 |
| Time to Interactive (TTI) | < 3s | < 4s |
| Total Blocking Time (TBT) | < 300ms | < 400ms |
| Bundle size (gzipped) | < 200KB | < 150KB |
| Image load time (above fold) | < 1s | < 1.5s |

### Database Performance

| Metric | Target |
|--------|--------|
| Query execution (simple) | < 10ms |
| Query execution (complex) | < 100ms |
| Index scan ratio | > 99% |
| Connection pool utilization | < 80% |
| Transaction duration (avg) | < 50ms |
| Deadlock rate | < 0.01% |
| Slow query threshold | > 100ms |
| Query plan cache hit rate | > 90% |

### Resource Limits

```yaml
# Backend Services
api-server:
  cpu:
    request: 500m
    limit: 2000m
  memory:
    request: 512Mi
    limit: 2Gi
  max_request_body: 50MB
  request_timeout: 30s
  graceful_shutdown: 30s

reality-server:
  cpu:
    request: 500m
    limit: 2000m
  memory:
    request: 512Mi
    limit: 2Gi
  max_request_body: 50MB
  request_timeout: 30s

workers:
  cpu:
    request: 250m
    limit: 1000m
  memory:
    request: 256Mi
    limit: 1Gi
```

---

## Scalability Requirements

### Horizontal Scaling

| Component | Min Replicas | Max Replicas | Scale Trigger |
|-----------|--------------|--------------|---------------|
| api-server | 3 | 10 | CPU > 70% or RPS > 500/pod |
| reality-server | 3 | 15 | CPU > 70% or RPS > 700/pod |
| notification-worker | 2 | 5 | Queue depth > 1000 |
| ai-worker | 1 | 3 | Queue depth > 100 |
| import-worker | 1 | 3 | Queue depth > 50 |

### Vertical Scaling Thresholds

```yaml
# When to consider vertical scaling
database:
  connections: > 80% pool utilization
  cpu: > 70% sustained
  memory: > 80%
  storage: > 70%
  action: Upgrade instance size or add read replicas

redis:
  memory: > 80%
  connections: > 80%
  action: Upgrade instance or enable clustering

message_queue:
  message_backlog: > 10,000
  consumer_lag: > 5 minutes
  action: Add more consumers or upgrade
```

### Data Volume Scaling

| Entity | Year 1 | Year 3 | Year 5 |
|--------|--------|--------|--------|
| Organizations | 100 | 500 | 2,000 |
| Buildings | 5,000 | 25,000 | 100,000 |
| Units | 50,000 | 250,000 | 1,000,000 |
| Users | 100,000 | 500,000 | 2,000,000 |
| Faults | 500,000 | 2,500,000 | 10,000,000 |
| Documents | 1,000,000 | 5,000,000 | 20,000,000 |
| Messages | 5,000,000 | 25,000,000 | 100,000,000 |
| Listings | 50,000 | 200,000 | 500,000 |
| Storage (S3) | 500GB | 2TB | 10TB |

### Database Scaling Strategy

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    DATABASE SCALING PROGRESSION                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Phase 1: Single Primary                                                 │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  PostgreSQL Primary (db.r6g.xlarge)                               │   │
│  │  • 100 connections                                                │   │
│  │  • 500GB storage                                                  │   │
│  │  • Point-in-time recovery                                         │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Phase 2: Primary + Read Replicas (at 50k users)                         │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Primary ──► Read Replica 1 (api-server reads)                    │   │
│  │         └──► Read Replica 2 (reality-server, reporting)           │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  Phase 3: Sharding by Organization (at 500k users)                       │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │  Router ──► Shard 1 (orgs A-M)                                    │   │
│  │        └──► Shard 2 (orgs N-Z)                                    │   │
│  │        └──► Shard 3 (reality portal)                              │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Multi-Region Strategy

```yaml
# Phase 1: Single Region (eu-central-1)
primary_region: eu-central-1
dr_region: eu-west-1  # Disaster recovery only

# Phase 2: Active-Passive (at enterprise customers)
regions:
  eu-central-1:
    role: primary
    services: [api-server, reality-server, workers]
    database: primary
  eu-west-1:
    role: standby
    services: [api-server, reality-server]
    database: read-replica
  failover_time: < 15 minutes

# Phase 3: Active-Active (at global scale)
regions:
  eu-central-1:
    role: active
    geo_routing: Europe
  us-east-1:
    role: active
    geo_routing: Americas
  ap-southeast-1:
    role: active
    geo_routing: Asia-Pacific
  database: CockroachDB or Spanner (global consistency)
```

---

## Security Requirements

### Authentication & Authorization

| Requirement | Implementation |
|-------------|----------------|
| Password hashing | Argon2id (memory: 64MB, iterations: 3, parallelism: 4) |
| Password policy | Min 12 chars, mixed case, numbers, symbols |
| Session tokens | JWT RS256, 15min access, 7d refresh |
| MFA support | TOTP (RFC 6238), WebAuthn/FIDO2, SMS backup |
| OAuth providers | Google, Apple, Facebook, Microsoft |
| Rate limiting (auth) | 5 attempts/15min per IP, 20/hour per user |
| Account lockout | 5 failed attempts → 15min lock, progressive backoff |
| Session management | Max 5 concurrent sessions, forced logout on password change |

### Data Protection

| Data Type | At Rest | In Transit | Retention |
|-----------|---------|------------|-----------|
| Passwords | Argon2id hash | N/A | Never stored |
| PII (names, emails) | AES-256 | TLS 1.3 | Account lifetime |
| Documents | AES-256 (S3 SSE) | TLS 1.3 | Per organization policy |
| Financial data | AES-256 | TLS 1.3 | 7 years (legal) |
| Session tokens | HMAC-SHA256 | TLS 1.3 | 7 days |
| Audit logs | Plain (integrity hash) | TLS 1.3 | 2 years |
| Backups | AES-256 | TLS 1.3 | 30 days |

### API Security

```yaml
# Security Headers (all responses)
headers:
  Strict-Transport-Security: "max-age=31536000; includeSubDomains; preload"
  X-Content-Type-Options: "nosniff"
  X-Frame-Options: "DENY"
  X-XSS-Protection: "1; mode=block"
  Content-Security-Policy: "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' https://api.*.com"
  Referrer-Policy: "strict-origin-when-cross-origin"
  Permissions-Policy: "geolocation=(), microphone=(), camera=()"

# CORS Configuration
cors:
  allowed_origins:
    - https://app.ppt.sk
    - https://reality.ppt.sk
    - https://*.ppt.sk  # Subdomains
  allowed_methods: [GET, POST, PUT, PATCH, DELETE, OPTIONS]
  allowed_headers: [Authorization, Content-Type, X-Request-ID, X-Org-ID]
  expose_headers: [X-Request-ID, X-RateLimit-*]
  max_age: 86400
  credentials: true

# Rate Limiting
rate_limits:
  global:
    requests: 1000/minute
    burst: 100
  per_user:
    requests: 100/minute
    burst: 20
  per_ip:
    requests: 500/minute
    burst: 50
  endpoints:
    /api/v1/auth/login:
      requests: 5/minute
      burst: 5
    /api/v1/auth/register:
      requests: 3/minute
      burst: 3
    /api/v1/files/upload:
      requests: 10/minute
      burst: 5
```

### Input Validation

| Input Type | Validation | Sanitization |
|------------|------------|--------------|
| Email | RFC 5322 regex | Lowercase, trim |
| Phone | E.164 format | Normalize to E.164 |
| UUID | UUID v4 format | Reject if invalid |
| Pagination | limit: 1-100, offset: >= 0 | Clamp to bounds |
| Text fields | Max length per field | HTML escape, trim |
| HTML content | Allowlist tags (announcements) | DOMPurify |
| File uploads | MIME type whitelist | Virus scan |
| URLs | Valid URL format | Protocol whitelist (https) |
| SQL/NoSQL | Parameterized queries only | N/A |
| JSON | Schema validation | Reject extra fields |

### Vulnerability Prevention

| Threat | Mitigation |
|--------|------------|
| SQL Injection | Parameterized queries (sqlx), ORM |
| XSS | Content-Security-Policy, output encoding |
| CSRF | SameSite cookies, CSRF tokens |
| IDOR | Authorization checks on all resources |
| Path Traversal | Filename sanitization, chroot storage |
| SSRF | Allowlist external URLs, no private IPs |
| XXE | Disable XML external entities |
| Clickjacking | X-Frame-Options: DENY |
| Open Redirect | Allowlist redirect URLs |
| Mass Assignment | Explicit field allowlists in DTOs |

### Secret Management

```yaml
# Secret Storage
production:
  provider: AWS Secrets Manager / HashiCorp Vault
  rotation:
    database_passwords: 90 days
    api_keys: 180 days
    jwt_secret: 365 days (with overlap)
  access:
    - services via IAM roles
    - no hardcoded secrets
    - no secrets in environment variables directly

# Secret Categories
secrets:
  critical:  # Vault, manual rotation
    - database_credentials
    - encryption_keys
    - jwt_signing_key
  high:  # Secrets Manager, auto-rotation
    - external_api_keys
    - oauth_secrets
  medium:  # Config maps, encrypted
    - feature_flags
    - non-sensitive config
```

---

## Logging Requirements

### Log Levels

| Level | Use Case | Examples |
|-------|----------|----------|
| ERROR | Failures requiring attention | Database errors, external API failures, unhandled exceptions |
| WARN | Potential issues | Rate limiting triggered, deprecated API usage, retry attempts |
| INFO | Business events | User login, resource created, payment processed |
| DEBUG | Development details | Query timing, cache hits/misses, request details |
| TRACE | Very verbose | Full request/response bodies (development only) |

### Structured Log Format

```json
{
  "timestamp": "2024-01-15T10:30:00.123Z",
  "level": "INFO",
  "service": "api-server",
  "version": "1.2.3",
  "environment": "production",
  "trace_id": "abc123def456",
  "span_id": "789xyz",
  "request_id": "req-uuid-here",
  "user_id": "user-uuid-here",
  "organization_id": "org-uuid-here",
  "message": "User logged in successfully",
  "event": "auth.login.success",
  "duration_ms": 45,
  "metadata": {
    "ip": "192.168.1.1",
    "user_agent": "Mozilla/5.0...",
    "method": "POST",
    "path": "/api/v1/auth/login"
  }
}
```

### Log Categories

| Category | Retention | Storage | PII |
|----------|-----------|---------|-----|
| Access logs | 30 days | CloudWatch/Loki | IP, User-Agent |
| Application logs | 90 days | CloudWatch/Loki | Masked |
| Error logs | 180 days | CloudWatch/Loki | Masked |
| Audit logs | 2 years | S3 (archived) | Minimal |
| Security logs | 2 years | S3 (archived) | IP, User ID |
| Debug logs | 7 days | Local/ELK | Full (dev only) |

### Audit Log Events

```yaml
# Must-log events (audit trail)
audit_events:
  authentication:
    - user.login.success
    - user.login.failed
    - user.logout
    - user.mfa.enabled
    - user.mfa.disabled
    - user.password.changed
    - user.password.reset

  authorization:
    - role.assigned
    - role.revoked
    - permission.granted
    - permission.denied
    - delegation.created
    - delegation.revoked

  data_access:
    - document.downloaded
    - document.shared
    - report.generated
    - export.requested
    - gdpr.data_exported
    - gdpr.data_deleted

  financial:
    - payment.initiated
    - payment.completed
    - payment.failed
    - invoice.created
    - invoice.sent

  admin:
    - organization.created
    - organization.settings_changed
    - building.created
    - building.deleted
    - user.created
    - user.deleted
    - user.suspended
```

### Log Aggregation

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         LOG AGGREGATION PIPELINE                         │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────────────────┐   │
│  │ api-server   │───►│              │    │                          │   │
│  └──────────────┘    │              │    │   Loki / CloudWatch      │   │
│  ┌──────────────┐    │   Fluentd/   │───►│   Logs                   │   │
│  │reality-server│───►│   Fluent Bit │    │                          │   │
│  └──────────────┘    │              │    │   ┌─────────────────┐    │   │
│  ┌──────────────┐    │              │    │   │ Grafana         │    │   │
│  │ workers      │───►│              │    │   │ (visualization) │    │   │
│  └──────────────┘    └──────────────┘    │   └─────────────────┘    │   │
│                                          │                          │   │
│                                          │   ┌─────────────────┐    │   │
│                                          │   │ Alerts          │    │   │
│                                          │   │ (PagerDuty/Slack│    │   │
│                                          │   └─────────────────┘    │   │
│                                          └──────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Monitoring & Observability

### Three Pillars

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      OBSERVABILITY STACK                                 │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐          │
│  │    METRICS      │  │     LOGS        │  │    TRACES       │          │
│  │                 │  │                 │  │                 │          │
│  │  Prometheus     │  │  Loki /         │  │  Jaeger /       │          │
│  │  + Grafana      │  │  CloudWatch     │  │  Tempo          │          │
│  │                 │  │                 │  │                 │          │
│  │  • Request rate │  │  • App logs     │  │  • Distributed  │          │
│  │  • Error rate   │  │  • Access logs  │  │    tracing      │          │
│  │  • Latency      │  │  • Audit logs   │  │  • Span data    │          │
│  │  • Resource use │  │  • Security     │  │  • Dependencies │          │
│  │                 │  │                 │  │                 │          │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘          │
│           │                    │                    │                    │
│           └────────────────────┼────────────────────┘                    │
│                                │                                         │
│                                ▼                                         │
│                     ┌─────────────────────┐                              │
│                     │     GRAFANA         │                              │
│                     │   (Unified View)    │                              │
│                     │                     │                              │
│                     │  Dashboards         │                              │
│                     │  Alerts             │                              │
│                     │  Correlations       │                              │
│                     └─────────────────────┘                              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Key Metrics (RED Method)

| Metric | Description | Target | Alert Threshold |
|--------|-------------|--------|-----------------|
| **R**ate | Requests per second | 1000 RPS | > 5000 RPS (capacity) |
| **E**rrors | Error rate (5xx) | < 0.1% | > 1% (5min avg) |
| **D**uration | P95 latency | < 200ms | > 500ms (5min avg) |

### Service Metrics

```yaml
# Application Metrics (Prometheus format)
http_requests_total:
  labels: [method, path, status, service]
  type: counter

http_request_duration_seconds:
  labels: [method, path, status, service]
  type: histogram
  buckets: [0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10]

http_requests_in_flight:
  labels: [service]
  type: gauge

# Database Metrics
db_query_duration_seconds:
  labels: [query_type, table]
  type: histogram

db_connections_active:
  labels: [pool]
  type: gauge

db_connections_idle:
  labels: [pool]
  type: gauge

# Cache Metrics
cache_hits_total:
  labels: [cache_name, key_pattern]
  type: counter

cache_misses_total:
  labels: [cache_name, key_pattern]
  type: counter

cache_evictions_total:
  labels: [cache_name, reason]
  type: counter

# Queue Metrics
queue_messages_published_total:
  labels: [queue_name]
  type: counter

queue_messages_consumed_total:
  labels: [queue_name, status]
  type: counter

queue_depth:
  labels: [queue_name]
  type: gauge

queue_consumer_lag_seconds:
  labels: [queue_name, consumer]
  type: gauge

# Business Metrics
users_active:
  labels: [organization_type]
  type: gauge

faults_created_total:
  labels: [category, priority]
  type: counter

votes_cast_total:
  labels: [vote_type]
  type: counter

payments_processed_total:
  labels: [status, payment_method]
  type: counter
```

### Health Checks

```yaml
# Kubernetes probes
healthChecks:
  liveness:
    path: /health/live
    interval: 10s
    timeout: 5s
    failure_threshold: 3
    checks:
      - process_running

  readiness:
    path: /health/ready
    interval: 5s
    timeout: 3s
    failure_threshold: 3
    checks:
      - database_connected
      - redis_connected
      - queue_connected

  startup:
    path: /health/startup
    interval: 5s
    timeout: 10s
    failure_threshold: 30
    checks:
      - migrations_complete
      - cache_warmed
```

### Alerting Rules

```yaml
# Critical Alerts (PagerDuty)
critical:
  - name: HighErrorRate
    condition: error_rate > 5% for 5m
    severity: critical

  - name: ServiceDown
    condition: up == 0 for 2m
    severity: critical

  - name: DatabaseConnectionsFull
    condition: db_connections_active / db_connections_max > 0.95 for 5m
    severity: critical

  - name: DiskSpaceCritical
    condition: disk_usage > 90%
    severity: critical

# Warning Alerts (Slack)
warning:
  - name: HighLatency
    condition: p95_latency > 500ms for 10m
    severity: warning

  - name: QueueBacklog
    condition: queue_depth > 10000 for 15m
    severity: warning

  - name: CacheMissRateHigh
    condition: cache_miss_rate > 20% for 30m
    severity: warning

  - name: MemoryHigh
    condition: memory_usage > 80% for 10m
    severity: warning

# Info Alerts (Slack)
info:
  - name: DeploymentCompleted
    condition: deployment_status == "completed"
    severity: info

  - name: HighTraffic
    condition: request_rate > 2x_baseline for 30m
    severity: info
```

### Dashboards

| Dashboard | Purpose | Key Panels |
|-----------|---------|------------|
| Service Overview | High-level health | RPS, error rate, latency, uptime |
| API Performance | Endpoint analysis | Latency by endpoint, slow queries, errors |
| Database | PostgreSQL health | Connections, query time, locks, replication lag |
| Cache | Redis performance | Hit rate, memory, evictions, keys |
| Queue | Message processing | Depth, consumer lag, throughput |
| Business | Product metrics | Active users, faults created, payments |
| Security | Security events | Failed logins, rate limits, suspicious activity |
| Infrastructure | Resource usage | CPU, memory, disk, network |

---

## SEO Requirements

### Reality Portal SEO

The Reality Portal (reality-web) requires comprehensive SEO for public listing pages.

#### Technical SEO

| Requirement | Implementation |
|-------------|----------------|
| Server-Side Rendering | Next.js SSR for listing pages |
| Meta tags | Dynamic title, description, Open Graph, Twitter Cards |
| Structured data | JSON-LD for Property, RealEstateAgent, Organization |
| Canonical URLs | `<link rel="canonical">` on all pages |
| Sitemap | Dynamic XML sitemap (`/sitemap.xml`) |
| Robots.txt | Allow crawling, specify sitemap location |
| Mobile-friendly | Responsive design, viewport meta tag |
| Page speed | Core Web Vitals targets (LCP < 2.5s, FID < 100ms, CLS < 0.1) |
| HTTPS | All pages served over HTTPS |
| URL structure | Clean, semantic URLs (`/listings/bratislava/3-izbovy-byt`) |

#### Meta Tags Template

```html
<!-- Listing Page -->
<head>
  <title>{property_type} na {transaction} | {city} | Reality Portal</title>
  <meta name="description" content="{rooms}-izbový byt na {transaction} v {city}, {district}. {size}m², {price}€. {features}.">

  <!-- Open Graph -->
  <meta property="og:type" content="website">
  <meta property="og:title" content="{property_type} na {transaction} | {city}">
  <meta property="og:description" content="{short_description}">
  <meta property="og:image" content="{primary_photo_url}">
  <meta property="og:url" content="{canonical_url}">
  <meta property="og:site_name" content="Reality Portal">
  <meta property="og:locale" content="sk_SK">

  <!-- Twitter Card -->
  <meta name="twitter:card" content="summary_large_image">
  <meta name="twitter:title" content="{property_type} na {transaction} | {city}">
  <meta name="twitter:description" content="{short_description}">
  <meta name="twitter:image" content="{primary_photo_url}">

  <!-- Additional -->
  <link rel="canonical" href="{canonical_url}">
  <meta name="robots" content="index, follow">
  <link rel="alternate" hreflang="sk" href="{sk_url}">
  <link rel="alternate" hreflang="en" href="{en_url}">
</head>
```

#### Structured Data (JSON-LD)

```json
{
  "@context": "https://schema.org",
  "@type": "RealEstateListing",
  "name": "3-izbový byt na predaj v Bratislave",
  "description": "Priestranný 3-izbový byt s balkónom...",
  "url": "https://reality.ppt.sk/listings/bratislava/3-izbovy-byt-123",
  "image": [
    "https://cdn.reality.ppt.sk/listings/123/photo1.jpg",
    "https://cdn.reality.ppt.sk/listings/123/photo2.jpg"
  ],
  "datePosted": "2024-01-15",
  "validThrough": "2024-04-15",
  "offers": {
    "@type": "Offer",
    "priceCurrency": "EUR",
    "price": 185000,
    "availability": "https://schema.org/InStock"
  },
  "address": {
    "@type": "PostalAddress",
    "streetAddress": "Hlavná 123",
    "addressLocality": "Bratislava",
    "addressRegion": "Bratislavský kraj",
    "postalCode": "81101",
    "addressCountry": "SK"
  },
  "geo": {
    "@type": "GeoCoordinates",
    "latitude": 48.1486,
    "longitude": 17.1077
  },
  "floorSize": {
    "@type": "QuantitativeValue",
    "value": 75,
    "unitCode": "MTK"
  },
  "numberOfRooms": 3,
  "numberOfBathroomsTotal": 1,
  "broker": {
    "@type": "RealEstateAgent",
    "name": "Ján Novák",
    "telephone": "+421900123456",
    "image": "https://reality.ppt.sk/agents/jan-novak.jpg",
    "worksFor": {
      "@type": "RealEstateAgent",
      "name": "Reality ABC s.r.o.",
      "url": "https://reality.ppt.sk/agencies/reality-abc"
    }
  }
}
```

#### Sitemap Generation

```xml
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <!-- Static pages -->
  <url>
    <loc>https://reality.ppt.sk/</loc>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
  <url>
    <loc>https://reality.ppt.sk/predaj</loc>
    <changefreq>daily</changefreq>
    <priority>0.9</priority>
  </url>
  <url>
    <loc>https://reality.ppt.sk/prenajom</loc>
    <changefreq>daily</changefreq>
    <priority>0.9</priority>
  </url>

  <!-- Dynamic listing pages (generated) -->
  <url>
    <loc>https://reality.ppt.sk/listings/bratislava/3-izbovy-byt-123</loc>
    <lastmod>2024-01-15</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
    <image:image>
      <image:loc>https://cdn.reality.ppt.sk/listings/123/photo1.jpg</image:loc>
      <image:title>3-izbový byt Bratislava</image:title>
    </image:image>
  </url>

  <!-- City/category pages -->
  <url>
    <loc>https://reality.ppt.sk/predaj/byty/bratislava</loc>
    <changefreq>daily</changefreq>
    <priority>0.8</priority>
  </url>
</urlset>
```

#### URL Structure

| Page Type | URL Pattern | Example |
|-----------|-------------|---------|
| Home | `/` | `reality.ppt.sk/` |
| Sale listings | `/predaj` | `reality.ppt.sk/predaj` |
| Rent listings | `/prenajom` | `reality.ppt.sk/prenajom` |
| Category | `/{transaction}/{property_type}/{city}` | `/predaj/byty/bratislava` |
| Listing detail | `/listings/{city}/{slug}-{id}` | `/listings/bratislava/3-izbovy-byt-123` |
| Agency | `/agencies/{slug}` | `/agencies/reality-abc` |
| Agent | `/agents/{slug}` | `/agents/jan-novak` |

### PPT Web SEO

The Property Management app (ppt-web) is a SPA and doesn't require traditional SEO, but should:

- Implement proper meta tags for shared links (when users share content)
- Use Open Graph for link previews in messaging apps
- Ensure accessibility (WCAG 2.1 AA)

---

## Caching Strategy

### Cache Layers

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          CACHING ARCHITECTURE                            │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  LAYER 1: CDN (Cloudflare/CloudFront)                            │    │
│  │                                                                   │    │
│  │  • Static assets (JS, CSS, images)     TTL: 1 year               │    │
│  │  • Listing photos                       TTL: 1 week              │    │
│  │  • Public API responses (listings)     TTL: 5 minutes            │    │
│  │  • SSR pages (reality-web)             TTL: 5 minutes            │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                │                                         │
│                                ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  LAYER 2: Application Cache (Redis)                              │    │
│  │                                                                   │    │
│  │  • Session data                         TTL: 15 minutes          │    │
│  │  • User permissions                     TTL: 5 minutes           │    │
│  │  • Rate limit counters                  TTL: varies              │    │
│  │  • API response cache                   TTL: 1-60 minutes        │    │
│  │  • Computed aggregations                TTL: 5-15 minutes        │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                │                                         │
│                                ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  LAYER 3: Query Cache (PostgreSQL)                               │    │
│  │                                                                   │    │
│  │  • Prepared statement cache             Size: 100 per connection │    │
│  │  • Query plan cache                     TTL: connection lifetime │    │
│  │  • pg_stat_statements                   For query optimization   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Cache TTL by Resource Type

| Resource | Cache Layer | TTL | Invalidation |
|----------|-------------|-----|--------------|
| Static assets | CDN | 1 year | Hash in filename |
| Listing photos | CDN + S3 | 1 week | Delete on update |
| Listing detail (public) | CDN | 5 min | Stale-while-revalidate |
| Search results | CDN | 1 min | Time-based |
| User session | Redis | 15 min | On logout |
| User permissions | Redis | 5 min | On role change |
| Organization settings | Redis | 15 min | On settings update |
| Building stats | Redis | 15 min | On relevant changes |
| Dashboard aggregations | Redis | 5 min | Periodic refresh |
| JWT tokens | Redis | Token lifetime | On revocation |

### Cache Keys

```yaml
# Key naming convention: {service}:{entity}:{id}:{variant}
cache_keys:
  session: "session:{session_id}"
  user_permissions: "user:{user_id}:permissions:{org_id}"
  org_settings: "org:{org_id}:settings"
  building_stats: "building:{building_id}:stats"
  listing_detail: "listing:{listing_id}:detail:{locale}"
  search_results: "search:{hash_of_params}"
  rate_limit: "ratelimit:{user_id}:{endpoint}"
```

### Cache Invalidation Strategies

| Strategy | Use Case | Implementation |
|----------|----------|----------------|
| Time-based (TTL) | General caching | Set appropriate TTL per resource |
| Event-based | User changes | Publish invalidation event via Redis pub/sub |
| Tag-based | Related data | Group keys by tag, invalidate by tag |
| Write-through | Critical data | Update cache on write |
| Cache-aside | Read-heavy | Load to cache on miss |

### Client-Side Caching

```typescript
// TanStack Query configuration
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 5 * 60 * 1000,      // 5 minutes
      gcTime: 30 * 60 * 1000,         // 30 minutes (garbage collection)
      refetchOnWindowFocus: true,
      refetchOnReconnect: true,
      retry: 3,
      retryDelay: (attemptIndex) => Math.min(1000 * 2 ** attemptIndex, 30000),
    },
  },
});

// Per-query overrides
const useListingDetail = (id: string) => {
  return useQuery({
    queryKey: ['listing', id],
    queryFn: () => api.getListingDetail(id),
    staleTime: 60 * 1000,           // 1 minute (listings change rarely)
    gcTime: 5 * 60 * 1000,          // 5 minutes
  });
};

const useUserNotifications = () => {
  return useQuery({
    queryKey: ['notifications'],
    queryFn: () => api.getNotifications(),
    staleTime: 30 * 1000,           // 30 seconds (near real-time)
    refetchInterval: 60 * 1000,     // Poll every minute
  });
};
```

### Service Worker Caching (Mobile/PWA)

```javascript
// sw.js - Service Worker caching strategies
const CACHE_VERSION = 'v1';

const CACHE_STRATEGIES = {
  // Cache-first for static assets
  static: {
    cacheName: `static-${CACHE_VERSION}`,
    patterns: [/\.js$/, /\.css$/, /\.woff2?$/, /\.png$/, /\.jpg$/],
    strategy: 'cache-first',
    maxAge: 365 * 24 * 60 * 60, // 1 year
  },

  // Network-first for API
  api: {
    cacheName: `api-${CACHE_VERSION}`,
    patterns: [/\/api\//],
    strategy: 'network-first',
    networkTimeout: 3000,
    maxEntries: 100,
    maxAge: 60 * 60, // 1 hour offline fallback
  },

  // Stale-while-revalidate for pages
  pages: {
    cacheName: `pages-${CACHE_VERSION}`,
    patterns: [/\/$/],
    strategy: 'stale-while-revalidate',
    maxAge: 24 * 60 * 60, // 1 day
  },
};
```

---

## Web Rendering Strategy (SSR/SSG)

### Rendering Strategy by App

| App | Strategy | Rationale |
|-----|----------|-----------|
| ppt-web | SPA (CSR) | Authenticated app, no SEO needed |
| reality-web | Hybrid (SSR + SSG + CSR) | SEO for listings, dynamic search |
| mobile | Native (React Native) | N/A |
| mobile-native | Native (KMP) | N/A |

### Reality Portal Rendering

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    REALITY-WEB RENDERING STRATEGY                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Page Type          Rendering    Revalidation    Cache                   │
│  ────────────────────────────────────────────────────────────            │
│                                                                          │
│  Homepage           SSG          60 seconds      CDN (1 min)             │
│  /                  + ISR                                                │
│                                                                          │
│  Category pages     SSG          60 seconds      CDN (1 min)             │
│  /predaj/byty       + ISR                                                │
│                                                                          │
│  Search results     SSR          N/A             CDN (1 min)             │
│  /search?q=...                   (dynamic)       + stale-while-revalidate│
│                                                                          │
│  Listing detail     SSR          N/A             CDN (5 min)             │
│  /listings/...      or SSG+ISR   5 minutes       + on-demand revalidation│
│                                                                          │
│  Agency pages       SSG          60 seconds      CDN (1 hour)            │
│  /agencies/...      + ISR                                                │
│                                                                          │
│  Agent pages        SSG          60 seconds      CDN (1 hour)            │
│  /agents/...        + ISR                                                │
│                                                                          │
│  User dashboard     CSR          N/A             None (authenticated)    │
│  /dashboard/*                                                            │
│                                                                          │
│  Contact/Inquiry    CSR          N/A             None                    │
│  /contact/*                                                              │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Next.js Implementation

```typescript
// pages/listings/[...slug].tsx
export async function getStaticPaths() {
  // Pre-generate top 1000 most viewed listings
  const listings = await api.getTopListings({ limit: 1000 });

  return {
    paths: listings.map(listing => ({
      params: { slug: [listing.city, listing.slug] },
    })),
    fallback: 'blocking', // SSR for non-pregenerated paths
  };
}

export async function getStaticProps({ params }) {
  const listing = await api.getListingBySlug(params.slug);

  if (!listing) {
    return { notFound: true };
  }

  return {
    props: {
      listing,
      relatedListings: await api.getRelatedListings(listing.id),
    },
    revalidate: 300, // ISR: regenerate every 5 minutes
  };
}

// API route for on-demand revalidation
// pages/api/revalidate.ts
export default async function handler(req, res) {
  const { secret, path } = req.query;

  if (secret !== process.env.REVALIDATION_SECRET) {
    return res.status(401).json({ message: 'Invalid secret' });
  }

  await res.revalidate(path);
  return res.json({ revalidated: true });
}
```

### Edge Rendering (Future)

```yaml
# Next.js Edge Runtime for search
# pages/api/search.ts
export const config = {
  runtime: 'edge',
};

# Benefits
edge_rendering:
  latency: < 50ms globally
  use_cases:
    - Search API
    - Personalization
    - A/B testing
    - Geo-routing
```

### Streaming & Suspense

```typescript
// app/listings/[id]/page.tsx (App Router)
import { Suspense } from 'react';

export default async function ListingPage({ params }) {
  return (
    <div>
      {/* Critical content - SSR immediately */}
      <ListingHeader listingId={params.id} />

      {/* Non-critical - stream when ready */}
      <Suspense fallback={<GallerySkeleton />}>
        <ListingGallery listingId={params.id} />
      </Suspense>

      <Suspense fallback={<MapSkeleton />}>
        <ListingMap listingId={params.id} />
      </Suspense>

      <Suspense fallback={<RelatedSkeleton />}>
        <RelatedListings listingId={params.id} />
      </Suspense>
    </div>
  );
}
```

### PPT Web (SPA)

```typescript
// vite.config.ts
export default defineConfig({
  build: {
    rollupOptions: {
      output: {
        // Code splitting by route
        manualChunks: {
          vendor: ['react', 'react-dom', 'react-router-dom'],
          query: ['@tanstack/react-query'],
          ui: ['@ppt/ui-kit'],
        },
      },
    },
  },
});

// Lazy loading routes
const Dashboard = lazy(() => import('./pages/Dashboard'));
const Buildings = lazy(() => import('./pages/Buildings'));
const Faults = lazy(() => import('./pages/Faults'));
const Voting = lazy(() => import('./pages/Voting'));

// App.tsx
<Suspense fallback={<PageLoader />}>
  <Routes>
    <Route path="/dashboard" element={<Dashboard />} />
    <Route path="/buildings/*" element={<Buildings />} />
    <Route path="/faults/*" element={<Faults />} />
    <Route path="/voting/*" element={<Voting />} />
  </Routes>
</Suspense>
```

---

## Reliability & Availability

### SLA Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Uptime | 99.9% (8.76h downtime/year) | Monthly |
| Planned maintenance | < 4h/month | Off-peak hours |
| RTO (Recovery Time Objective) | < 1 hour | Disaster recovery |
| RPO (Recovery Point Objective) | < 5 minutes | Data loss window |
| MTTR (Mean Time to Repair) | < 30 minutes | Incident response |

### Fault Tolerance

```yaml
# Redundancy at every layer
redundancy:
  application:
    api-server: 3+ replicas across AZs
    reality-server: 3+ replicas across AZs
    workers: 2+ replicas

  database:
    primary: 1 (with failover)
    read_replicas: 2+
    backup: Continuous (PITR)

  cache:
    redis: Cluster mode (3 masters, 3 replicas)

  queue:
    rabbitmq: 3-node cluster with mirrored queues

  storage:
    s3: Cross-region replication

  networking:
    load_balancer: Multi-AZ
    dns: Route53 with health checks
```

### Circuit Breaker Pattern

```rust
// Circuit breaker for external services
struct CircuitBreaker {
    state: State,
    failure_count: u32,
    failure_threshold: u32,       // 5 failures
    success_threshold: u32,       // 3 successes
    timeout: Duration,            // 30 seconds
    half_open_max_calls: u32,     // 3 calls
}

impl CircuitBreaker {
    async fn call<F, T>(&mut self, f: F) -> Result<T, Error>
    where
        F: Future<Output = Result<T, Error>>,
    {
        match self.state {
            State::Open => {
                if self.timeout_elapsed() {
                    self.state = State::HalfOpen;
                } else {
                    return Err(Error::CircuitOpen);
                }
            }
            State::HalfOpen => {
                if self.half_open_calls >= self.half_open_max_calls {
                    return Err(Error::CircuitOpen);
                }
            }
            State::Closed => {}
        }

        match timeout(self.timeout, f).await {
            Ok(Ok(result)) => {
                self.on_success();
                Ok(result)
            }
            _ => {
                self.on_failure();
                Err(Error::ServiceUnavailable)
            }
        }
    }
}
```

### Graceful Degradation

| Service | Degraded Mode |
|---------|---------------|
| AI/ML (chatbot, OCR) | Disable AI features, manual input only |
| Email service | Queue for later delivery |
| SMS gateway | Queue for later, email fallback |
| Payment gateway | Show maintenance message, retry later |
| External portal sync | Queue updates, reconcile later |
| File storage | Temporary local storage, sync when available |
| Search | Fall back to database query |
| Real-time notifications | Fall back to polling |

### Backup Strategy

```yaml
backups:
  database:
    type: Continuous (PITR)
    retention: 30 days
    snapshot_frequency: Daily
    snapshot_retention: 90 days
    cross_region: true
    encryption: AES-256

  file_storage:
    type: Cross-region replication
    versioning: Enabled
    lifecycle:
      - transition_to_ia: 30 days
      - transition_to_glacier: 90 days
      - expiration: 365 days (non-critical)

  redis:
    type: RDB + AOF
    snapshot_frequency: Hourly
    retention: 7 days

  configuration:
    type: Git + encrypted secrets
    backup: To separate repository
```

---

## Compliance Requirements

### GDPR Compliance

| Requirement | Implementation |
|-------------|----------------|
| Data minimization | Only collect necessary data |
| Purpose limitation | Clear privacy policy per data type |
| Right to access | `/api/v1/gdpr/export` endpoint |
| Right to erasure | `/api/v1/gdpr/delete` endpoint |
| Right to portability | JSON/CSV export of personal data |
| Data retention | Configurable per data type |
| Consent management | Granular consent tracking |
| Privacy by design | Encryption, pseudonymization |
| DPO contact | Configurable per organization |
| Breach notification | Audit log + alerting |

### Data Retention

| Data Type | Retention Period | Legal Basis |
|-----------|------------------|-------------|
| User accounts | Account lifetime + 30 days | Contract |
| Financial records | 10 years | Slovak law |
| Audit logs | 2 years | Legitimate interest |
| Messages | 2 years or user deletion | Contract |
| Documents | Organization policy | Contract |
| Session logs | 90 days | Security |
| Deleted user data | 30 days (soft delete) | Erasure compliance |

### Accessibility (WCAG 2.1)

| Level | Requirement |
|-------|-------------|
| A | Text alternatives, keyboard accessible, no seizure-inducing content |
| AA | Color contrast 4.5:1, resizable text, focus visible, consistent navigation |
| AAA | Sign language, extended audio description (optional) |

Target: **WCAG 2.1 Level AA** for all web applications.

### Security Certifications (Future)

| Certification | Timeline | Scope |
|---------------|----------|-------|
| SOC 2 Type I | Year 2 | Security, availability |
| SOC 2 Type II | Year 3 | Full audit |
| ISO 27001 | Year 3 | Information security |
| PCI DSS | If needed | Payment processing |

---

## Summary

| Category | Key Metrics |
|----------|-------------|
| **Performance** | P95 < 200ms, 1000 RPS, LCP < 2.5s |
| **Scalability** | 10x capacity, horizontal scaling, multi-region ready |
| **Security** | Zero-trust, encryption everywhere, OWASP Top 10 mitigated |
| **Logging** | Structured, 90-day retention, audit trail |
| **Monitoring** | RED metrics, < 5min alert response, 99.9% uptime |
| **SEO** | SSR for listings, JSON-LD, Core Web Vitals |
| **Caching** | 3-layer caching, < 1s stale tolerance |
| **Rendering** | SSG + ISR for public, SPA for authenticated |
| **Reliability** | 99.9% uptime, < 1h RTO, < 5min RPO |
| **Compliance** | GDPR compliant, WCAG 2.1 AA |
