//! Regression test for the delegation `scopes` enum-array decode fix (PR #1972,
//! follow-up #2034).
//!
//! `delegations.scopes` is a `delegation_scope[]` column (a Postgres array of a
//! user-defined ENUM). sqlx 0.9 cannot decode such an enum array straight into a
//! Rust `Vec<String>`, so PR #1972 added `scopes::text[]` on every read
//! (`DELEGATION_COLUMNS` + the `find_by_owner` / `find_by_delegate` summary
//! selects), `$3::text[]::delegation_scope[]` on the create/update binds, and
//! `ANY(scopes::text[])` in the scope-membership predicate — the same shape as
//! the `#859` `status::text` enum precedent.
//!
//! PR #1972 shipped without a failing-on-main guard (the IG3 gap this test
//! closes). Pre-fix, any read hitting `scopes` panicked on decode, and the
//! scope check compared a bound `text` against a raw enum array. This test:
//!   1. seeds a delegation with a multi-element `scopes` array via
//!      `DelegationRepository::create`,
//!   2. round-trips it through `find_by_owner` / `find_by_delegate` (the
//!      `list_delegations_for_owner` / `list_delegations_for_delegate` methods
//!      named in #2034) asserting `scopes` decodes into the expected `Vec`
//!      (fails pre-fix on the enum-array decode), and
//!   3. asserts `has_delegation` (the `has_active_delegation` method named in
//!      #2034) returns true for a granted scope and for an `all`-scoped
//!      delegation — pinning the `ANY(scopes::text[])` predicate.
//!
//! Note on naming: #2034 refers to `create_delegation` /
//! `list_delegations_for_owner` / `list_delegations_for_delegate` /
//! `has_active_delegation`; the concrete repository methods are `create` /
//! `find_by_owner` / `find_by_delegate` / `has_delegation`.

use db::models::delegation::CreateDelegation;
use db::repositories::DelegationRepository;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::set_ctx;

/// Seed a user and return its id. `slug` keeps the email/name unique per row.
async fn seed_user(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(format!("{slug}@delegation-scopes.test"))
    .bind(format!("Delegation Scopes {slug}"))
    .fetch_one(pool)
    .await
    .expect("seed user")
}

/// A multi-element `scopes` array round-trips through `create`, `find_by_owner`
/// and `find_by_delegate` as a `Vec<String>`. Pre-fix this panics on the
/// `delegation_scope[]` -> `Vec<String>` decode.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delegation_scopes_array_round_trips_as_vec(pool: PgPool) {
    // Super-admin context: the `delegations_super_admin` FOR ALL policy lets the
    // seed insert + reads bypass the owner/delegate RLS legs.
    set_ctx(&pool, None, None, true).await;

    let owner_id = seed_user(&pool, "owner").await;
    let delegate_id = seed_user(&pool, "delegate").await;
    let repo = DelegationRepository::new(pool.clone());

    // Multi-element array using real `delegation_scope` enum members
    // (`financial`, `faults`) — the enum has no `finance`/`maintenance`.
    let expected_scopes = vec!["financial".to_string(), "faults".to_string()];

    let created = repo
        .create(
            owner_id,
            CreateDelegation {
                delegate_user_id: delegate_id,
                unit_id: None,
                scopes: expected_scopes.clone(),
                start_date: None,
                end_date: None,
            },
        )
        .await
        .expect("create must decode the delegation_scope[] RETURNING into Vec<String>");

    assert_eq!(
        created.scopes, expected_scopes,
        "create RETURNING must decode scopes as its text array"
    );

    // list_delegations_for_owner
    let by_owner = repo
        .find_by_owner(owner_id)
        .await
        .expect("find_by_owner must decode the delegation_scope[] column into Vec<String>");
    assert_eq!(
        by_owner.len(),
        1,
        "the one seeded delegation must be returned"
    );
    assert_eq!(by_owner[0].id, created.id);
    assert_eq!(
        by_owner[0].scopes, expected_scopes,
        "find_by_owner must decode scopes as its text array"
    );

    // list_delegations_for_delegate
    let by_delegate = repo
        .find_by_delegate(delegate_id)
        .await
        .expect("find_by_delegate must decode the delegation_scope[] column into Vec<String>");
    assert_eq!(by_delegate.len(), 1);
    assert_eq!(by_delegate[0].id, created.id);
    assert_eq!(
        by_delegate[0].scopes, expected_scopes,
        "find_by_delegate must decode scopes as its text array"
    );
}

/// `has_delegation` pins the `$3 = ANY(scopes::text[]) OR 'all' = ANY(...)`
/// predicate: true for a granted scope, false for an ungranted one, and true for
/// any scope when the delegation is `all`-scoped. Pre-fix, comparing a bound
/// `text` against the raw `delegation_scope[]` array errors.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn has_delegation_matches_granted_scope_and_all(pool: PgPool) {
    set_ctx(&pool, None, None, true).await;

    let owner_id = seed_user(&pool, "owner").await;
    let delegate_id = seed_user(&pool, "delegate").await;
    let repo = DelegationRepository::new(pool.clone());

    // Scoped delegation ({financial, faults}) — must be `active` for
    // has_delegation, so create (pending) then accept.
    let scoped = repo
        .create(
            owner_id,
            CreateDelegation {
                delegate_user_id: delegate_id,
                unit_id: None,
                scopes: vec!["financial".to_string(), "faults".to_string()],
                start_date: None,
                end_date: None,
            },
        )
        .await
        .expect("create scoped delegation");
    repo.accept(scoped.id, delegate_id)
        .await
        .expect("accept scoped delegation")
        .expect("scoped delegation must transition pending -> active");

    // unit_id NULL means "all units", so any queried unit satisfies the
    // `(unit_id = $2 OR unit_id IS NULL)` leg.
    let some_unit = Uuid::new_v4();

    assert!(
        repo.has_delegation(delegate_id, some_unit, "financial")
            .await
            .expect("has_delegation must not error casting text against scopes::text[]"),
        "a granted scope must match via `$3 = ANY(scopes::text[])`"
    );
    assert!(
        !repo
            .has_delegation(delegate_id, some_unit, "voting")
            .await
            .expect("has_delegation must not error"),
        "an ungranted scope must not match (no `all`, not in scopes)"
    );

    // `all`-scoped delegation from a second owner: any queried scope matches via
    // the `'all' = ANY(scopes::text[])` leg.
    let all_owner_id = seed_user(&pool, "all-owner").await;
    let all_deleg = repo
        .create(
            all_owner_id,
            CreateDelegation {
                delegate_user_id: delegate_id,
                unit_id: None,
                scopes: vec!["all".to_string()],
                start_date: None,
                end_date: None,
            },
        )
        .await
        .expect("create all-scoped delegation");
    repo.accept(all_deleg.id, delegate_id)
        .await
        .expect("accept all-scoped delegation")
        .expect("all-scoped delegation must transition pending -> active");

    assert!(
        repo.has_delegation(delegate_id, some_unit, "voting")
            .await
            .expect("has_delegation must not error"),
        "an `all`-scoped delegation must match any queried scope via `'all' = ANY(scopes::text[])`"
    );
}
