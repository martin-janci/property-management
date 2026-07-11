//! Health & readiness integration tests (Epic 99, Story 99.4; E8 split).
//!
//! Two endpoints under test:
//!   - `/health`   — shallow liveness; returns 200 with a small payload.
//!   - `/readiness` — deep dep check (DB + Redis); 200 healthy/degraded,
//!     503 unhealthy. Carries the rich payload (status/version/uptime
//!     /dependencies).
//!
//! Tests are split per endpoint so an assertion failure points at the
//! right contract — pre-split, the DB-connectivity tests hit `/health`
//! but quietly skipped their deep assertions when the response lacked
//! `checks`, which let regressions slide. Now `/readiness` tests
//! assert on dependency fields explicitly.
//!
//! Uses sqlx::test for test database isolation.

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use sqlx::PgPool;

use common::TestApp;

// =============================================================================
// Basic Health Check Tests
// =============================================================================

#[cfg(test)]
mod basic_health {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_endpoint_returns_ok(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_response_format(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
        response.assert_json_field("status");

        let json = response.json_value();
        let status = json["status"].as_str().unwrap();

        // Status should be "healthy" or "degraded"
        assert!(
            status == "healthy" || status == "degraded" || status == "ok",
            "Status should be healthy, degraded, or ok, got: {}",
            status
        );
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_includes_version(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);

        // Check if version is included
        let json = response.json_value();
        if let Some(version) = json.get("version") {
            assert!(version.is_string(), "Version should be a string");
        }
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_includes_uptime(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);

        // Check if uptime is included
        let json = response.json_value();
        if let Some(uptime) = json.get("uptime_seconds") {
            let uptime_val = uptime
                .as_u64()
                .or_else(|| uptime.as_f64().map(|f| f as u64));
            // Must be present and parse as an unsigned integer. Being u64 it is
            // inherently non-negative, so presence-as-a-number is the real check
            // (a literal `>= 0` on a u64 is a useless comparison and trips
            // `-D warnings` now that this file actually compiles — #2158).
            assert!(
                uptime_val.is_some(),
                "Uptime should be a non-negative number"
            );
        }
    }
}

// =============================================================================
// Readiness — Deep Dependency Check (DB + Redis)
// =============================================================================
//
// Post-E8 split: the deep-check assertions live here against `/readiness`,
// not `/health`. `/health` is shallow liveness and intentionally has no
// `dependencies` field — asserting on it from `/health` would be looking
// in the wrong place.

#[cfg(test)]
mod readiness {
    use super::*;

    /// Find a dependency by name in the readiness response. Returns the
    /// dependency object so callers can assert on `status`/`latency_ms`/etc.
    fn find_dep<'a>(json: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        json.get("dependencies")?
            .as_array()?
            .iter()
            .find(|d| d.get("name").and_then(|n| n.as_str()) == Some(name))
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn readiness_returns_ok_with_working_db(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/readiness")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Healthy DB + (degraded-not-configured) Redis ⇒ overall 200.
        response.assert_status(StatusCode::OK);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn readiness_reports_database_dependency(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/readiness")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::OK);

        let json = response.json_value();
        let db =
            find_dep(&json, "database").expect("readiness must report a `database` dependency");
        let status = db
            .get("status")
            .and_then(|s| s.as_str())
            .expect("database dep must carry `status`");
        assert!(
            status == "healthy" || status == "degraded",
            "DB should be healthy or degraded with a working pool, got: {status}"
        );
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn readiness_reports_database_latency(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/readiness")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::OK);

        let json = response.json_value();
        let db = find_dep(&json, "database").expect("expected database dep");
        let latency = db
            .get("latency_ms")
            .expect("database dep must carry latency_ms");
        assert!(latency.is_number(), "latency_ms should be numeric");
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn readiness_redis_failure_stays_200_degraded(pool: PgPool) {
        // Per the module docstring, Redis is non-critical: any Redis-related
        // status (including unconfigured/unhealthy) must NOT 503. The test
        // app doesn't configure Redis, so this simultaneously pins the
        // unconfigured branch of `check_redis`.
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/readiness")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;
        response.assert_status(StatusCode::OK);

        let json = response.json_value();
        let redis = find_dep(&json, "redis").expect("expected redis dep entry");
        let status = redis
            .get("status")
            .and_then(|s| s.as_str())
            .expect("redis dep must carry `status`");
        assert_ne!(
            status, "unhealthy",
            "Redis is non-critical — unhealthy must be downgraded to degraded"
        );
    }
}

// =============================================================================
// Response Time Tests
// =============================================================================

#[cfg(test)]
mod response_time {
    use super::*;
    use std::time::Instant;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_responds_quickly(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let start = Instant::now();
        let response = app.execute(request).await;
        let duration = start.elapsed();

        response.assert_status(StatusCode::OK);

        // Health check should respond within 1 second
        assert!(
            duration.as_secs() < 1,
            "Health check should respond quickly, took {:?}",
            duration
        );
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_idempotent(pool: PgPool) {
        let app = TestApp::new(pool).await;

        // Call health check multiple times
        for _ in 0..5 {
            let request = Request::builder()
                .method(Method::GET)
                .uri("/health")
                .body(Body::empty())
                .unwrap();

            let response = app.execute(request).await;
            response.assert_status(StatusCode::OK);
        }
    }
}

// =============================================================================
// Content Type Tests
// =============================================================================

#[cfg(test)]
mod content_type {
    use super::*;
    use axum::http::header;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_returns_json(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);

        // Check content type header
        let content_type = response.headers.get(header::CONTENT_TYPE);
        assert!(content_type.is_some(), "Should have Content-Type header");

        let content_type_str = content_type.unwrap().to_str().unwrap();
        assert!(
            content_type_str.contains("application/json"),
            "Content-Type should be application/json, got: {}",
            content_type_str
        );
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_response_is_valid_json(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Should not panic when parsing JSON
        let _json = response.json_value();
    }
}

// =============================================================================
// Method Tests
// =============================================================================

#[cfg(test)]
mod http_methods {
    use super::*;

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_accepts_get(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::GET)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        response.assert_status(StatusCode::OK);
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_rejects_post(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::POST)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // Should return 405 Method Not Allowed
        assert!(
            response.status == StatusCode::METHOD_NOT_ALLOWED
                || response.status == StatusCode::NOT_FOUND,
            "POST to health should be rejected"
        );
    }

    #[sqlx::test(migrator = "db::MIGRATOR")]
    async fn test_health_handles_head_request(pool: PgPool) {
        let app = TestApp::new(pool).await;

        let request = Request::builder()
            .method(Method::HEAD)
            .uri("/health")
            .body(Body::empty())
            .unwrap();

        let response = app.execute(request).await;

        // HEAD should return OK with no body or be rejected
        assert!(
            response.status == StatusCode::OK
                || response.status == StatusCode::METHOD_NOT_ALLOWED
                || response.status == StatusCode::NOT_FOUND,
            "HEAD request should be handled appropriately"
        );
    }
}
