//! Integration tests for board meetings voting endpoints (UC-143).

#[allow(dead_code)]
mod common;

use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, RequestBuilder, TestApp, TestConfig};

// Helper fixtures (copied/adapted from board_meetings_cross_org_idor_tests.rs)

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Voting Test Org {slug}"))
    .bind(format!("voting-test-org-{slug}"))
    .bind(format!("{slug}@voting-test.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Board Voter User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_board_member(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO board_members (organization_id, user_id, role, term_start, is_active)
        VALUES ($1, $2, 'director'::board_role, NOW()::date, true)
        RETURNING id
        "#
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed board member")
}

async fn seed_meeting(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO board_meetings (organization_id, title, scheduled_start, created_by)
        VALUES ($1, 'Voting Test Meeting', NOW() + INTERVAL '7 days', $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .expect("seed meeting")
}

async fn seed_motion(pool: &PgPool, meeting_id: Uuid, proposed_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO meeting_motions (meeting_id, title, motion_text, proposed_by)
        VALUES ($1, 'Approve motion', 'Motion text description', $2)
        RETURNING id
        "#,
    )
    .bind(meeting_id)
    .bind(proposed_by)
    .fetch_one(pool)
    .await
    .expect("seed motion")
}

fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "Board Voter User", None, None)
        .expect("mint access token")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cast_vote_as_board_member_succeeds(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "member").await;
    let user_id = seed_user(&pool, "member@voting-test.test").await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let board_member_id = seed_board_member(&pool, org_id, user_id).await;
    let meeting_id = seed_meeting(&pool, org_id, user_id).await;
    let motion_id = seed_motion(&pool, meeting_id, user_id).await;

    let token = mint_token(user_id, "member@voting-test.test");
    let uri = format!("/api/v1/board-meetings/motions/{motion_id}/vote");
    
    let body = json!({
        "motion_id": motion_id,
        "vote": "in_favor",
        "recusal_reason": null
    });

    let resp = app
        .execute(
            RequestBuilder::new(Method::POST, &uri)
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::OK, "Board member should be able to cast a vote");

    // Assert that the stored row carries the board_member's ID, NOT the motion id
    let stored_board_member_id: Uuid = sqlx::query_scalar(
        "SELECT board_member_id FROM motion_votes WHERE motion_id = $1"
    )
    .bind(motion_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(stored_board_member_id, board_member_id, "The vote must carry the real board member ID");
    assert_ne!(stored_board_member_id, motion_id, "The vote must NOT carry the motion ID as a placeholder");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cast_vote_as_non_board_member_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, "nonmember").await;
    let user_id = seed_user(&pool, "nonmember@voting-test.test").await;
    // They are a regular organization member but NOT a board member.
    seed_membership(&pool, org_id, user_id, "resident").await;
    
    // We need another user to be the board member to create the meeting/motion.
    let creator_user_id = seed_user(&pool, "creator@voting-test.test").await;
    seed_membership(&pool, org_id, creator_user_id, "org_admin").await;
    seed_board_member(&pool, org_id, creator_user_id).await;
    
    let meeting_id = seed_meeting(&pool, org_id, creator_user_id).await;
    let motion_id = seed_motion(&pool, meeting_id, creator_user_id).await;

    let token = mint_token(user_id, "nonmember@voting-test.test");
    let uri = format!("/api/v1/board-meetings/motions/{motion_id}/vote");
    
    let body = json!({
        "motion_id": motion_id,
        "vote": "in_favor",
        "recusal_reason": null
    });

    let resp = app
        .execute(
            RequestBuilder::new(Method::POST, &uri)
                .bearer(&token)
                .header("X-Tenant-ID", &org_id.to_string())
                .json(body)
                .build(),
        )
        .await;

    assert_eq!(resp.status, StatusCode::FORBIDDEN, "Non-board member should be rejected with 403 Forbidden");

    // Also assert that no vote row was created
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM motion_votes WHERE motion_id = $1"
    )
    .bind(motion_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(count, 0, "No vote row should have been created");
}
