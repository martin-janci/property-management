//! Host-resolution (tenant-resolution) middleware — Phase 1 keystone.
//!
//! This middleware runs FIRST on the request pipeline. It inspects the inbound
//! `Host` header (or HTTP/2 `:authority`), maps it to exactly one organization
//! (the RLS tenant) via the `agency_domains` table, and injects a
//! [`ResolvedTenant`] into the request extensions. Downstream extractors (e.g.
//! `HostRlsConnection`) read that extension to open an RLS-scoped connection.
//!
//! # Why this is NOT the old `rls_context` middleware
//!
//! The deprecated `middleware/rls_context.rs` set RLS context on the *pool*,
//! which does nothing useful — PostgreSQL session settings are connection-
//! scoped. This middleware deliberately **never touches a DB connection's RLS
//! context**. It only resolves host -> org and injects an extension. The
//! `HostRlsConnection` extractor is what acquires a connection and sets context
//! on that exact connection.
//!
//! # Resolution algorithm (`host_tenant_middleware`)
//!
//! 1. Public-allowlist paths (`/health`, `/readiness`, `/metrics`,
//!    `/swagger-ui`, `/api-docs`) pass through with no resolution.
//! 2. Extract the TRUSTED host: the HTTP `Host` header / HTTP/2 `:authority`.
//!    `X-Forwarded-Host` is ignored unless `TRUST_FORWARDED_HOST=true`.
//!    The host is normalized (lowercase, port stripped, trailing dot stripped)
//!    and rejected if it contains whitespace/control chars or exceeds 253 chars.
//! 3. Dev-mode only: a `/a/{slug}` path prefix resolves the slug to an org and
//!    the request URI is rewritten to strip the prefix (`source = DevPath`).
//! 4. Otherwise: cache lookup, then `AgencyDomainRepository::resolve_host_system`
//!    on miss, caching the (possibly negative) result.
//! 5. Resolved -> inject [`ResolvedTenant`] into extensions, continue.
//! 6. Unresolved -> **fail closed with HTTP 404** `"Unknown tenant host"`.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{StatusCode, Uri},
    middleware::Next,
    response::Response,
};
use db::repositories::AgencyDomainRepository;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::tenant_ops::{meter_request, TenantRateLimiterSet};

/// How a request's tenant was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TenantSource {
    /// Resolved from a platform subdomain (e.g. `acme.app.example.com`).
    Subdomain,
    /// Resolved from a customer-owned custom domain (e.g. `realty.acme.com`).
    CustomDomain,
    /// Resolved from a dev-mode `/a/{slug}` path prefix.
    DevPath,
    /// Resolved as the platform-wide reality portal host (Phase 4).
    ///
    /// This variant means "no specific tenant — global read context": the
    /// request hit the platform's public reality portal (e.g. `reality.example.com`
    /// or any of the seeded-reserved hosts in `reserved_platform_hosts`). The
    /// downstream `HostRlsConnection` extractor turns this into a database
    /// session with `app.global_read = on` and `app.current_org_id` UNSET, so
    /// the 4th RLS context on `listings` (see migration 00136) reveals the
    /// union of `is_published = TRUE` rows across all orgs.
    ///
    /// `organization_id` for this variant is the nil UUID — a sentinel meaning
    /// "no specific tenant." Code that branches on `source == PlatformHost`
    /// MUST NOT use the `organization_id` field as a real tenant id; nothing
    /// in the database has the nil UUID as its organization.
    PlatformHost,
}

/// The tenant a request was resolved to, injected into request extensions.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedTenant {
    /// For `Subdomain`/`CustomDomain`/`DevPath`: the organization id from
    /// `agency_domains`. For `PlatformHost`: the nil UUID — see the docs on
    /// [`TenantSource::PlatformHost`]. Code that consumes this field MUST
    /// branch on `source` first to know whether the id is meaningful.
    pub organization_id: Uuid,
    pub source: TenantSource,
}

impl ResolvedTenant {
    /// True if this resolution is the platform-wide global-read context (Phase 4).
    pub fn is_platform_host(&self) -> bool {
        matches!(self.source, TenantSource::PlatformHost)
    }
}

