//! Axum extractors and request-data helpers for Reality Server.
//!
//! Authentication itself goes through `api_core::extractors::RequestPrincipal`
//! (and its optional sibling). The only custom helper that lives here is
//! the session-token extractor used by the logout flow — see
//! [`auth`] for the rationale.

pub mod auth;
