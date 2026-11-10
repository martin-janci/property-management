//! Guard tests for the unified router (issue #867).
//!
//! Background: the production binary (`src/main.rs`) used to hand-maintain its
//! own chain of `.nest(...)` calls, separate from `lib.rs::create_router`
//! (which the integration tests exercise via `TestApp`). The two drifted —
//! PR #836 had to retro-fit FIVE routers (`push-tokens`, `mfa/recovery-codes`,
//! `pricing`, `property-valuations`, `data-residency`) that existed only in the
//! test-side router and were therefore unreachable in production.
//!
//! These tests assert that the route table is sourced from a SINGLE builder,
//! `api_server::route_table()`, so the two paths cannot silently diverge again.
//! They are pure source/static checks: no database, no async runtime, so they
//! run in every `cargo test` invocation regardless of DB availability.

#![allow(dead_code)]

use std::path::PathBuf;

fn read_src(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(file);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()))
}

/// `main.rs` (the production binary) MUST build its application router from the
/// shared `route_table()` builder, not from a hand-listed chain of API
/// `.nest(...)` calls.
#[test]
fn main_builds_from_route_table() {
    let main_src = read_src("main.rs");

    assert!(
        main_src.contains("route_table()"),
        "main.rs must build its router from the shared `route_table()` \
         (single source of truth, #867)."
    );
}

/// The production binary MUST NOT hand-list the bulk of the API surface.
///
/// A handful of `.nest(...)` calls are legitimately production-only and stay in
/// `main.rs` (the Phase-3 host-routing endpoints: `/tenant-config`,
/// `/admin/tenants/...`, `/internal/caddy-ask`). Everything under `/api/v1`
/// belongs in `route_table()`. This guard fails if anyone re-introduces an
/// `/api/v1` nest in `main.rs`, which is exactly how the #836 divergence crept
/// in. The threshold is generous (any `/api/v1` nest trips it) precisely
/// because a single re-added API route is the failure mode we are guarding
/// against.
#[test]
fn main_does_not_hand_list_api_routes() {
    let main_src = read_src("main.rs");

    let offenders: Vec<&str> = main_src
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            // A route registration line targeting the public API namespace.
            (t.starts_with(".nest(") || t.starts_with(".route(")) && t.contains("/api/v1")
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "main.rs must not hand-list /api/v1 routes — they belong in \
         `route_table()` (single source of truth, #867). Offending lines:\n{}",
        offenders.join("\n")
    );
}

/// `route_table()` is the single source of truth, so the routers that PR #836
/// had to manually back-fill into the production binary MUST be present in the
/// shared builder. This locks in the #867 fix: if any of these is dropped from
/// `route_table()`, this test fails before the divergence can ship.
#[test]
fn route_table_contains_the_previously_diverged_routers() {
    let lib_src = read_src("lib.rs");

    // Pin the `route_table()` function body so we assert on the SHARED builder,
    // not on `create_router`/`main.rs` happening to mention a path elsewhere.
    let start = lib_src
        .find("pub fn route_table()")
        .expect("route_table() must exist in lib.rs (#867)");
    let after = &lib_src[start..];
    let end = after
        .find("\npub fn create_router")
        .expect("create_router must follow route_table in lib.rs");
    let table = &after[..end];

    // The five routers PR #836 had to retro-fit, plus the routers that were
    // previously main-only (multi-currency, webhooks/portals, feature-packages,
    // ecosystem, portfolio-performance) — all must live in the shared builder.
    let required = [
        "routes::push_tokens::router()",
        "routes::mfa::recovery_codes_router()",
        "routes::market_pricing::router()",
        "routes::property_valuation::router()",
        "routes::data_residency::router()",
        "routes::multi_currency::router()",
        "routes::portal_webhooks::router()",
        "routes::feature_packages::router()",
        "routes::api_ecosystem::router()",
        "routes::portfolio_performance::router()",
    ];

    let missing: Vec<&str> = required
        .iter()
        .filter(|needle| !table.contains(**needle))
        .copied()
        .collect();

    assert!(
        missing.is_empty(),
        "route_table() is missing routers that must be in the single source of \
         truth (#867): {missing:?}"
    );
}