/// Shared configuration + state for [`host_tenant_middleware`].
#[derive(Clone)]
pub struct HostTenantConfig {
    pub pool: db::DbPool,
    pub cache: Arc<TenantResolutionCache>,
    /// Driven by `RUST_ENV == "development"`. Enables `/a/{slug}` routing.
    pub dev_mode: bool,
    /// Phase 4: hosts that resolve to [`TenantSource::PlatformHost`] (the
    /// global reality portal). Lower-cased on construction; matched with
    /// equality after [`normalize_host`].
    ///
    /// Sourced from the `PLATFORM_HOST` env var (comma-separated) plus a small
    /// set of dev defaults (`localhost`, `127.0.0.1`) when `dev_mode` is on.
    /// The same list is seeded into the DB-side `reserved_platform_hosts`
    /// table by migration 00135 so the agency_domains check constraint and
    /// this resolver agree on what "the platform" is.
    pub platform_hosts: Arc<Vec<String>>,
    /// Phase 5.5: per-tenant rate limiter set used at the `SEAM(leak#15)`
    /// hook. ONE process-wide instance — share the `Arc` across both
    /// servers' main routers and any test fixtures so a flooding tenant is
    /// detected regardless of which router served the request.
    pub rate_limiters: Arc<TenantRateLimiterSet>,
}

impl HostTenantConfig {
    /// Build a config with a fresh cache (300s positive / 30s negative TTL,
    /// 10k max entries), `dev_mode` derived from `RUST_ENV`, the
    /// `PLATFORM_HOST` env var parsed into the platform-host list, and a
    /// fresh per-tenant rate limiter set seeded with the default 600 rpm
    /// baseline.
    pub fn new(pool: db::DbPool) -> Self {
        let dev_mode = std::env::var("RUST_ENV")
            .map(|v| v == "development")
            .unwrap_or(false);
        let platform_hosts = Arc::new(load_platform_hosts(dev_mode));
        Self {
            pool,
            cache: Arc::new(TenantResolutionCache::new(300, 30, 10_000)),
            dev_mode,
            platform_hosts,
            rate_limiters: Arc::new(TenantRateLimiterSet::new()),
        }
    }

    /// Test/explicit constructor: caller chooses everything except the
    /// rate-limiter set, which defaults to a fresh instance with the default
    /// rpm baseline. Use [`Self::with_rate_limiters`] to swap in a tighter
    /// limiter for tests.
    pub fn with_parts(
        pool: db::DbPool,
        cache: Arc<TenantResolutionCache>,
        dev_mode: bool,
        platform_hosts: Vec<String>,
    ) -> Self {
        Self {
            pool,
            cache,
            dev_mode,
            platform_hosts: Arc::new(platform_hosts),
            rate_limiters: Arc::new(TenantRateLimiterSet::new()),
        }
    }

    /// Builder hook: swap the per-tenant rate limiter set. Tests use this
    /// to inject a tight limiter (e.g. a few rpm) so the 429 path is
    /// reachable without flooding millions of requests.
    pub fn with_rate_limiters(mut self, limiters: Arc<TenantRateLimiterSet>) -> Self {
        self.rate_limiters = limiters;
        self
    }
}

