//! Caddy on-demand TLS ask-endpoint (Phase 3, defends leak #4).
//!
//! Caddy queries this endpoint BEFORE asking Let's Encrypt for a cert for a
//! never-seen-before host. The endpoint answers `200` if and only if the
//! host is already registered in `agency_domains` with a `verifying` or
//! `verified` state — i.e. a tenant we expect to issue a cert for.
//!
//! # Defense (leak #4: Caddy on-demand TLS as DoS amplifier)
//!
//! Without this gate, a flood of unknown-host TLS handshakes would each
//! trigger a Let's Encrypt order, eventually tripping LE's per-domain
//! rate-limit and locking us out of certificate issuance for the rest of
//! our agencies. This endpoint enforces "never request a cert for a host
//! we don't already know."
//!
//! # Wire-level contract with Caddy
//!
//! Per Caddy's docs: the `ask` endpoint receives `GET /?domain=<host>`. If
//! it returns ANY 2xx, Caddy proceeds; on anything else (including 5xx),
//! Caddy refuses to issue. We deliberately answer `403` (rather than 4xx
//! `400`) for the unknown-host case so Caddy logs make the cause obvious;
//! malformed input is a separate `400`.
//!
//! # Auth model
//!
//! - **Dev:** the endpoint must only be exposed on `127.0.0.1` (the Caddy
//!   sidecar lives on the same host). No header auth required.
//! - **Prod:** require a shared-secret `X-Internal-Token` header matching
//!   `INTERNAL_API_TOKEN`. The Caddyfile injects it via
//!   `header X-Internal-Token {env.INTERNAL_API_TOKEN}` so the secret
//!   never leaves the host.
//!
//! In production, missing/wrong tokens return `401` so a misconfigured
//! Caddy fails loud and Let's Encrypt is never contacted.
//!
//! # Per-IP rate limit
//!
//! Even with the allowlist, a single IP can spray distinct subdomains we
//! happen to recognize. A cheap per-IP token bucket (60/min) is layered
//! on top to bound CPU + DB load. Mounted independently of Caddy's own
//! `internal` rate limit module so we degrade gracefully if Caddy's
//! limiter is misconfigured.

use axum::{
    extract::{ConnectInfo, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use db::repositories::AgencyDomainRepository;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::state::AppState;

/// Mount `/internal/caddy-ask`. Mounted directly in `main.rs` with NO host
/// resolution middleware in front — this endpoint runs BEFORE TLS exists,
/// so by definition the request has no resolved tenant.
pub fn router() -> Router<AppState> {
    Router::new().route("/", get(caddy_ask))
}

#[derive(Debug, Deserialize)]
pub struct CaddyAskQuery {
    /// The domain Caddy is about to issue a certificate for.
    pub domain: String,
}

/// Normalize a Caddy-supplied domain to the same form `agency_domains.host`
/// is stored in.
///
/// Inlined here (rather than reused from `api_core::middleware::host_tenant::normalize_host`)
/// because:
/// 1. This file is a security boundary — the algorithm is short and we want
///    it visible at the call site.
/// 2. The middleware version normalizes the inbound `Host` header; this is
///    a Caddy-supplied query param. Same logic, separate concerns.
fn normalize_caddy_domain(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.len() > 253 {
        return None;
    }
    if raw.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    let without_port = match raw.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => host,
        _ => raw,
    };
    let trimmed = without_port.strip_suffix('.').unwrap_or(without_port);
    let normalized = trimmed.to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    Some(normalized)
}

// ============================================================================
// Per-IP rate limit (60 requests / minute, sliding window approximated as
// a simple token bucket per IP). Process-local — fine because this endpoint
// only runs on the loopback; the Caddy sidecar is the only legitimate
// caller.
// ============================================================================

const RATE_LIMIT_PER_MIN: u32 = 60;
const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

struct RateLimitEntry {
    count: u32,
    window_start: Instant,
}

static RATE_LIMITER: once_cell::sync::Lazy<Mutex<HashMap<std::net::IpAddr, RateLimitEntry>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));

fn rate_limit_check(ip: std::net::IpAddr) -> bool {
    let mut map = match RATE_LIMITER.lock() {
        Ok(g) => g,
        // Lock poisoning is non-fatal here — if the limiter crashes we'd
        // rather pass requests through than 500 on a healthy ask flow.
        Err(p) => p.into_inner(),
    };
    let now = Instant::now();
    let entry = map.entry(ip).or_insert(RateLimitEntry {
        count: 0,
        window_start: now,
    });
    if now.duration_since(entry.window_start) >= RATE_LIMIT_WINDOW {
        entry.count = 0;
        entry.window_start = now;
    }
    entry.count = entry.count.saturating_add(1);
    entry.count <= RATE_LIMIT_PER_MIN
}

// ============================================================================
// Handler
// ============================================================================

