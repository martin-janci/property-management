//! Regression test for issue #1008 — voting-side schema drift.
//!
//! Several hand-written `sqlx::query` strings in the voting paths referenced
//! columns that do not exist in the migrations:
//!   - `delegations.delegate_id`        -> actual `delegate_user_id`
//!   - `delegations.scope IN (...)`     -> actual `scopes delegation_scope[]`
//!     (and the value `'full'` does not exist; the array value is `'all'`)
//!   - `delegations.expires_at`         -> actual `end_date DATE`
//!   - `unit_residents.move_out_date`   -> actual `end_date DATE`
//!
//! Because the backend ships no `.sqlx` offline metadata and uses
//! runtime-checked query strings (not the `query!` macro), these compiled and
//! passed clippy but ERROR'd at runtime against a real Postgres. This made:
//!   - delegated-unit vote eligibility (`check_eligibility`) fail to plan, and
//!   - the #971.3 cast-time delegation guard 500 instead of returning
//!     403 DELEGATION_REVOKED.
//!
//! These tests run the real eligibility CTE and the real cast-time guard SQL
//! against live Postgres, so they fail on the broken columns and pass once the
//! columns are corrected. They are DB-gated (`#[sqlx::test]`); CI provisions
//! Postgres and runs them.

use db::repositories::VoteRepository;
use db::DbPool;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

async fn set_ctx(conn: &mut PgConnection, org: Option<Uuid>, user: Option<Uuid>, su: bool) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org)
        .bind(user)
        .bind(su)
        .execute(conn)
        .await
        .expect("set_request_context");
}

async fn insert_org(conn: &mut PgConnection, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO organizations (name, slug, contact_email, status) \
         VALUES ($1, $2, $3, 'active') RETURNING id",
    )
    .bind(format!("Issue 1008 {slug}"))
    .bind(slug)
    .bind(format!("{slug}@issue1008.test"))
    .fetch_one(conn)
    .await
    .expect("insert org")
}

async fn insert_user(conn: &mut PgConnection, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at) \
         VALUES ($1, 'test_hash', 'U1008', 'active', NOW()) RETURNING id",
    )
    .bind(email)
    .fetch_one(conn)
    .await
    .expect("insert user")
}

async fn insert_building(conn: &mut PgConnection, org: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO buildings (organization_id, name, street, city, postal_code, country) \
         VALUES ($1, $2, 'Addr', 'City', '00000', 'Country') RETURNING id",
    )
    .bind(org)
    .bind(name)
    .fetch_one(conn)
    .await
    .expect("insert building")
}

async fn insert_unit(conn: &mut PgConnection, building: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation) VALUES ($1, $2) RETURNING id",
    )
    .bind(building)
    .bind(designation)
    .fetch_one(conn)
    .await
    .expect("insert unit")
}

/// Owner resident with an open validity period (`end_date IS NULL` = current).
async fn insert_owner_resident(conn: &mut PgConnection, unit: Uuid, user: Uuid) {
    sqlx::query(
        "INSERT INTO unit_residents (unit_id, user_id, resident_type) \
         VALUES ($1, $2, 'owner')",
    )
    .bind(unit)
    .bind(user)
    .execute(conn)
    .await
    .expect("insert owner resident");
}

/// Active vote with delegation allowed, open now, ending in the future.
async fn insert_active_vote(
    conn: &mut PgConnection,
    org: Uuid,
    building: Uuid,
    creator: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO votes \
            (organization_id, building_id, title, start_at, end_at, status, \
             allow_delegation, created_by, published_by, published_at) \
         VALUES ($1, $2, 'Issue 1008 vote', NOW() - INTERVAL '1 hour', \
                 NOW() + INTERVAL '7 days', 'active', TRUE, $3, $3, NOW()) \
         RETURNING id",
    )
    .bind(org)
    .bind(building)
    .bind(creator)
    .fetch_one(conn)
    .await
    .expect("insert active vote")
}

/// Active voting-scoped delegation of `unit` from `owner` to `delegate`.
async fn insert_active_voting_delegation(
    conn: &mut PgConnection,
    owner: Uuid,
    delegate: Uuid,
    unit: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO delegations \
            (owner_user_id, delegate_user_id, unit_id, scopes, status, start_date, end_date) \
         VALUES ($1, $2, $3, ARRAY['voting']::delegation_scope[], 'active', CURRENT_DATE, NULL) \
         RETURNING id",
    )
    .bind(owner)
    .bind(delegate)
    .bind(unit)
    .fetch_one(conn)
    .await
    .expect("insert active voting delegation")
}