/// Read `PLATFORM_HOST` from env (comma-separated, normalized) and tack on the
/// dev fallbacks. The default `reality.example.com` mirrors the seed in
/// migration 00135 so a zero-config dev box recognises the platform host.
fn load_platform_hosts(dev_mode: bool) -> Vec<String> {
    let mut hosts: Vec<String> = std::env::var("PLATFORM_HOST")
        .ok()
        .map(|raw| {
            raw.split(',')
                .filter_map(|h| normalize_host(h))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Always include the production placeholder so the seed in 00135 and the
    // resolver agree without needing PLATFORM_HOST in dev.
    if !hosts.iter().any(|h| h == "reality.example.com") {
        hosts.push("reality.example.com".to_string());
    }

    if dev_mode {
        for fallback in ["localhost", "127.0.0.1"] {
            if !hosts.iter().any(|h| h == fallback) {
                hosts.push(fallback.to_string());
            }
        }
    }

    hosts
}

// ============================================================================
// TenantResolutionCache
// ============================================================================

/// A cached host-resolution entry with its own expiry instant.
#[derive(Clone)]
struct CachedResolution {
    /// `None` means a *negative* cache entry (host did not resolve).
    value: Option<ResolvedTenant>,
    expires_at: Instant,
}

/// In-memory host -> resolution cache.
///
/// Mirrors the `TokenValidationCache` pattern in `reality-server/src/state.rs`:
/// `Arc<RwLock<HashMap<..>>>`, per-entry `Instant` expiry, `max_entries`
/// eviction. Positive and negative results have separate TTLs (negative
/// entries expire faster so a newly-verified domain becomes reachable sooner).
pub struct TenantResolutionCache {
    cache: Arc<RwLock<HashMap<String, CachedResolution>>>,
    ttl: Duration,
    negative_ttl: Duration,
    max_entries: usize,
}

impl TenantResolutionCache {
    /// Create a new cache.
    ///
    /// * `ttl_secs` — TTL for positive (resolved) entries.
    /// * `negative_ttl_secs` — TTL for negative (unresolved) entries.
    /// * `max_entries` — soft cap; expired entries are evicted first, then
    ///   oldest entries if still over cap.
    pub fn new(ttl_secs: u64, negative_ttl_secs: u64, max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_secs),
            negative_ttl: Duration::from_secs(negative_ttl_secs),
            max_entries,
        }
    }

    /// Look up a host.
    ///
    /// Returns:
    /// * `None` — cache miss (caller must resolve).
    /// * `Some(None)` — negative cache hit (host known-unresolved).
    /// * `Some(Some(tenant))` — positive cache hit.
    pub async fn get(&self, host: &str) -> Option<Option<ResolvedTenant>> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(host) {
            if Instant::now() < entry.expires_at {
                return Some(entry.value);
            }
        }
        None
    }

    /// Store a resolution result (positive or negative) for a host.
    pub async fn set(&self, host: &str, value: Option<ResolvedTenant>) {
        let ttl = if value.is_some() {
            self.ttl
        } else {
            self.negative_ttl
        };
        let mut cache = self.cache.write().await;

        // Evict if at capacity: expired entries first, then oldest-by-expiry.
        if cache.len() >= self.max_entries {
            let now = Instant::now();
            let expired: Vec<String> = cache
                .iter()
                .filter(|(_, v)| v.expires_at < now)
                .map(|(k, _)| k.clone())
                .collect();
            for key in expired {
                cache.remove(&key);
            }
            if cache.len() >= self.max_entries {
                let to_remove = cache.len() - self.max_entries + 1;
                let mut entries: Vec<(String, Instant)> = cache
                    .iter()
                    .map(|(k, v)| (k.clone(), v.expires_at))
                    .collect();
                entries.sort_by_key(|(_, exp)| *exp);
                for (key, _) in entries.into_iter().take(to_remove) {
                    cache.remove(&key);
                }
            }
        }

        cache.insert(
            host.to_string(),
            CachedResolution {
                value,
                expires_at: Instant::now() + ttl,
            },
        );
    }

    /// Drop a single host's cache entry (e.g. after a domain is verified).
    pub async fn invalidate(&self, host: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(host);
    }

    /// Drop every cache entry.
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }
}

// ============================================================================
// AgencyDomainCacheInvalidator impl (N6)
// ============================================================================

/// Bridges the `db`-side [`db::repositories::agency_domain::AgencyDomainCacheInvalidator`]
/// trait onto our [`TenantResolutionCache`]. The `db` crate intentionally does
/// not depend on `api-core`, so the impl lives here and is wired by the binary
/// at startup via [`AgencyDomainRepository::with_cache`].
#[async_trait::async_trait]
impl db::repositories::agency_domain::AgencyDomainCacheInvalidator for TenantResolutionCache {
    async fn invalidate(&self, host: &str) {
        TenantResolutionCache::invalidate(self, host).await;
    }
    async fn invalidate_all(&self) {
        self.clear().await;
    }
}

// ============================================================================
// Host normalization + path helpers
// ============================================================================

/// Paths that bypass tenant resolution entirely (infra / docs / health).
///
/// `/internal` is added in Phase 3 so the Caddy on-demand TLS ask-endpoint
/// (`/internal/caddy-ask`) is reachable without a resolved tenant — that
/// endpoint, by definition, runs BEFORE TLS / a host -> tenant mapping
/// exists. The endpoint enforces its own auth (loopback + shared secret).
const PUBLIC_ALLOWLIST: [&str; 6] = [
    "/health",
    "/readiness",
    "/metrics",
    "/swagger-ui",
    "/api-docs",
    "/internal",
];

