//! Phase 2 (D1.1) — `OptionalRequestPrincipal` extractor branch coverage.
//!
//! `OptionalRequestPrincipal` wraps `RequestPrincipal` for endpoints that
//! accept BOTH authenticated and anonymous traffic. The contract under test:
//!
//!   1. **No `Authorization` header** → `Self(None)`. Anonymous request.
//!   2. **Authorization header present and resolves cleanly** → `Self(Some(_))`.
//!   3. **Authorization header present but JWT invalid OR membership 403** →
//!      propagate the rejection (do NOT silently demote to "anonymous"; that
//!      would mask leak #10/#11 — a stolen-but-revoked token, or a token for
//!      an org the bearer no longer belongs to, must be told "no").
//!
//! These tests exercise branches (1) and (3). Branch (2)'s happy path
//! requires a populated `users` row plus a `MembershipRepository` lookup —
//! that is covered by the DB-backed integration tests under
//! `backend/servers/*/tests/`. Here we pin the *delegation contract* to
//! `RequestPrincipal::from_request_parts` so a future refactor cannot turn
//! a 401/403 into a silent `None`.

use api_core::extractors::tenant::TenantMembershipProvider;
use api_core::extractors::{OptionalRequestPrincipal, RequestPrincipal};
use axum::extract::FromRequestParts;
use axum::http::{Request, StatusCode};
use db::DbPool;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Test plumbing
// ---------------------------------------------------------------------------

/// Stub state that satisfies `TenantMembershipProvider`. The pool is a
/// `connect_lazy` handle pointed at an unreachable address — the JWT-failure
/// branches return BEFORE any query runs, so the pool is never touched.
#[derive(Clone)]
struct StubState {
    pool: DbPool,
}

impl StubState {
    fn new() -> Self {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy("postgres://never-used:never@127.0.0.1:1/never")
            .expect("lazy pool init");
        Self { pool }
    }
}

impl TenantMembershipProvider for StubState {
    fn db_pool(&self) -> &DbPool {
        &self.pool
    }
}

/// Minimal `RequestPrincipal`-shape claim set for token minting in tests.
/// Mirrors `api_core::extractors::principal::PrincipalClaims` field-for-field
/// (the extractor decodes by name, not by struct identity).
#[derive(Debug, Serialize, Deserialize)]
struct TestClaims {
    sub: Uuid,
    iat: i64,
    exp: i64,
    kind: Option<String>,
}

fn parts_for(req: Request<()>) -> axum::http::request::Parts {
    let (parts, _) = req.into_parts();
    parts
}

// ---------------------------------------------------------------------------
// Branch (1): no Authorization header → Self(None)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_auth_header_yields_anonymous_none() {
    // Anonymous SSR / public-listing-detail-without-login is the canonical
    // shape this branch supports. The extractor must NOT call into the DB
    // — it must return `Self(None)` from a header check alone.
    let state = StubState::new();
    let mut parts = parts_for(
        Request::builder()
            .uri("/anything")
            .body(())
            .expect("build req"),
    );

    let result = OptionalRequestPrincipal::from_request_parts(&mut parts, &state).await;

    let opt = result.expect("anonymous request must not error");
    assert!(
        opt.0.is_none(),
        "missing Authorization header must yield None, got Some(_)"
    );
}

// ---------------------------------------------------------------------------
// Branch (3): Authorization present but JWT invalid → propagate 401
// ---------------------------------------------------------------------------

#[tokio::test]
async fn invalid_bearer_token_propagates_401() {
    // Critical security contract: presence of `Authorization` is a
    // *commitment* by the client to "I am authenticated". A garbage token
    // must surface as 401, not be silently demoted to anonymous (which
    // would let an attacker probe the anonymous code path with a junk
    // token to confirm it diverges from the authenticated one).
    //
    // Note: the JWT decoder runs BEFORE any DB lookup, so the stub pool's
    // unreachable address is fine — we never reach a query.
    std::env::set_var(
        "JWT_SECRET",
        "test-secret-min-32-chars-for-jwt-signing-okay",
    );
    let state = StubState::new();

    let mut parts = parts_for(
        Request::builder()
            .uri("/anything")
            .header("Authorization", "Bearer not-a-real-jwt-just-some-garbage")
            .body(())
            .expect("build req"),
    );

    let result = OptionalRequestPrincipal::from_request_parts(&mut parts, &state).await;

    let (status, _msg) = result.expect_err("invalid token must propagate as Err, not Some/None");
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "invalid bearer token must surface as 401 (not silently demoted to None)"
    );
}

