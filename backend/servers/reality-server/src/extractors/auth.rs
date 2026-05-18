//! Session-token extraction helpers for Reality Server.
//!
//! # Status (Phase 2.5 / E1.3)
//!
//! `RequestPrincipal` (in `api-core`) is the ONLY authentication path used
//! by route handlers in reality-server. It validates the JWT directly
//! against `JWT_SECRET` and re-derives `principal_kind` from the trusted
//! `users` table on every request, so endpoints never need to parse the
//! Authorization header themselves.
//!
//! That leaves exactly one place that still needs the *raw* session token:
//! the logout flow. Logout has to invalidate the underlying session row in
//! `portal_sessions`, which is keyed on a SHA-256 hash of the raw token —
//! `RequestPrincipal` only carries the validated user id, not the token
//! string. So this module exposes two narrow helpers:
//!
//! * [`extract_session_token`] — pulls the token out of the `Authorization:
//!   Bearer …` header or the `portal_session` cookie.
//! * [`extract_session_cookie`] — cookie-only variant (the SSO logout
//!   endpoint requires the cookie path specifically).
//!
//! These were previously duplicated in three places (`extractors::auth`,
//! `routes::users::logout`, `routes::sso`). Consolidating them here removes
//! the duplication and the legacy `AuthenticatedUser` / `OptionalAuth`
//! extractors that the D1.2 sweep made dead code.

use axum::http::HeaderMap;

/// Pull a session token out of the request headers.
///
/// Looks at `Authorization: Bearer …` first; falls back to the
/// `portal_session` cookie. Returns `None` if neither is present, so the
/// caller can choose the right error response (401 vs. silent skip).
pub fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            return Some(token.to_string());
        }
    }

    extract_session_cookie(headers)
}

/// Cookie-only variant of [`extract_session_token`].
///
/// Used by the SSO logout endpoint, which is invoked by the browser after
/// the cookie is set on the SSO callback — there is no Authorization header
/// to fall back to in that flow.
pub fn extract_session_cookie(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies
                .split(';')
                .find(|c| c.trim().starts_with("portal_session="))
                .map(|c| {
                    c.trim()
                        .strip_prefix("portal_session=")
                        .unwrap()
                        .to_string()
                })
        })
}
