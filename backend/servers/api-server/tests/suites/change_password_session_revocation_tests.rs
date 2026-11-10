//! Regression suite for #3352 — authenticated password change must revoke the
//! user's OTHER sessions.
//!
//! Lives beside `auth_tests.rs` (both are registered in `suite_2`) and extends
//! its `mod change_password` coverage. A password change is a security event:
//! after a successful `POST /api/v1/auth/password/change` every pre-existing
//! refresh token / session on OTHER devices must be revoked (the usual reason a
//! signed-in user changes their password is that someone else holds a session),
//! while the caller's OWN current session is preserved so they are not logged
//! out of the tab they changed their password from.
//!
//! Before the fix `change_password` updated the password hash but revoked
//! nothing, so `refresh2` below would still return 200 — this suite fails on
//! that pre-fix handler and passes once the revocation is added.
//!
//! Uses sqlx::test for test database isolation.

use serde_json::json;
use sqlx::PgPool;

use crate::common::{create_authenticated_user, TestApp, TestUser};

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn change_password_revokes_other_sessions_keeps_current(pool: PgPool) {
    use axum::http::StatusCode;

    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();

    // Session 1 = the caller; session 2 = another device logged into the same
    // account.
    let (access1, refresh1) = create_authenticated_user(&app, &user).await;
    let login2 = app
        .post("/api/v1/auth/login")
        .json(user.login_body())
        .build();
    let login2_resp = app.execute(login2).await;
    login2_resp.assert_status(StatusCode::OK);
    let refresh2 = login2_resp.json_value()["refreshToken"]
        .as_str()
        .expect("refreshToken")
        .to_string();

    // Caller changes their password as a cookie-based ppt-web client: the
    // refresh token lives in the HttpOnly cookie, so the handler can resolve
    // (and preserve) the caller's current session via `resolve_current_session_id`.
    let change = app
        .post("/api/v1/auth/password/change")
        .bearer(&access1)
        .header("Cookie", &format!("refresh_token={}", refresh1))
        .json(json!({
            "currentPassword": user.password,
            "newPassword": "BrandNewPassword456!"
        }))
        .build();
    app.execute(change).await.assert_status(StatusCode::OK);

    // The caller's own session survives: their refresh token still works.
    let refresh_ok = app
        .post("/api/v1/auth/refresh")
        .header("Cookie", &format!("refresh_token={}", refresh1))
        .json(json!({ "refreshToken": "" }))
        .build();
    assert_eq!(
        app.execute(refresh_ok).await.status,
        StatusCode::OK,
        "caller's own session must survive the password change"
    );

    // The other device is signed out: its refresh token is now rejected.
    let refresh_bad = app
        .post("/api/v1/auth/refresh")
        .header("Cookie", &format!("refresh_token={}", refresh2))
        .json(json!({ "refreshToken": "" }))
        .build();
    assert_eq!(
        app.execute(refresh_bad).await.status,
        StatusCode::UNAUTHORIZED,
        "all other sessions must be revoked after a password change"
    );
}