#[tokio::test]
async fn malformed_auth_scheme_propagates_401() {
    // A non-Bearer scheme is also a malformed-credentials case. Same
    // rationale as the invalid-token test: do not swallow.
    std::env::set_var(
        "JWT_SECRET",
        "test-secret-min-32-chars-for-jwt-signing-okay",
    );
    let state = StubState::new();

    let mut parts = parts_for(
        Request::builder()
            .uri("/anything")
            .header("Authorization", "Basic dXNlcjpwYXNz")
            .body(())
            .expect("build req"),
    );

    let result = OptionalRequestPrincipal::from_request_parts(&mut parts, &state).await;

    let (status, _msg) = result.expect_err("malformed scheme must propagate as Err");
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "non-Bearer Authorization must surface as 401, not be demoted to None"
    );
}

// ---------------------------------------------------------------------------
// Branch (2) — delegation pin
// ---------------------------------------------------------------------------

/// We cannot exercise the full happy path here (it needs a populated `users`
/// row and a `MembershipRepository::is_active` query) but we CAN pin the
/// "Authorization-present means we delegate to the inner extractor" half of
/// the contract: a syntactically valid JWT signed with the right secret
/// must produce a request-shape that `RequestPrincipal::from_request_parts`
/// would also accept past the JWT-decode stage. We assert this by minting
/// a valid token and observing that the wrapper does NOT short-circuit to
/// `None` (it will try to do the DB lookup; the unreachable lazy pool will
/// surface a 500 — that's the "not None" signal we care about, and it
/// proves we did not silently demote to anonymous).
#[tokio::test]
async fn valid_token_with_dead_db_does_not_demote_to_anonymous() {
    std::env::set_var(
        "JWT_SECRET",
        "test-secret-min-32-chars-for-jwt-signing-okay",
    );
    let state = StubState::new();

    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: Uuid::new_v4(),
        iat: now,
        exp: now + 3600,
        kind: Some("public".to_string()),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(b"test-secret-min-32-chars-for-jwt-signing-okay"),
    )
    .expect("mint test JWT");

    let mut parts = parts_for(
        Request::builder()
            .uri("/anything")
            .header("Authorization", format!("Bearer {token}"))
            .body(())
            .expect("build req"),
    );

    let result = OptionalRequestPrincipal::from_request_parts(&mut parts, &state).await;

    // The contract: with a valid JWT we must NOT see `Ok(Self(None))`.
    // That branch is reserved for the "no Authorization at all" case.
    // Here we expect either Ok(Some(_)) (impossible with a dead pool) or
    // Err(_) from the downstream DB lookup — anything but Ok(None).
    match result {
        Ok(OptionalRequestPrincipal(None)) => panic!(
            "valid Authorization header must NOT be silently demoted to None; \
             this is the leak-#10/#11 defense the wrapper exists to enforce"
        ),
        Ok(OptionalRequestPrincipal(Some(_))) => {
            // Surprising on a dead pool, but contract-correct.
        }
        Err((status, _)) => {
            // The dead pool will yield an INTERNAL_SERVER_ERROR from the
            // principal_kind lookup. The exact code is incidental — what
            // matters is that we propagated rather than swallowed.
            assert_ne!(
                status,
                StatusCode::OK,
                "propagated error must not somehow be 200"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Sanity: the inner `RequestPrincipal` still rejects no-header requests
// with 401. This pins that the wrapper's "None on missing header" behavior
// is genuinely additive, not a refactor of the inner contract.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn inner_request_principal_still_rejects_missing_header() {
    let state = StubState::new();
    let mut parts = parts_for(
        Request::builder()
            .uri("/anything")
            .body(())
            .expect("build req"),
    );

    let result = RequestPrincipal::from_request_parts(&mut parts, &state).await;

    let (status, _) = result.expect_err("inner extractor must still 401 on missing header");
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
