//! Regression tests for the cross-tenant IDOR fix on the fault
//! read-and-mutate-by-id paths (issue #770).
//!
//! Audit history: `get_fault` extracted a verified `RequestPrincipal` but then
//! called `fault_repo.find_by_id_with_details(id)` — a query on the raw pool
//! with NO `organization_id` predicate and NOT running under an
//! `RlsConnection`. A caller in org B could therefore read org A's fault
//! (reporter PII, building address, timeline + attachment counts) by guessing
//! or enumerating the fault UUID. The legacy pool-based mutate-by-id handlers
//! (assign/resolve/confirm/reopen, comments, attachments) shared the same gap.
//!
//! The fix adds `_for_org` repository variants that thread the principal's
//! `effective_org` into the SQL WHERE clause (the same idiom the violations and
//! AI/LLM routers already use, see `ai_llm_cross_tenant_idor_tests.rs`).
//!
//! Why repo-layer and not HTTP-layer: `TestApp` mounts the router WITHOUT
//! `host_tenant_middleware`, so `RequestPrincipal` can never resolve an
//! `effective_org` in tests. The IDOR fix itself lives in the repository's
//! WHERE clause, so the security contract is asserted there directly: the
//! scoped method must NOT return another org's row even though the raw
//! (pre-fix) query would. Each test also runs the unscoped lookup to
//! demonstrate the leak the `_for_org` variant closes.

#![allow(dead_code)]

use db::repositories::FaultRepository;
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
    .bind(format!("FaultIDOR Org {slug}"))
    .bind(format!("fault-idor-org-{slug}"))
    .bind(format!("{slug}@fault-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'FaultIDOR User', 'active', NOW())
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
        VALUES ($1, 'FaultIDOR Building', 'Test Street 1', 'Test City', '12345', 'SK', 'active')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_fault(pool: &PgPool, org_id: Uuid) -> Uuid {
    let user_id = seed_user(pool, &format!("reporter-{org_id}@fault-idor.test")).await;
    let building_id = seed_building(pool, org_id).await;

    // Insert directly with explicit enum casts. (The production `create_rls`
    // path relies on a `query_as` bind that decodes fine at runtime but the
    // raw text-bind here needs the cast to satisfy the enum-typed columns.)
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO faults (
            organization_id, building_id, reporter_id,
            title, description, location_description, category, priority
        )
        VALUES (
            $1, $2, $3,
            'Leaking pipe', 'Water under the kitchen sink', 'Kitchen',
            'plumbing'::fault_category, 'high'::fault_priority
        )
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(building_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed fault")
}

// ---------------------------------------------------------------------------
// (1) get_fault path: find_by_id_with_details_for_org is tenant-scoped.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_by_id_with_details_for_org_blocks_cross_tenant_read(pool: PgPool) {
    let repo = FaultRepository::new(pool.clone());

    let org_a = seed_org(&pool, "details-a").await;
    let org_b = seed_org(&pool, "details-b").await;
    let fault_in_a = seed_fault(&pool, org_a).await;

    // Same-org read SUCCEEDS — proves the org predicate does not over-filter.
    let same_org = repo
        .find_by_id_with_details_for_org(fault_in_a, org_a)
        .await
        .expect("query ok");
    assert!(
        same_org.is_some(),
        "org A must be able to read its own fault details"
    );

    // Cross-org read returns None (→ 404). The IDOR is blocked.
    let cross_org = repo
        .find_by_id_with_details_for_org(fault_in_a, org_b)
        .await
        .expect("query ok");
    assert!(
        cross_org.is_none(),
        "org B must NOT be able to read org A's fault details"
    );

    // Sanity: the unscoped query (the vulnerable pre-fix `find_by_id_with_details`
    // path) leaks the row regardless of org.
    let leaked = repo
        .find_by_id_with_details(fault_in_a)
        .await
        .expect("query ok");
    assert!(
        leaked.is_some(),
        "sanity: the unscoped query (pre-fix path) does leak the fault"
    );
}

// ---------------------------------------------------------------------------
// (2) mutate-by-id paths: find_by_id_for_org guards assign/resolve/confirm/
//     reopen/comments/attachments before the pool-based mutation runs.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_by_id_for_org_blocks_cross_tenant_mutate(pool: PgPool) {
    let repo = FaultRepository::new(pool.clone());

    let org_a = seed_org(&pool, "mutate-a").await;
    let org_b = seed_org(&pool, "mutate-b").await;
    let fault_in_a = seed_fault(&pool, org_a).await;

    // Same-org guard SUCCEEDS — the action proceeds for the owning org.
    let same_org = repo
        .find_by_id_for_org(fault_in_a, org_a)
        .await
        .expect("query ok");
    assert!(
        same_org.is_some(),
        "org A must pass the org-scope guard for its own fault"
    );

    // Cross-org guard returns None — the handler maps this to 404 and the
    // assign/resolve/confirm/reopen/comment/attachment mutation never runs.
    let cross_org = repo
        .find_by_id_for_org(fault_in_a, org_b)
        .await
        .expect("query ok");
    assert!(
        cross_org.is_none(),
        "org B must NOT pass the org-scope guard for org A's fault"
    );

    // A fully random id is also rejected for the owning org.
    let missing = repo
        .find_by_id_for_org(Uuid::new_v4(), org_a)
        .await
        .expect("query ok");
    assert!(missing.is_none(), "unknown fault id must resolve to None");
}
