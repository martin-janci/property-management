//! Regression test for #1586 — `list_board_members` / `list_motions` joined
//! `users` on `u.full_name`, but the `users` column is `name` (`full_name` was
//! never created in any migration). Both queries therefore `42703`-failed at
//! fetch → 500 on the board-member-list and motion-list routes, the identical
//! failure mode fixed in `document.rs` by PR #1565.
//!
//! This drives both repo methods through a real joined user and asserts they
//! return the populated joined name — it fails on the old `u.full_name` SQL and
//! passes with `u.name`.

use crate::common::seed_org;
use db::models::board_meetings::{BoardMemberQuery, MotionQuery};
use db::repositories::BoardMeetingRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str, name: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', $2, 'active', NOW(), 'public')
        RETURNING id
        "#,
    )
    .bind(email)
    .bind(name)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_board_members_and_motions_populate_joined_user_name(pool: PgPool) {
    let org = seed_org(&pool, "board-join").await;
    let user = seed_user(&pool, "director@board-join.test", "Jane Director").await;

    sqlx::query(
        r#"
        INSERT INTO board_members (organization_id, user_id, role, title, term_start, is_active)
        VALUES ($1, $2, 'president', 'Board President', DATE '2026-01-01', true)
        "#,
    )
    .bind(org)
    .bind(user)
    .execute(&pool)
    .await
    .expect("seed board member");

    let meeting = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO board_meetings (organization_id, title, scheduled_start, created_by)
        VALUES ($1, 'Annual General Meeting', NOW(), $2)
        RETURNING id
        "#,
    )
    .bind(org)
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("seed meeting");

    sqlx::query(
        r#"
        INSERT INTO meeting_motions (meeting_id, title, motion_text, proposed_by)
        VALUES ($1, 'Approve 2026 budget', 'Motion to approve the 2026 budget', $2)
        "#,
    )
    .bind(meeting)
    .bind(user)
    .execute(&pool)
    .await
    .expect("seed motion");

    let repo = BoardMeetingRepository::new(pool.clone());

    // list_board_members joins users for `user_name`. With the old `u.full_name`
    // this 42703-fails at fetch (→ 500 on the route); with `u.name` it returns
    // the member with the populated name.
    let members = repo
        .list_board_members(&pool, org, BoardMemberQuery::default())
        .await
        .expect("list_board_members must not 42703 on the users join (#1586)");
    assert_eq!(members.len(), 1, "the seeded board member must be listed");
    assert_eq!(
        members[0].user_name.as_deref(),
        Some("Jane Director"),
        "the joined users.name must populate user_name"
    );

    // list_motions joins users for `proposed_by_name` — same bug, same guard.
    let motions = repo
        .list_motions(&pool, org, MotionQuery::default())
        .await
        .expect("list_motions must not 42703 on the users join (#1586)");
    assert_eq!(motions.len(), 1, "the seeded motion must be listed");
    assert_eq!(
        motions[0].proposed_by_name.as_deref(),
        Some("Jane Director"),
        "the joined users.name must populate proposed_by_name"
    );
}
