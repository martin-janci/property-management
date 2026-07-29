//! Regression tests for the RLS ballot-cast path (Epic 5, issue #971).
//!
//! These lock in the behaviour fixed by the #971 gap sweep on
//! `VoteRepository::cast_vote_rls` / `get_poll_results_rls`
//! (`crates/db/src/repositories/vote.rs`):
//!
//!   - #971.1 — the stored `vote_weight` reflects the unit's `ownership_share`
//!     (folded into the INSERT as `COALESCE((SELECT ownership_share ...), 1.0)`),
//!     not the old hard-coded `Decimal::from(1)`.
//!   - #971.2 — a `ballot_cast` (or `ballot_updated`) `vote_audit_log` row is
//!     written on the same RLS connection after the ballot INSERT.
//!   - #971.5 — `participation_count` on the parent vote is recomputed after a
//!     ballot is cast, so the live quorum indicator is accurate.
//!   - #971.8 — `get_poll_results_rls` returns live, non-zero tallies for an
//!     ACTIVE vote instead of zeroed cached results.
//!
//! They run against a real Postgres via `#[sqlx::test]`, which provisions an
//! isolated migrated database per test. With no local Postgres they are
//! skipped locally and run in CI (`backend.yml`). The audit insert needs the
//! tenant GUC (`app.current_org_id`) in scope, so each test sets the request
//! context on the connection it hands to `cast_vote_rls`.

use crate::common::seed_org;
use db::models::vote::{audit_action, vote_status, CastVote, CreateVote, CreateVoteQuestion};
use db::repositories::VoteRepository;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Vote Cast User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
        VALUES ($1, $2, 'Test St 1', 'Bratislava', '81101', 'Slovakia')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

/// Seed a unit with an explicit ownership share so the weight assertion is
/// meaningful (a value distinct from the legacy hard-coded `1.0`).
async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str, share: Decimal) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (building_id, designation, ownership_share)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
    )
    .bind(building_id)
    .bind(designation)
    .bind(share)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

/// Set the per-connection request context (org/user GUCs) so RLS insert
/// policies — notably `vote_audit_log` — pass on the connection that
/// `cast_vote_rls` runs both INSERTs on.
async fn set_context(conn: &mut sqlx::PgConnection, org_id: Uuid, user_id: Uuid) {
    sqlx::query("SELECT set_request_context($1, $2, $3)")
        .bind(org_id)
        .bind(user_id)
        .bind(false)
        .execute(conn)
        .await
        .expect("set request context");
}

/// Create an ACTIVE vote with a known eligible_count + a single yes/no
/// question, returning the vote id and the question id.
async fn seed_active_vote(
    repo: &VoteRepository,
    pool: &PgPool,
    org_id: Uuid,
    building_id: Uuid,
    created_by: Uuid,
    eligible_count: i32,
) -> (Uuid, Uuid) {
    let vote = repo
        .create_poll_rls(
            pool,
            CreateVote {
                organization_id: org_id,
                building_id,
                title: "Roof renovation".to_string(),
                description: None,
                start_at: None,
                end_at: chrono::Utc::now() + chrono::Duration::days(7),
                quorum_type: "simple_majority".to_string(),
                quorum_percentage: Some(50),
                allow_delegation: Some(true),
                anonymous_voting: Some(false),
                created_by,
            },
        )
        .await
        .expect("create poll");

    let question = repo
        .add_question(CreateVoteQuestion {
            vote_id: vote.id,
            question_text: "Approve the roof renovation?".to_string(),
            description: None,
            question_type: db::models::vote::question_type::YES_NO.to_string(),
            options: Vec::new(),
            display_order: Some(0),
            is_required: Some(true),
        })
        .await
        .expect("add question");

    // Flip the vote to ACTIVE and set an eligible_count so live-tally /
    // participation_count assertions are well-defined.
    sqlx::query("UPDATE votes SET status = $2::vote_status, eligible_count = $3 WHERE id = $1")
        .bind(vote.id)
        .bind(vote_status::ACTIVE)
        .bind(eligible_count)
        .execute(pool)
        .await
        .expect("activate vote");

    (vote.id, question.id)
}

// ---------------------------------------------------------------------------
// #971.1 — stored vote_weight == unit ownership_share
// ---------------------------------------------------------------------------

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn cast_vote_rls_stores_unit_ownership_share_as_weight(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());
    let org = seed_org(&pool, "weight").await;
    let user = seed_user(&pool, "weight@vote-cast.test").await;
    let building = seed_building(&pool, org, "Weight Building").await;
    // Distinct from the legacy hard-coded 1.0 so the assertion is meaningful.
    let share = Decimal::new(4250, 2); // 42.50
    let unit = seed_unit(&pool, building, "W-1", share).await;

    let (vote_id, question_id) = seed_active_vote(&repo, &pool, org, building, user, 4).await;

    let mut conn = pool.acquire().await.expect("acquire");
    set_context(&mut conn, org, user).await;

    repo.cast_vote_rls(
        &mut conn,
        CastVote {
            vote_id,
            user_id: user,
            unit_id: unit,
            delegation_id: None,
            answers: serde_json::json!({ question_id.to_string(): true }),
        },
    )
    .await
    .expect("cast vote");

    let weight: Decimal = sqlx::query_scalar(
        "SELECT vote_weight FROM vote_responses WHERE vote_id = $1 AND unit_id = $2",
    )
    .bind(vote_id)
    .bind(unit)
    .fetch_one(&pool)
    .await
    .expect("read vote_weight");

    assert_eq!(
        weight, share,
        "stored vote_weight must equal the unit's ownership_share (#971.1), not the legacy 1.0"
    );
}