/// Middleware parity guard (#954 / PR #963 follow-up).
///
/// The route table is shared via `route_table()`, but middleware layers are
/// applied separately in two places: the production binary
/// (`main.rs::apply_middleware`) and the test-side `lib.rs::create_router`.
/// Security-critical edge middleware that lives in only one of the two paths is
/// the same drift failure mode #867 fixed for routes — the `security_headers`
/// layer originally shipped in `main.rs` only, so integration tests ran without
/// it and the wiring was never exercised.
///
/// This is a pure source check (no DB / no runtime) so it runs in every
/// `cargo test`, complementing the DB-backed runtime assertions in
/// `security_headers_tests.rs`.
#[test]
fn security_headers_layer_is_wired_into_both_router_paths() {
    let main_src = read_src("main.rs");
    let lib_src = read_src("lib.rs");

    assert!(
        main_src.contains("api_core::middleware::security_headers"),
        "main.rs (production binary) must apply the `security_headers` layer (#954)."
    );

    // Pin the assertion to the `create_router` body so we are checking the
    // actual served chain, not a stray mention elsewhere in lib.rs.
    let start = lib_src
        .find("pub fn create_router")
        .expect("create_router must exist in lib.rs");
    let create_router_body = &lib_src[start..];

    assert!(
        create_router_body.contains("api_core::middleware::security_headers"),
        "lib.rs::create_router must apply the `security_headers` layer so the \
         integration-test path matches production (#954). It previously lived \
         only in main.rs, leaving every integration test running without \
         security headers."
    );
}

/// CORS-policy parity guard (code-review finding: api-core CORS prod/test
/// divergence).
///
/// Background: the CORS *policy* (allowed methods / headers / credentials /
/// max-age) was defined in TWO places — `main.rs::cors_layer` (the production
/// binary) and an inline `CorsLayer::new()...` chain in
/// `lib.rs::create_router` (the path every integration test exercises via
/// `TestApp`). The two drifted: the test-side policy allowed an extra
/// `x-tenant-id` request header that the production policy did NOT, so the
/// integration tests never exercised the real production CORS policy. This is
/// the same single-source-of-truth drift #867 fixed for the route table and
/// #954 for the security-headers layer.
///
/// Both paths now build their CORS layer from ONE shared builder,
/// `api_server::cors_layer(origins)` (origins stay path-specific — the binary
/// reads `CORS_ALLOWED_ORIGINS`, the test path uses the defaults — but the
/// policy is identical). This guard fails the build if either path
/// re-introduces an inline `CorsLayer`, or if the shared policy re-admits the
/// divergent `x-tenant-id` header. Pure source check — no DB, no runtime — so
/// it runs in every `cargo test`.
#[test]
fn cors_policy_is_single_source_of_truth() {
    let main_src = read_src("main.rs");
    let lib_src = read_src("lib.rs");

    // The test-side router MUST delegate to the shared builder, not inline its
    // own policy.
    let cr_start = lib_src
        .find("pub fn create_router")
        .expect("create_router must exist in lib.rs");
    let create_router_body = &lib_src[cr_start..];
    assert!(
        !create_router_body.contains("CorsLayer::new()"),
        "lib.rs::create_router must NOT inline a `CorsLayer` — build CORS from \
         the shared `cors_layer()` so the integration-test path exercises the \
         SAME policy as production. The inline policy diverged (it allowed an \
         extra `x-tenant-id` request header production does not)."
    );
    assert!(
        create_router_body.contains("cors_layer("),
        "lib.rs::create_router must build CORS from the shared `cors_layer()` \
         builder (single source of truth)."
    );

    // The production binary MUST build CORS from the same shared builder.
    assert!(
        !main_src.contains("CorsLayer::new()"),
        "main.rs must NOT inline a `CorsLayer` — build CORS from the shared \
         `api_server::cors_layer()` so it cannot drift from the test path."
    );
    assert!(
        main_src.contains("api_server::cors_layer("),
        "main.rs must build CORS from the shared `api_server::cors_layer()` builder."
    );

    // The shared policy must match production and must NOT re-admit the header
    // that caused the original test/prod divergence. Scope the check to the
    // shared `cors_layer()` builder body (not the whole file — the doc comments
    // legitimately name the header while explaining the regression).
    let cl_start = lib_src
        .find("pub fn cors_layer(")
        .expect("the shared `cors_layer()` builder must exist in lib.rs");
    let after = &lib_src[cl_start..];
    let cl_end = after
        .find("\npub fn ")
        .expect("another `pub fn` must follow `cors_layer()` in lib.rs");
    let cors_layer_body = &after[..cl_end];
    assert!(
        !cors_layer_body.contains("x-tenant-id"),
        "the shared CORS policy must match production and must NOT allow the \
         `x-tenant-id` request header — that extra header was the test-only \
         divergence this guard exists to prevent."
    );
}
