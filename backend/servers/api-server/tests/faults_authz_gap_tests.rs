//! Regression tests for the fault backend gap-sweep (issue #970).
//!
//! Covers two distinct gaps that PR #989 closed in
//! `routes/faults.rs` + `repositories/fault.rs`:
//!
//! (#970.2) Idempotent offline create. `create_fault` now looks up an
//!   existing fault by `idempotency_key` on the RLS-scoped connection BEFORE
//!   inserting, so an offline retry returns the existing row (200) instead of
//!   letting the global UNIQUE index 500 on the duplicate insert. The lookup
//!   uses the org-scoped `find_by_idempotency_key_rls` (NOT the unscoped
//!   pool-based `find_by_idempotency_key`, which would risk a cross-tenant
//!   key collision). This file asserts the scoped finder's contract.
//!
//! (#970.4) Authz guards on `update_fault` / `get_ai_suggestion`. A fault may
//!   be edited only by the reporter who filed it or by a manager in the
//!   tenant; `get_ai_suggestion` is manager-only. Both branches pivot on
//!   `MembershipRepository::is_manager_in_org`, so a non-reporter
//!   non-manager same-org member must NOT clear the manager gate (→ 403).
//!
//! Why repo/helper-layer and not HTTP-layer: `TestApp` mounts the fault
//! router WITHOUT `host_tenant_middleware`, so a `RequestPrincipal` can never
//! resolve an `effective_org` in tests — every handler would short-circuit at
//! `require_tenant_id` (403 TENANT_REQUIRED) before the gap logic runs. The
//! security/idempotency contracts therefore live in the repository methods,
//! which is exactly where the sibling `faults_cross_org_idor_tests.rs` asserts
//! them. Each test exercises the real query path against a migrated database.

#[allow(dead_code)]
mod common;

use common::seed_membership;
use db::repositories::{FaultRepository, MembershipRepository};
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("FaultAuthz Org {slug}"))
    .bind(format!("fault-authz-org-{slug}"))
    .bind(format!("{slug}@fault-authz.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    // `users` has a single `name` column (no first_name/last_name — see #1008).
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'FaultAuthz User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
        VALUES ($1, 'FaultAuthz Building', 'Test Street 1', 'Test City', '12345', 'SK', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed a fault carrying an `idempotency_key`, returning its id.
async fn seed_fault_with_key(pool: &PgPool, org_id: Uuid, key: &str) -> Uuid {
    let user_id = seed_user(pool, &format!("reporter-{key}@fault-authz.test")).await;
    let building_id = seed_building(pool, org_id).await;

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO faults (
            organization_id, building_id, reporter_id,
            title, description, location_description,
            category, priority, idempotency_key
        )
        VALUES (
            $1, $2, $3,
            'Leaking pipe', 'Water under the kitchen sink', 'Kitchen',
            'plumbing'::fault_category, 'high'::fault_priority, $4
        )
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(user_id)
    .bind(key)
    .fetch_one(pool)
    .await
    .expect("seed fault with idempotency key")
}

// ---------------------------------------------------------------------------
// (#970.2) Idempotent offline create: find_by_idempotency_key_rls lookup.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_by_idempotency_key_rls_returns_existing_fault(pool: PgPool) {
    let repo = FaultRepository::new(pool.clone());
    let org = seed_org(&pool, "idem").await;
    let key = "offline-create-retry-0001";
    let fault_id = seed_fault_with_key(&pool, org, key).await;

    // A retry with the same key must resolve to the already-created fault, so
    // `create_fault` returns it (200) instead of 500-ing on the UNIQUE index.
    let found = repo
        .find_by_idempotency_key_rls(&pool, key)
        .await
        .expect("query ok");
    assert!(
        found.is_some(),
        "the scoped finder must return the existing fault for a known key"
    );
    assert_eq!(
        found.unwrap().id,
        fault_id,
        "must return the exact fault that was created with this key"
    );

    // An unknown key resolves to None, so a first-time create proceeds to INSERT.
    let missing = repo
        .find_by_idempotency_key_rls(&pool, "never-seen-key")
        .await
        .expect("query ok");
    assert!(
        missing.is_none(),
        "an unknown idempotency key must resolve to None"
    );
}

// ---------------------------------------------------------------------------
// (#970.4) Authz pivot: a plain same-org member is NOT a manager, so the
//          update_fault non-reporter branch and get_ai_suggestion gate 403.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_reporter_non_manager_member_fails_manager_gate(pool: PgPool) {
    let repo = MembershipRepository::new(pool.clone());
    let org = seed_org(&pool, "authz").await;

    // A same-org member with a non-manager role: this is the caller who is
    // neither the fault's reporter nor a manager. update_fault's fallback
    // (`require_manager`) and get_ai_suggestion's gate both reject them (403).
    let member = seed_user(&pool, "plain-member@fault-authz.test").await;
    seed_membership(&pool, org, member, "member").await;

    let member_is_manager = repo.is_manager_in_org(member, org).await.expect("query ok");
    assert!(
        !member_is_manager,
        "a non-manager same-org member must NOT clear the manager gate (→ 403)"
    );

    // Sanity: a manager-tier member DOES clear the gate, proving the guard
    // does not over-reject (e.g. update_fault by a manager succeeds).
    let manager = seed_user(&pool, "real-manager@fault-authz.test").await;
    seed_membership(&pool, org, manager, "Manager").await;

    let manager_is_manager = repo
        .is_manager_in_org(manager, org)
        .await
        .expect("query ok");
    assert!(
        manager_is_manager,
        "a manager-tier member must clear the manager gate"
    );

    // A user with no membership in this org is likewise rejected.
    let outsider = seed_user(&pool, "outsider@fault-authz.test").await;
    let outsider_is_manager = repo
        .is_manager_in_org(outsider, org)
        .await
        .expect("query ok");
    assert!(
        !outsider_is_manager,
        "a non-member must NOT clear the manager gate"
    );
}
