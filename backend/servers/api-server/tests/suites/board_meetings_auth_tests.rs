//! Regression: board-meeting endpoints must require authentication (#849).
//!
//! 32 of the board_meetings handlers (member CRUD, meeting lifecycle, agenda
//! items, motions incl. `cast_vote`, minutes, action items, documents) had no
//! `AuthUser` extractor and served/mutated governance data anonymously. The
//! fix adds the extractor to each, so an unauthenticated request is rejected
//! with 401 before reaching the handler.
//!
//! No DB seeding: `AuthUser` rejects a request with no bearer token at the
//! extractor.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use sqlx::PgPool;

use crate::common::TestApp;

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

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn board_meeting_endpoints_require_auth(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let id = "00000000-0000-0000-0000-000000000001";
    let base = "/api/v1/board-meetings";

    let cases: Vec<(Method, String, Option<&str>)> = vec![
        (Method::GET, format!("{base}/members/{id}"), None),
        (Method::PUT, format!("{base}/members/{id}"), Some("{}")),
        (Method::DELETE, format!("{base}/members/{id}"), None),
        (Method::GET, format!("{base}/members/{id}/attendance"), None),
        (Method::GET, format!("{base}/{id}"), None),
        (Method::PUT, format!("{base}/{id}"), Some("{}")),
        (Method::DELETE, format!("{base}/{id}"), None),
        (Method::POST, format!("{base}/{id}/start"), None),
        (Method::POST, format!("{base}/{id}/end"), None),
        (Method::GET, format!("{base}/motions/{id}"), None),
        (Method::PUT, format!("{base}/motions/{id}"), Some("{}")),
        (
            Method::POST,
            format!("{base}/motions/{id}/start-voting"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/motions/{id}/end-voting"),
            None,
        ),
        (
            Method::POST,
            format!("{base}/motions/{id}/vote"),
            Some("{}"),
        ),
        (Method::DELETE, format!("{base}/motions/{id}"), None),
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
