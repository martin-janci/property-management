//! Regression tests: IoT sensor endpoints must require authentication
//! (closes #817, #824, #845).
//!
//! 16 of the 22 handlers under `/api/v1/iot/sensors` had no auth extractor and
//! served/mutated sensor data, readings, thresholds, alerts, correlations and
//! templates anonymously. The fix adds `RequestPrincipal` + `require_tenant_id`
//! to each, so an unauthenticated request is rejected at the extractor (401)
//! before reaching the handler.
//!
//! `RequestPrincipal` rejects a request with no `Authorization` header with
//! 401, so these tests need no DB seeding — they assert the auth gate fires.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use sqlx::PgPool;

use common::TestApp;

fn req(method: Method, uri: &str, body: Option<&str>) -> Request<Body> {
    let b = Request::builder().method(method).uri(uri);
    match body {
        Some(j) => b
            .header(axum::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(j.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

/// Every previously-unauthenticated IoT endpoint must reject an anonymous
/// request with 401. Each entry is (method, path, optional-json-body).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn iot_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let sid = "00000000-0000-0000-0000-000000000001";
    let tid = "00000000-0000-0000-0000-000000000002";
    let cid = "00000000-0000-0000-0000-000000000003";
    let aid = "00000000-0000-0000-0000-000000000004";
    let tpl = "00000000-0000-0000-0000-000000000005";
    let base = "/api/v1/iot/sensors";

    let cases: Vec<(Method, String, Option<&str>)> = vec![
        (Method::POST, base.to_string(), Some(r#"{"name":"x"}"#)),
        (Method::GET, format!("{base}/{sid}"), None),
        (Method::PUT, format!("{base}/{sid}"), Some(r#"{}"#)),
        (Method::DELETE, format!("{base}/{sid}"), None),
        (Method::GET, format!("{base}/{sid}/readings"), None),
        (
            Method::POST,
            format!("{base}/{sid}/readings"),
            Some(r#"{}"#),
        ),
        (
            Method::POST,
            format!("{base}/{sid}/readings/batch"),
            Some(r#"{"readings":[]}"#),
        ),
        (
            Method::GET,
            format!("{base}/{sid}/readings/aggregated"),
            None,
        ),
        (Method::GET, format!("{base}/{sid}/thresholds"), None),
        (
            Method::POST,
            format!("{base}/{sid}/thresholds"),
            Some(r#"{}"#),
        ),
        (
            Method::PUT,
            format!("{base}/thresholds/{tid}"),
            Some(r#"{}"#),
        ),
        (Method::DELETE, format!("{base}/thresholds/{tid}"), None),
        (Method::POST, format!("{base}/alerts/{aid}/resolve"), None),
        (Method::GET, format!("{base}/{sid}/correlations"), None),
        (Method::DELETE, format!("{base}/correlations/{cid}"), None),
        (Method::POST, format!("{base}/templates/{tpl}/apply"), None),
    ];

    for (method, uri, body) in cases {
        let resp = app.execute(req(method.clone(), &uri, body)).await;
        assert_eq!(
            resp.status,
            StatusCode::UNAUTHORIZED,
            "{method} {uri} must require auth (401), got {}",
            resp.status
        );
    }
}
