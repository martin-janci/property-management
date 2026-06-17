//! Canonical tenant-aware request helpers shared by the `db` crate's RLS
//! integration tests (#1370).
//!
//! Why this module exists
//! ----------------------
//! Roughly two dozen RLS test files under `crates/db/tests/` each defined their
//! own byte-identical inline `set_ctx` helper plus a near-identical `seed_org`
//! that differed only in the cosmetic org name / email prefix. The drift risk is
//! real: when the GUC contract of `set_request_context($1,$2,$3)` changes, every
//! private copy has to be found and updated by hand. This module hoists the
//! canonical versions so RLS tests can `mod common;` + `use common::*;` instead
//! of copy-pasting.
//!
//! `set_ctx` deliberately delegates to the crate-public
//! [`db::set_request_context`] (the single source of truth for the GUC call) and
//! only adds the panic-on-error ergonomics tests want. Keep new RLS tests on
//! these helpers rather than re-introducing private copies.
//!
//! NOTE: `tests/common/mod.rs` is a module, not a test target. Each helper is
//! `#[allow(dead_code)]` because individual test files use only a subset, and
//! Cargo compiles the module once per test binary.

#![allow(dead_code)]

use sqlx::PgPool;
use uuid::Uuid;

/// Set the RLS request context for the given connection/pool.
///
/// Thin, panic-on-error wrapper over [`db::set_request_context`] — the canonical
/// implementation of the `SELECT set_request_context($1, $2, $3)` GUC call. Use
/// this in tests instead of re-binding the query by hand so the GUC contract
/// lives in exactly one place.
pub async fn set_ctx(
    pool: &PgPool,
    org_id: Option<Uuid>,
    user_id: Option<Uuid>,
    is_super_admin: bool,
) {
    db::set_request_context(pool, org_id, user_id, is_super_admin)
        .await
        .expect("set_request_context");
}

/// Seed an `active` organization and return its id.
///
/// Name and contact email are derived deterministically from `slug`, so two test
/// files seeding the same slug get equivalent rows without colliding on the
/// unique `slug` constraint across tests (each `#[sqlx::test]` runs in its own
/// database).
pub async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING id
        "#,
    )
    .bind(format!("Org {slug}"))
    .bind(slug)
    .bind(format!("{slug}@orgs.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}