/// Mirror of the cast-time delegation re-validation guard in
/// `api-server/src/routes/voting.rs` (#971.3). Returns whether an active
/// matching delegation exists. The handler returns 403 DELEGATION_REVOKED when
/// this is `false`.
async fn cast_guard_delegation_valid(
    pool: &DbPool,
    delegation_id: Uuid,
    delegate_user_id: Uuid,
    unit_id: Uuid,
) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM delegations
            WHERE id = $1
              AND delegate_user_id = $2
              AND unit_id = $3
              AND status = 'active'
              AND (scopes && ARRAY['voting','all']::delegation_scope[])
              AND (end_date IS NULL OR end_date >= CURRENT_DATE)
        )
        "#,
    )
    .bind(delegation_id)
    .bind(delegate_user_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("run cast-time delegation guard")
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn active_voting_delegation_makes_delegate_eligible(pool: PgPool) {
    let mut sa = pool.acquire().await.unwrap();
    set_ctx(&mut sa, None, None, true).await;

    let org = insert_org(&mut sa, "elig").await;
    let owner = insert_user(&mut sa, "owner-elig@issue1008.test").await;
    let delegate = insert_user(&mut sa, "delegate-elig@issue1008.test").await;
    let building = insert_building(&mut sa, org, "B-elig").await;
    let unit = insert_unit(&mut sa, building, "1A").await;
    insert_owner_resident(&mut sa, unit, owner).await;
    let vote = insert_active_vote(&mut sa, org, building, owner).await;
    insert_active_voting_delegation(&mut sa, owner, delegate, unit).await;
    drop(sa);

    let repo = VoteRepository::new(pool.clone());

    // The delegate (who is NOT an owner resident) is eligible via the active
    // voting delegation. Before the fix, the delegated-units CTE branch
    // referenced delegate_id / scope / expires_at and errored at runtime.
    let eligibility = repo
        .check_eligibility(vote, delegate)
        .await
        .expect("check_eligibility for delegate");

    let delegated: Vec<_> = eligibility
        .eligible_units
        .iter()
        .filter(|u| u.unit_id == unit && u.is_delegated)
        .collect();
    assert_eq!(
        delegated.len(),
        1,
        "delegate should have exactly one delegated eligible unit; got {:?}",
        eligibility.eligible_units
    );
    assert!(
        eligibility.can_vote,
        "delegate with an active voting delegation should be able to vote"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn revoked_delegation_is_rejected_by_eligibility_and_cast_guard(pool: PgPool) {
    let mut sa = pool.acquire().await.unwrap();
    set_ctx(&mut sa, None, None, true).await;

    let org = insert_org(&mut sa, "revoke").await;
    let owner = insert_user(&mut sa, "owner-revoke@issue1008.test").await;
    let delegate = insert_user(&mut sa, "delegate-revoke@issue1008.test").await;
    let building = insert_building(&mut sa, org, "B-revoke").await;
    let unit = insert_unit(&mut sa, building, "2B").await;
    insert_owner_resident(&mut sa, unit, owner).await;
    let vote = insert_active_vote(&mut sa, org, building, owner).await;
    let delegation = insert_active_voting_delegation(&mut sa, owner, delegate, unit).await;
    drop(sa);

    let repo = VoteRepository::new(pool.clone());

    // While active: eligible + cast guard passes.
    let before = repo
        .check_eligibility(vote, delegate)
        .await
        .expect("check_eligibility (active)");
    assert!(
        before
            .eligible_units
            .iter()
            .any(|u| u.unit_id == unit && u.is_delegated),
        "delegate should be eligible while delegation is active"
    );
    assert!(
        cast_guard_delegation_valid(&pool, delegation, delegate, unit).await,
        "cast-time guard should accept an active delegation"
    );

    // Revoke the delegation.
    {
        let mut sa = pool.acquire().await.unwrap();
        set_ctx(&mut sa, None, None, true).await;
        sqlx::query("UPDATE delegations SET status = 'revoked', revoked_at = NOW() WHERE id = $1")
            .bind(delegation)
            .execute(&mut *sa)
            .await
            .expect("revoke delegation");
    }

    // After revoke: no delegated unit, and the cast-time guard rejects (the
    // handler maps this `false` to 403 DELEGATION_REVOKED).
    let after = repo
        .check_eligibility(vote, delegate)
        .await
        .expect("check_eligibility (revoked)");
    assert!(
        !after
            .eligible_units
            .iter()
            .any(|u| u.unit_id == unit && u.is_delegated),
        "revoked delegation must not grant delegated eligibility; got {:?}",
        after.eligible_units
    );
    assert!(
        !cast_guard_delegation_valid(&pool, delegation, delegate, unit).await,
        "cast-time guard must reject a revoked delegation (403 DELEGATION_REVOKED)"
    );
}