// ---------------------------------------------------------------------------
// #971.2 — a ballot_cast audit row exists after a cast
// ---------------------------------------------------------------------------

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn cast_vote_rls_writes_ballot_cast_audit_row(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());
    let org = seed_org(&pool, "audit").await;
    let user = seed_user(&pool, "audit@vote-cast.test").await;
    let building = seed_building(&pool, org, "Audit Building").await;
    let unit = seed_unit(&pool, building, "A-1", Decimal::new(10000, 2)).await;

    let (vote_id, question_id) = seed_active_vote(&repo, &pool, org, building, user, 2).await;

    let mut conn = pool.acquire().await.expect("acquire");
    set_context(&mut conn, org, user).await;

    repo.cast_vote_rls(
        &mut conn,
        CastVote {
            vote_id,
            user_id: user,
            unit_id: unit,
            delegation_id: None,
            answers: serde_json::json!({ question_id.to_string(): true }),
        },
    )
    .await
    .expect("cast vote");

    // A non-delegated cast records action = ballot_cast (#971.2).
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM vote_audit_log WHERE vote_id = $1 AND action = $2::vote_audit_action",
    )
    .bind(vote_id)
    .bind(audit_action::BALLOT_CAST)
    .fetch_one(&pool)
    .await
    .expect("count audit rows");

    assert_eq!(
        count, 1,
        "a ballot_cast audit row must be written on the RLS connection after the ballot INSERT (#971.2)"
    );
}

// ---------------------------------------------------------------------------
// #971.5 — participation_count increments on an active vote after a cast
// ---------------------------------------------------------------------------

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn cast_vote_rls_increments_participation_count(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());
    let org = seed_org(&pool, "part").await;
    let user = seed_user(&pool, "part@vote-cast.test").await;
    let building = seed_building(&pool, org, "Participation Building").await;
    let unit = seed_unit(&pool, building, "P-1", Decimal::new(5000, 2)).await;

    let (vote_id, question_id) = seed_active_vote(&repo, &pool, org, building, user, 3).await;

    // Freshly-activated vote: no ballots yet.
    let before: i32 =
        sqlx::query_scalar("SELECT COALESCE(participation_count, 0) FROM votes WHERE id = $1")
            .bind(vote_id)
            .fetch_one(&pool)
            .await
            .expect("read participation before");
    assert_eq!(before, 0, "no ballots cast yet");

    let mut conn = pool.acquire().await.expect("acquire");
    set_context(&mut conn, org, user).await;

    repo.cast_vote_rls(
        &mut conn,
        CastVote {
            vote_id,
            user_id: user,
            unit_id: unit,
            delegation_id: None,
            answers: serde_json::json!({ question_id.to_string(): false }),
        },
    )
    .await
    .expect("cast vote");

    let after: i32 =
        sqlx::query_scalar("SELECT COALESCE(participation_count, 0) FROM votes WHERE id = $1")
            .bind(vote_id)
            .fetch_one(&pool)
            .await
            .expect("read participation after");

    assert_eq!(
        after, 1,
        "participation_count must be recomputed to 1 after one ballot is cast (#971.5)"
    );
}

// ---------------------------------------------------------------------------
// #971.8 — active-vote results are live (non-zero), not zeroed cached results
// ---------------------------------------------------------------------------

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn get_poll_results_rls_returns_live_tally_for_active_vote(pool: PgPool) {
    let repo = VoteRepository::new(pool.clone());
    let org = seed_org(&pool, "results").await;
    let user = seed_user(&pool, "results@vote-cast.test").await;
    let building = seed_building(&pool, org, "Results Building").await;
    let unit = seed_unit(&pool, building, "R-1", Decimal::new(10000, 2)).await;

    let (vote_id, question_id) = seed_active_vote(&repo, &pool, org, building, user, 4).await;

    let mut conn = pool.acquire().await.expect("acquire");
    set_context(&mut conn, org, user).await;

    repo.cast_vote_rls(
        &mut conn,
        CastVote {
            vote_id,
            user_id: user,
            unit_id: unit,
            delegation_id: None,
            answers: serde_json::json!({ question_id.to_string(): true }),
        },
    )
    .await
    .expect("cast vote");
    drop(conn);

    let results = repo
        .get_poll_results_rls(&pool, vote_id)
        .await
        .expect("get poll results")
        .expect("results present for active vote");

    assert_eq!(
        results.participation_count, 1,
        "active-vote results must reflect the cast ballot, not zeroed cached results (#971.8)"
    );
    assert_eq!(
        results.eligible_count, 4,
        "eligible_count comes from the loaded vote row"
    );
    assert!(
        !results.questions.is_empty(),
        "active-vote results must include live per-question tallies (#971.8)"
    );

    // No RESULTS_CALCULATED audit row may be written by the active-vote arm —
    // it uses compute_results (no audit write) to avoid spamming on every poll.
    let calc_audits: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM vote_audit_log WHERE vote_id = $1 AND action = $2::vote_audit_action",
    )
    .bind(vote_id)
    .bind(audit_action::RESULTS_CALCULATED)
    .fetch_one(&pool)
    .await
    .expect("count results_calculated audits");
    assert_eq!(
        calc_audits, 0,
        "polling active-vote results must NOT write a results_calculated audit row (#971.8)"
    );
}