/// Whether this request is allowed past the auth gate.
///
/// Logic:
/// - Production (`RUST_ENV != "development"`): `X-Internal-Token` header
///   MUST equal `INTERNAL_API_TOKEN`. Missing/blank token in prod = 401.
/// - Development: any caller is accepted (the dev compose file binds
///   the endpoint to `127.0.0.1`).
///
/// This is intentionally NOT IP-checked here — IP-binding is the Caddyfile's
/// job (`bind 127.0.0.1`); auditing two layers of "who can call this" tends
/// to drift. The header is the contract.
fn auth_gate(headers: &HeaderMap) -> Result<(), StatusCode> {
    let is_dev = std::env::var("RUST_ENV")
        .map(|v| v == "development")
        .unwrap_or(false);
    if is_dev {
        return Ok(());
    }

    let expected = match std::env::var("INTERNAL_API_TOKEN") {
        Ok(t) if !t.is_empty() => t,
        _ => {
            tracing::error!(
                "INTERNAL_API_TOKEN missing in non-development environment; \
                 caddy-ask refuses all requests"
            );
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let supplied = headers
        .get("x-internal-token")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    // Length-checked, branchless compare.
    if supplied.len() == expected.len()
        && constant_time_eq(supplied.as_bytes(), expected.as_bytes())
    {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// `GET /internal/caddy-ask?domain={host}`
///
/// Caddy on-demand TLS ask-endpoint. See module docs for the contract.
pub async fn caddy_ask(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<CaddyAskQuery>,
) -> impl IntoResponse {
    if let Err(code) = auth_gate(&headers) {
        return code;
    }

    if !rate_limit_check(addr.ip()) {
        tracing::warn!(ip = %addr.ip(), "caddy-ask rate-limited");
        return StatusCode::TOO_MANY_REQUESTS;
    }

    let host = match normalize_caddy_domain(&q.domain) {
        Some(h) => h,
        None => {
            tracing::warn!(domain = %q.domain, "caddy-ask: malformed domain");
            return StatusCode::BAD_REQUEST;
        }
    };

    let repo = AgencyDomainRepository::new(state.db.clone());
    // Resolve via the SAME pre-auth path the host-resolution middleware uses.
    // `resolve_host_system` returns Some(org_id) iff the domain row exists
    // AND its verification_state is 'verified'. For the cert-issuance gate
    // we want to allow `verifying` too (cert + domain verification race),
    // so we re-query directly when the verified path returns None.
    match repo.resolve_host_system(&host).await {
        Ok(Some(_org_id)) => {
            tracing::debug!(host = %host, "caddy-ask: verified host, allowing cert");
            StatusCode::OK
        }
        Ok(None) => {
            // Fall through to a second check that also accepts `verifying`.
            // We deliberately do NOT match on `pending` or `failed` — those
            // are not authorized to mint certs.
            match host_in_verifying_state(state.db.clone(), &host).await {
                Ok(true) => {
                    tracing::debug!(host = %host, "caddy-ask: verifying host, allowing cert");
                    StatusCode::OK
                }
                Ok(false) => {
                    tracing::info!(host = %host, "caddy-ask: unknown host, refusing cert");
                    StatusCode::FORBIDDEN
                }
                Err(e) => {
                    tracing::error!(error = %e, host = %host, "caddy-ask: DB error on verifying lookup");
                    // Defense: any DB error is a refusal, not a 5xx.
                    // 5xx would let Caddy's "ask returned non-200, refuse"
                    // logic still hold — but we make it explicit here so the
                    // contract ("only 200 means yes") is honored even if Caddy
                    // version semantics ever drift.
                    StatusCode::SERVICE_UNAVAILABLE
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, host = %host, "caddy-ask: DB error on resolve_host_system");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}

/// Secondary lookup: whether `host` is in `verifying` state. Same system-
/// connection discipline as `AgencyDomainRepository::resolve_host_system`.
async fn host_in_verifying_state(
    pool: db::DbPool,
    host: &str,
) -> Result<bool, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    db::tenant_context::set_request_context(&mut *conn, None, None, true).await?;

    let result = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT organization_id
        FROM agency_domains
        WHERE host = $1 AND verification_state = 'verifying'
        "#,
    )
    .bind(host)
    .fetch_optional(&mut *conn)
    .await
    .map(|r| r.is_some());

    if let Err(e) = db::tenant_context::clear_request_context(&mut *conn).await {
        tracing::warn!(
            error = %e,
            "Failed to clear system RLS context after host_in_verifying_state"
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_caddy_domain_basics() {
        assert_eq!(
            normalize_caddy_domain("ACME.example.com").as_deref(),
            Some("acme.example.com")
        );
        assert_eq!(
            normalize_caddy_domain("acme.example.com:443").as_deref(),
            Some("acme.example.com")
        );
        assert_eq!(
            normalize_caddy_domain("acme.example.com.").as_deref(),
            Some("acme.example.com")
        );
        assert_eq!(normalize_caddy_domain(""), None);
        assert_eq!(normalize_caddy_domain("  "), None);
        assert_eq!(normalize_caddy_domain("acme example.com"), None);
        assert_eq!(normalize_caddy_domain("acme\nexample.com"), None);
    }

    #[test]
    fn constant_time_eq_basics() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hello!"));
    }

    #[test]
    fn rate_limiter_blocks_after_threshold() {
        let ip: std::net::IpAddr = "192.0.2.1".parse().unwrap();
        // Reset state for this IP.
        if let Ok(mut g) = RATE_LIMITER.lock() {
            g.remove(&ip);
        }
        for _ in 0..RATE_LIMIT_PER_MIN {
            assert!(rate_limit_check(ip));
        }
        assert!(!rate_limit_check(ip));
    }
}
