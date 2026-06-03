//! Baseline security response headers for the API edge (#954).
//!
//! Both api-server and reality-server previously shipped **zero** security
//! headers on their JSON API responses — no HSTS, no `X-Content-Type-Options`,
//! no framing/referrer policy — even though they sit behind TLS and carry auth
//! credentials. The reality-web (Next.js) host already ships the full suite;
//! this middleware brings the Rust API hosts up to the same baseline.
//!
//! Headers set on every API response:
//! - `Strict-Transport-Security` — close the first-request SSL-strip window.
//! - `X-Content-Type-Options: nosniff` — never MIME-sniff a JSON body.
//! - `X-Frame-Options: DENY` — an API has nothing to embed in a frame.
//! - `Referrer-Policy` — conservative default.
//! - `Content-Security-Policy: default-src 'none'` — the API emits JSON,
//!   never HTML/CSS/JS; `default-src 'none'` is the tightest possible policy
//!   and acts as a defence-in-depth signal if a browser ever tries to render
//!   a response as a document. HTML-serving edges (reality-web / SPA nginx)
//!   use their own, richer CSP configured per-host.

use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

/// HSTS value: 2 years + subdomains. Matches the reality-web baseline
/// (`preload` is omitted here since the API hosts are not on the HSTS preload
/// list; the SPA/edge owns the preloaded apex).
const HSTS_VALUE: &str = "max-age=63072000; includeSubDomains";

/// CSP for a pure JSON API: nothing is ever rendered, so `default-src 'none'`
/// is the correct baseline. HTML edges (SPA nginx, Next.js) apply their own
/// richer policies; this value must not be widened here.
const CSP_VALUE: &str = "default-src 'none'";

/// Axum middleware that attaches the baseline security headers to every
/// response. Register with `axum::middleware::from_fn(security_headers)`.
///
/// Uses `insert` (override) rather than `entry`/append: these headers are never
/// set by the API handlers themselves, and a single deterministic value is what
/// we want at the edge.
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        HeaderValue::from_static(HSTS_VALUE),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP_VALUE),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt; // for `oneshot`

    /// Build a minimal test router wrapped in the security_headers layer.
    fn test_router() -> Router {
        Router::new()
            .route("/ok", get(|| async { "ok" }))
            .route(
                "/error",
                get(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "boom") }),
            )
            .layer(axum::middleware::from_fn(security_headers))
    }

    /// Issue a GET to `uri` through `app` and return the response.
    async fn get_response(app: Router, uri: &str) -> axum::response::Response {
        app.oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // ── Regression tests for PR #963 ──────────────────────────────────────────
    // These assert the exact header values shipped in the middleware so any
    // future edit that changes a value (e.g. loosening CSP or HSTS) is caught
    // at the unit-test level before reaching CI.

    /// HSTS must be present with a 2-year max-age and includeSubDomains.
    /// Changing this value weakens the SSL-strip protection window.
    #[tokio::test]
    async fn security_header_hsts_is_set() {
        let response = get_response(test_router(), "/ok").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::STRICT_TRANSPORT_SECURITY)
                .expect("HSTS header must be present"),
            "max-age=63072000; includeSubDomains",
            "HSTS value changed — verify the new value is still ≥2 years with includeSubDomains"
        );
    }

    /// X-Content-Type-Options must be `nosniff` to prevent MIME-type confusion
    /// attacks on browsers that honour the hint.
    #[tokio::test]
    async fn security_header_x_content_type_options_is_nosniff() {
        let response = get_response(test_router(), "/ok").await;
        assert_eq!(
            response
                .headers()
                .get(header::X_CONTENT_TYPE_OPTIONS)
                .expect("X-Content-Type-Options header must be present"),
            "nosniff",
            "X-Content-Type-Options must be 'nosniff'"
        );
    }

    /// CSP must be `default-src 'none'` for the API layer.
    ///
    /// A JSON API emits no markup, so the strictest possible policy is correct
    /// here. The HTML-serving edges (SPA nginx, Next.js) use their own, wider
    /// policies. Widening this value without a deliberate security review is a
    /// regression.
    #[tokio::test]
    async fn security_header_csp_is_default_src_none() {
        let response = get_response(test_router(), "/ok").await;
        assert_eq!(
            response
                .headers()
                .get(header::CONTENT_SECURITY_POLICY)
                .expect("Content-Security-Policy header must be present"),
            "default-src 'none'",
            "API CSP must remain 'default-src none' — widen only with an explicit security review"
        );
    }

    /// All four baseline headers must appear together in a single response.
    /// This is a combined smoke-test that ensures the middleware does not
    /// accidentally skip one header.
    #[tokio::test]
    async fn security_headers_all_present_on_200() {
        let response = get_response(test_router(), "/ok").await;
        assert_eq!(response.status(), StatusCode::OK);
        let h = response.headers();

        assert_eq!(
            h.get(header::STRICT_TRANSPORT_SECURITY).unwrap(),
            "max-age=63072000; includeSubDomains"
        );
        assert_eq!(h.get(header::X_CONTENT_TYPE_OPTIONS).unwrap(), "nosniff");
        assert_eq!(h.get(header::X_FRAME_OPTIONS).unwrap(), "DENY");
        assert_eq!(
            h.get(header::REFERRER_POLICY).unwrap(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(
            h.get(header::CONTENT_SECURITY_POLICY).unwrap(),
            "default-src 'none'"
        );
    }

    /// Security headers must be present even when the handler returns a 5xx.
    /// The middleware must not be bypassed on error paths.
    #[tokio::test]
    async fn security_headers_present_on_error_response() {
        let response = get_response(test_router(), "/error").await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let h = response.headers();

        assert!(
            h.get(header::STRICT_TRANSPORT_SECURITY).is_some(),
            "HSTS must be set even on 5xx responses"
        );
        assert!(
            h.get(header::X_CONTENT_TYPE_OPTIONS).is_some(),
            "X-Content-Type-Options must be set even on 5xx responses"
        );
        assert!(
            h.get(header::CONTENT_SECURITY_POLICY).is_some(),
            "CSP must be set even on 5xx responses"
        );
    }

    /// Security headers must be present on 404 responses (unmatched routes).
    /// axum returns its own 404 body without going through a handler, so the
    /// middleware must still fire.
    #[tokio::test]
    async fn security_headers_present_on_404() {
        let response = get_response(test_router(), "/nonexistent").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let h = response.headers();

        assert!(
            h.get(header::STRICT_TRANSPORT_SECURITY).is_some(),
            "HSTS must be set on 404 responses"
        );
        assert!(
            h.get(header::X_CONTENT_TYPE_OPTIONS).is_some(),
            "X-Content-Type-Options must be set on 404 responses"
        );
        assert!(
            h.get(header::CONTENT_SECURITY_POLICY).is_some(),
            "CSP must be set on 404 responses"
        );
    }
}