/// Whether `path` is in the public allowlist (prefix match).
fn is_public_allowlist_path(path: &str) -> bool {
    PUBLIC_ALLOWLIST
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{p}/")))
}

/// Phase 3: paths that participate in resolution (so the handler can read a
/// `ResolvedTenant` when present) but DO NOT fail closed on an unresolved
/// host. The platform host (no `agency_domains` row) is a legitimate caller
/// for these — the handler returns platform defaults instead of 404.
///
/// Currently: `/tenant-config` (read by reality-web on every request,
/// including from the global portal which has no tenant).
const SOFT_RESOLVE_ALLOWLIST: [&str; 1] = ["/tenant-config"];

fn is_soft_resolve_path(path: &str) -> bool {
    SOFT_RESOLVE_ALLOWLIST
        .iter()
        .any(|p| path == *p || path.starts_with(&format!("{p}/")))
}

/// Normalize a raw host string into a canonical form, or `None` if invalid.
///
/// * lowercases
/// * strips an `:port` suffix
/// * strips a single trailing dot
/// * rejects empty hosts, hosts with whitespace/control chars, or length > 253
pub fn normalize_host(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    // Reject control chars / whitespace anywhere in the host.
    if raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }

    // Strip port. Note: IPv6 literals (`[::1]:8080`) are not expected for tenant
    // hosts; a bracketed host without a port still works, and `[::1]:8080`
    // would split on the last colon leaving `[::1]` which simply won't resolve.
    let without_port = match raw.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) && !port.is_empty() => host,
        _ => raw,
    };

    // Strip a single trailing dot (FQDN root).
    let trimmed = without_port.strip_suffix('.').unwrap_or(without_port);

    let normalized = trimmed.to_ascii_lowercase();
    if normalized.is_empty() || normalized.len() > 253 {
        return None;
    }
    Some(normalized)
}

/// If `path` is a dev-mode `/a/{slug}` path, return `(slug, rest)` where `rest`
/// is the remainder of the path to rewrite to (always starts with `/`).
///
/// Equivalent to the regex `^/a/([a-z0-9-]+)(/.*)?$`.
pub fn parse_dev_slug_path(path: &str) -> Option<(&str, &str)> {
    let after = path.strip_prefix("/a/")?;
    // slug runs until the next '/' or end of string.
    let (slug, rest) = match after.find('/') {
        Some(idx) => (&after[..idx], &after[idx..]),
        None => (after, ""),
    };
    if slug.is_empty()
        || !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return None;
    }
    let rest = if rest.is_empty() { "/" } else { rest };
    Some((slug, rest))
}

/// Extract the trusted host from request headers.
///
/// Uses the `Host` header (HTTP/1.1) / `:authority` (HTTP/2 — axum surfaces it
/// via the `Host` header too). `X-Forwarded-Host` is consulted ONLY when
/// `TRUST_FORWARDED_HOST=true`.
fn extract_trusted_host(request: &Request<Body>) -> Option<String> {
    let trust_forwarded = std::env::var("TRUST_FORWARDED_HOST")
        .map(|v| v == "true")
        .unwrap_or(false);

    if trust_forwarded {
        if let Some(fwd) = request
            .headers()
            .get("x-forwarded-host")
            .and_then(|v| v.to_str().ok())
        {
            // X-Forwarded-Host may contain a comma-separated list; take the first.
            let first = fwd.split(',').next().unwrap_or(fwd);
            if let Some(host) = normalize_host(first) {
                return Some(host);
            }
        }
    }

    request
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .and_then(normalize_host)
}

/// Rewrite a request URI, replacing the path while preserving the query string.
fn rewrite_uri_path(uri: &Uri, new_path: &str) -> Result<Uri, ()> {
    let path_and_query = match uri.query() {
        Some(q) => format!("{new_path}?{q}"),
        None => new_path.to_string(),
    };
    // Rebuild from parts so scheme/authority (if any) are preserved.
    let mut parts = uri.clone().into_parts();
    parts.path_and_query = Some(path_and_query.parse().map_err(|_| ())?);
    Uri::from_parts(parts).map_err(|_| ())
}

// ============================================================================
// Middleware
// ============================================================================

