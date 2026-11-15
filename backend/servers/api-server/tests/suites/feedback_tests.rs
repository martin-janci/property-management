//! General in-app feedback endpoint integration tests (#3544).
//!
//! The mobile `FeedbackManager` POSTs general feedback to
//! `POST /api/v1/feedback`. Before #3544 no backend server mounted that route,
//! so every mobile submission returned **404** regardless of auth. These tests
//! lock in that the route is now mounted, authenticated, and persists the
//! submission.
//!
//! IG3 regression proof: `test_submit_feedback_without_auth_is_404_before_fix`
//! asserts the route rejects an unauthenticated POST with **401**. On `dev`
//! (route unmounted) the same request returns **404**, so the assertion fails
//! before the fix and passes with it.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;

use crate::common::{create_authenticated_user, TestApp, TestUser};

fn valid_feedback_body() -> serde_json::Value {
    serde_json::json!({
        "type": "bug",
        "title": "Crash on dashboard",
        "description": "The app crashes when I open the dashboard after login.",
        "email": "reporter@example.com",
        "deviceInfo": {
            "platform": "ios",
            "osVersion": "17.2",
            "deviceModel": "iPhone",
            "appVersion": "1.0.0",
            "buildNumber": "42",
            "locale": "en-US",
            "timezone": "Europe/Bratislava"
        },
        "appContext": {
            "currentScreen": "Dashboard",
            "timestamp": "2026-11-15T10:00:00Z",
            "sessionDuration": 120
        }
    })
}

/// IG3: route must be mounted. Without auth it returns 401 (NOT 404).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_submit_feedback_without_auth_is_404_before_fix(pool: PgPool) {
    let app = TestApp::new(pool).await;

    let request = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/feedback")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(valid_feedback_body().to_string()))
        .unwrap();

    let response = app.execute(request).await;

    assert_ne!(
        response.status,
        StatusCode::NOT_FOUND,
        "POST /api/v1/feedback must be a mounted route, not 404 (#3544)"
    );
    assert_eq!(
        response.status,
        StatusCode::UNAUTHORIZED,
        "POST /api/v1/feedback must reject unauthenticated requests with 401, got {}",
        response.status
    );
}

/// Authenticated submission is persisted and returns the new record id.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_submit_feedback_authenticated_persists_and_returns_id(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (access_token, _refresh) = create_authenticated_user(&app, &user).await;

    let response = app
        .post("/api/v1/feedback")
        .bearer(&access_token)
        .json(valid_feedback_body())
        .send()
        .await;

    assert_eq!(
        response.status,
        StatusCode::CREATED,
        "authenticated feedback submission should return 201, got {} ({})",
        response.status,
        response.text()
    );

    let body = response.json_value();
    assert!(
        body.get("id").and_then(|v| v.as_str()).is_some(),
        "response must carry the persisted feedback id, got {body}"
    );
}

/// An unknown feedback `type` is a client bug and must be rejected (400), not
/// stored silently.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_submit_feedback_rejects_unknown_type(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (access_token, _refresh) = create_authenticated_user(&app, &user).await;

    let mut body = valid_feedback_body();
    body["type"] = serde_json::json!("not-a-real-type");

    let response = app
        .post("/api/v1/feedback")
        .bearer(&access_token)
        .json(body)
        .send()
        .await;

    assert_eq!(
        response.status,
        StatusCode::BAD_REQUEST,
        "unknown feedback type should be rejected with 400, got {}",
        response.status
    );
}
