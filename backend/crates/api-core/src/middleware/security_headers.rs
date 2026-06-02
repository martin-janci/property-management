//! Baseline security response headers for the API edge (#954).
//!
//! Both api-server and reality-server previously shipped **zero** security
//! headers on their JSON API responses — no HSTS, no `X-Content-Type-Options`,
//! no framing/referrer policy — even though they sit behind TLS and carry auth
//! credentials. The reality-web (Next.js) host already ships the full suite;
//! this middleware brings the Rust API hosts up to the same baseline.
//!
//! It deliberately sets only the headers that make sense for a JSON API:
//! - `Strict-Transport-Security` — close the first-request SSL-strip window.
//! - `X-Content-Type-Options: nosniff` — never MIME-sniff a JSON body.
//! - `X-Frame-Options: DENY` — an API has nothing to embed in a frame.
//! - `Referrer-Policy` — conservative default.
//!
//! Content-Security-Policy is intentionally left to the HTML-serving edges
//! (reality-web / the SPA nginx) — a JSON API renders no markup, so a CSP here
//! would add bytes without protecting anything.

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

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::StatusCode, routing::get, Router};
    use tower::ServiceExt; // for `oneshot`

    async fn ok() -> &'static str {
        "ok"
    }

    #[tokio::test]
    async fn sets_all_baseline_headers() {
        let app = Router::new()
            .route("/health", get(ok))
            .layer(axum::middleware::from_fn(security_headers));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

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
    }
}