/// Host-resolution middleware. See module docs for the full algorithm.
pub async fn host_tenant_middleware(
    State(cfg): State<HostTenantConfig>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, (StatusCode, &'static str)> {
    let path = request.uri().path().to_string();

    // 1. Public-allowlist paths bypass resolution entirely. No rate limit,
    //    no metering — these are infra/health endpoints.
    if is_public_allowlist_path(&path) {
        return Ok(next.run(request).await);
    }

    // 2. Extract the trusted host.
    let soft = is_soft_resolve_path(&path);
    let host = match extract_trusted_host(&request) {
        Some(h) => h,
        None => {
            if soft {
                // Soft path: no host means platform-default response.
                tracing::debug!(path = %path, "Soft-resolve path with no Host header — pass through unresolved");
                return Ok(next.run(request).await);
            }
            tracing::warn!(path = %path, "Tenant resolution: missing/invalid Host header");
            return Err((StatusCode::NOT_FOUND, "Unknown tenant host"));
        }
    };

    // 2.5 Phase 4: PlatformHost short-circuit.
    //
    // If the inbound host matches a PLATFORM_HOST sentinel, we resolve to the
    // 4th RLS context (global-read across all orgs) BEFORE consulting
    // `agency_domains`. The schema-level `agency_domains_host_not_reserved`
    // check constraint (migration 00135) prevents an agency from owning the
    // same host, so this branch can never be wrong about ownership.
    //
    // PlatformHost requests carry the nil UUID as `organization_id` — a
    // sentinel meaning "no specific tenant." Downstream code MUST branch on
    // `source == PlatformHost` and not treat the id as a real org id.
    if cfg
        .platform_hosts
        .iter()
        .any(|ph| ph.as_str() == host.as_str())
    {
        let resolved = ResolvedTenant {
            organization_id: Uuid::nil(),
            source: TenantSource::PlatformHost,
        };
        request.extensions_mut().insert(resolved);
        // SEAM(leak#15): PlatformHost intentionally bypasses per-tenant rate
        // limiting. The nil UUID is a sentinel meaning "no specific tenant",
        // so binding all platform traffic to a single bucket would be an
        // artificial bottleneck. Edge / ingress layers (CDN + load balancer)
        // already cap inbound platform-host volume; this middleware would
        // duplicate that without adding tenant isolation.
        let response = next.run(request).await;
        // SEAM(leak#19): we still meter PlatformHost requests so platform-
        // wide traffic shows up in `requests_total` (org_id = nil) for
        // capacity planning. No per-tenant billing implication.
        let bytes = response_content_length(&response);
        meter_request(Uuid::nil(), bytes);
        return Ok(response);
    }

    // 3. Dev-mode `/a/{slug}` path routing.
    if cfg.dev_mode {
        if let Some((slug, rest)) = parse_dev_slug_path(&path) {
            let repo = AgencyDomainRepository::new(cfg.pool.clone());
            let org_id = repo.resolve_slug_system(slug).await.map_err(|e| {
                tracing::error!(error = %e, slug = %slug, "resolve_slug_system failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Tenant resolution error")
            })?;

            match org_id {
                Some(organization_id) => {
                    // Rewrite the URI to strip the `/a/{slug}` prefix.
                    let new_uri = rewrite_uri_path(request.uri(), rest).map_err(|_| {
                        tracing::error!(slug = %slug, "failed to rewrite dev-slug URI");
                        (StatusCode::INTERNAL_SERVER_ERROR, "Tenant resolution error")
                    })?;
                    *request.uri_mut() = new_uri;

                    let resolved = ResolvedTenant {
                        organization_id,
                        source: TenantSource::DevPath,
                    };
                    request.extensions_mut().insert(resolved);
                    return Ok(rate_limit_and_meter(&cfg, organization_id, request, next).await);
                }
                None => {
                    tracing::warn!(slug = %slug, "dev-mode /a/{{slug}} did not resolve");
                    return Err((StatusCode::NOT_FOUND, "Unknown tenant host"));
                }
            }
        }
    }

    // 4. Host-based resolution: cache, then system lookup on miss.
    let resolved = match cfg.cache.get(&host).await {
        Some(cached) => cached,
        None => {
            let repo = AgencyDomainRepository::new(cfg.pool.clone());
            let org_id = repo.resolve_host_system(&host).await.map_err(|e| {
                tracing::error!(error = %e, host = %host, "resolve_host_system failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Tenant resolution error")
            })?;
            // `source` is Subdomain by default. Fetching `kind` to distinguish
            // CustomDomain is intentionally deferred — Wave 3 does not branch on
            // it. See the contract note in the Phase 1 spec.
            let value = org_id.map(|organization_id| ResolvedTenant {
                organization_id,
                source: TenantSource::Subdomain,
            });
            cfg.cache.set(&host, value).await;
            value
        }
    };

    match resolved {
        Some(resolved) => {
            let org_id = resolved.organization_id;
            request.extensions_mut().insert(resolved);
            Ok(rate_limit_and_meter(&cfg, org_id, request, next).await)
        }
        None => {
            if soft {
                // Soft path: pass through with no `ResolvedTenant` extension
                // — the handler returns platform defaults for the global
                // portal case.
                tracing::debug!(host = %host, "Soft-resolve path: unresolved host passes through");
                Ok(next.run(request).await)
            } else {
                // 6. Fail closed — an unknown host must never reach a handler.
                tracing::warn!(host = %host, "Tenant resolution: unknown host, failing closed (404)");
                Err((StatusCode::NOT_FOUND, "Unknown tenant host"))
            }
        }
    }
}

/// Combine the per-tenant rate-limit gate (defense leak #15) and the
/// per-tenant metering counter (defense leak #19) around the downstream
/// service call. Returns the response that should be sent to the caller —
/// either the 429 we synthesized when the rate limiter denied, or the
/// downstream service's response (with metering already recorded).
///
/// Exposed at crate visibility so the integration smoke tests in
/// `tests/host_tenant_middleware_tests.rs` can drive the wiring without
/// standing up a database for the host-resolution path.
pub async fn rate_limit_and_meter(
    cfg: &HostTenantConfig,
    org_id: Uuid,
    request: Request<Body>,
    next: Next,
) -> Response {
    // SEAM(leak#15): per-tenant rate limit. A flooding tenant gets a 429
    // here without affecting the latency of any other tenant — the
    // limiter's state is per-org-id.
    if let Err(retry_after) = cfg.rate_limiters.check_with_retry(org_id).await {
        tracing::warn!(
            org_id = %org_id,
            retry_after_secs = retry_after.as_secs(),
            "Per-tenant rate limit exceeded — returning 429"
        );
        // Round up so a sub-second retry-after still nudges clients to wait
        // ≥1 second; HTTP `Retry-After` is integer seconds.
        let retry_secs = retry_after.as_secs().max(1);
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("Retry-After", retry_secs.to_string())
            .header(
                axum::http::header::CONTENT_TYPE,
                "text/plain; charset=utf-8",
            )
            .body(Body::from("rate limit exceeded for this tenant"))
            .expect("static 429 response is valid");
    }

    let response = next.run(request).await;

    // SEAM(leak#19): per-tenant metering. Best-effort body length from
    // `Content-Length`. Streaming/chunked responses lack a known length up
    // front; counting only the request itself (response_bytes = 0) still
    // yields the request-rate signal needed for billing.
    let bytes = response_content_length(&response);
    meter_request(org_id, bytes);

    response
}

/// Best-effort response body length from the `Content-Length` header.
/// Returns 0 when the header is absent or unparseable (chunked transfer
/// encoding, etc.). The metering counter still increments the request
/// count via [`meter_request`]'s unconditional `requests_total` bump; only
/// the byte counter is omitted.
fn response_content_length(response: &Response) -> u64 {
    response
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

// ============================================================================
// ResolvedTenantExtractor
// ============================================================================

/// Extractor that pulls the [`ResolvedTenant`] injected by
/// [`host_tenant_middleware`] out of the request extensions.
///
/// Returns `500` if absent — that means the middleware did not run, which is a
/// server misconfiguration (the middleware fails closed for unresolved hosts,
/// so a missing extension is never a client error).
pub struct ResolvedTenantExtractor(pub ResolvedTenant);

impl<S> axum::extract::FromRequestParts<S> for ResolvedTenantExtractor
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<ResolvedTenant>()
            .copied()
            .map(ResolvedTenantExtractor)
            .ok_or((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Tenant not resolved (host_tenant_middleware did not run)",
            ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_host_lowercases() {
        assert_eq!(
            normalize_host("ACME.Example.COM").as_deref(),
            Some("acme.example.com")
        );
    }

    #[test]
    fn normalize_host_strips_port() {
        assert_eq!(
            normalize_host("acme.example.com:8080").as_deref(),
            Some("acme.example.com")
        );
        assert_eq!(
            normalize_host("acme.example.com:443").as_deref(),
            Some("acme.example.com")
        );
    }

    #[test]
    fn normalize_host_strips_trailing_dot() {
        assert_eq!(
            normalize_host("acme.example.com.").as_deref(),
            Some("acme.example.com")
        );
    }

    #[test]
    fn normalize_host_strips_port_and_trailing_dot() {
        assert_eq!(
            normalize_host("ACME.example.com.:8080").as_deref(),
            Some("acme.example.com")
        );
    }

    #[test]
    fn normalize_host_rejects_empty() {
        assert_eq!(normalize_host(""), None);
        assert_eq!(normalize_host("   "), None);
    }

    #[test]
    fn normalize_host_rejects_whitespace_and_control() {
        assert_eq!(normalize_host("acme .example.com"), None);
        assert_eq!(normalize_host("acme\texample.com"), None);
        assert_eq!(normalize_host("acme\nexample.com"), None);
        assert_eq!(normalize_host("acme\u{0007}.com"), None);
    }

    #[test]
    fn normalize_host_rejects_too_long() {
        let long = format!("{}.com", "a".repeat(260));
        assert_eq!(normalize_host(&long), None);
        // exactly 253 is allowed
        let ok = "a".repeat(253);
        assert_eq!(normalize_host(&ok).as_deref(), Some(ok.as_str()));
    }

    #[test]
    fn normalize_host_non_numeric_port_suffix_kept() {
        // A trailing `:something` that is not all digits is NOT a port; the
        // resulting host just won't resolve, but we don't strip it.
        assert_eq!(
            normalize_host("acme.example.com:abc").as_deref(),
            Some("acme.example.com:abc")
        );
    }

    #[test]
    fn parse_dev_slug_path_basic() {
        assert_eq!(parse_dev_slug_path("/a/acme"), Some(("acme", "/")));
    }

    #[test]
    fn parse_dev_slug_path_with_rest() {
        assert_eq!(
            parse_dev_slug_path("/a/acme/listings/123"),
            Some(("acme", "/listings/123"))
        );
        assert_eq!(parse_dev_slug_path("/a/acme/"), Some(("acme", "/")));
    }

    #[test]
    fn parse_dev_slug_path_allows_digits_and_hyphens() {
        assert_eq!(parse_dev_slug_path("/a/acme-2/x"), Some(("acme-2", "/x")));
    }

    #[test]
    fn parse_dev_slug_path_rejects_non_matching() {
        assert_eq!(parse_dev_slug_path("/api/v1/listings"), None);
        assert_eq!(parse_dev_slug_path("/a/"), None);
        assert_eq!(parse_dev_slug_path("/a"), None);
        // uppercase / underscores / dots are not valid slug chars
        assert_eq!(parse_dev_slug_path("/a/Acme"), None);
        assert_eq!(parse_dev_slug_path("/a/ac_me"), None);
        assert_eq!(parse_dev_slug_path("/a/ac.me"), None);
    }

    #[test]
    fn public_allowlist_paths_match() {
        assert!(is_public_allowlist_path("/health"));
        assert!(is_public_allowlist_path("/readiness"));
        assert!(is_public_allowlist_path("/metrics"));
        assert!(is_public_allowlist_path("/swagger-ui"));
        assert!(is_public_allowlist_path("/swagger-ui/index.html"));
        assert!(is_public_allowlist_path("/api-docs"));
        assert!(is_public_allowlist_path("/api-docs/openapi.json"));
    }

    #[test]
    fn public_allowlist_paths_no_false_positives() {
        assert!(!is_public_allowlist_path("/api/v1/listings"));
        assert!(!is_public_allowlist_path("/healthcheck"));
        assert!(!is_public_allowlist_path("/metricsx"));
        assert!(!is_public_allowlist_path("/"));
    }

    #[tokio::test]
    async fn cache_miss_then_hit() {
        let cache = TenantResolutionCache::new(300, 30, 100);
        assert!(cache.get("acme.example.com").await.is_none());

        let tenant = ResolvedTenant {
            organization_id: Uuid::new_v4(),
            source: TenantSource::Subdomain,
        };
        cache.set("acme.example.com", Some(tenant)).await;

        let hit = cache.get("acme.example.com").await;
        assert!(matches!(hit, Some(Some(_))));
        assert_eq!(
            hit.unwrap().unwrap().organization_id,
            tenant.organization_id
        );
    }

    #[tokio::test]
    async fn cache_negative_entry() {
        let cache = TenantResolutionCache::new(300, 30, 100);
        cache.set("unknown.example.com", None).await;
        // Negative hit: Some(None).
        assert!(matches!(cache.get("unknown.example.com").await, Some(None)));
    }

    #[tokio::test]
    async fn cache_ttl_expiry() {
        // 0-second TTL => entries are immediately expired.
        let cache = TenantResolutionCache::new(0, 0, 100);
        let tenant = ResolvedTenant {
            organization_id: Uuid::new_v4(),
            source: TenantSource::Subdomain,
        };
        cache.set("acme.example.com", Some(tenant)).await;
        // Allow the Instant to advance past the 0s TTL.
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(cache.get("acme.example.com").await.is_none());
    }

    #[tokio::test]
    async fn cache_invalidate_and_clear() {
        let cache = TenantResolutionCache::new(300, 30, 100);
        let tenant = ResolvedTenant {
            organization_id: Uuid::new_v4(),
            source: TenantSource::Subdomain,
        };
        cache.set("a.example.com", Some(tenant)).await;
        cache.set("b.example.com", Some(tenant)).await;

        cache.invalidate("a.example.com").await;
        assert!(cache.get("a.example.com").await.is_none());
        assert!(cache.get("b.example.com").await.is_some());

        cache.clear().await;
        assert!(cache.get("b.example.com").await.is_none());
    }

    #[tokio::test]
    async fn cache_eviction_respects_max_entries() {
        let cache = TenantResolutionCache::new(300, 30, 3);
        let tenant = ResolvedTenant {
            organization_id: Uuid::new_v4(),
            source: TenantSource::Subdomain,
        };
        // Insert more than max_entries; cache must not grow unbounded.
        for i in 0..10 {
            cache
                .set(&format!("host-{i}.example.com"), Some(tenant))
                .await;
        }
        let len = cache.cache.read().await.len();
        assert!(len <= 3, "cache grew beyond max_entries: {len}");
    }

    // NOTE: DB-dependent paths (resolve_host_system / resolve_slug_system and
    // the full `host_tenant_middleware` request flow) are exercised by
    // integration tests in a later wave — they require a live Postgres with
    // migrations applied and `agency_domains` rows seeded.

    // ============================================================================
    // Phase 4: PlatformHost wiring.
    // ============================================================================

    #[test]
    fn platform_host_default_includes_reality_example_com() {
        // No env var set, no dev_mode: default fallback is `reality.example.com`
        // (mirrors the seed in migration 00135).
        let hosts = load_platform_hosts(false);
        assert!(
            hosts.iter().any(|h| h == "reality.example.com"),
            "default platform host list missing reality.example.com: {hosts:?}"
        );
    }

    #[test]
    fn platform_host_dev_mode_adds_localhost_fallbacks() {
        let hosts = load_platform_hosts(true);
        for expected in ["reality.example.com", "localhost", "127.0.0.1"] {
            assert!(
                hosts.iter().any(|h| h == expected),
                "dev-mode platform host list missing {expected}: {hosts:?}"
            );
        }
    }

    #[test]
    fn platform_host_env_overrides_are_normalized() {
        // Standalone helper test — we don't `set_var` the global env because
        // that races with parallel tests. We re-check `normalize_host` does
        // the right thing, then trust `load_platform_hosts` which delegates.
        assert_eq!(
            normalize_host("Reality.Production.COM").as_deref(),
            Some("reality.production.com")
        );
    }

    #[test]
    fn resolved_tenant_is_platform_host_helper() {
        let agency = ResolvedTenant {
            organization_id: Uuid::new_v4(),
            source: TenantSource::Subdomain,
        };
        let platform = ResolvedTenant {
            organization_id: Uuid::nil(),
            source: TenantSource::PlatformHost,
        };
        assert!(!agency.is_platform_host());
        assert!(platform.is_platform_host());
        assert_eq!(platform.organization_id, Uuid::nil());
    }
}
