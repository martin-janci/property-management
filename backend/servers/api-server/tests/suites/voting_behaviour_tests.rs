//! Voting endpoint happy-path integration tests — Wave 3 (BIT-267).
//!
//! Covers the remaining partial voting endpoints not exercised by the original
//! voting_tests.rs happy-path test:
//!   update_vote, delete_vote, cancel_vote, close_vote,
//!   list_questions, update_question, delete_question,
//!   check_eligibility, get_my_response, add_comment, list_comments,
//!   list_replies, hide_comment, get_audit_log, list_active_by_building.

#![allow(dead_code)]

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

// ---------------------------------------------------------------------------
// Shared seed helpers
// ---------------------------------------------------------------------------

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country)
           VALUES ($1, 'Behaviour Building', 'Test St 1', 'Bratislava', '81101', 'Slovakia')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation, ownership_share)
           VALUES ($1, 'U-BHV-1', 1.0) RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_resident(pool: &PgPool, unit_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO unit_residents (unit_id, user_id, resident_type)
           VALUES ($1, $2, 'owner') ON CONFLICT DO NOTHING"#,
    )
    .bind(unit_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed resident");
}

async fn seed_membership(pool: &PgPool, org_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO user_memberships (user_id, organization_id, role)
           VALUES ($1, $2, 'OrgAdmin') ON CONFLICT DO NOTHING"#,
    )
    .bind(user_id)
    .bind(org_id)
    .execute(pool)
    .await
    .expect("seed membership");
}

/// Create a draft vote, returning its id.
async fn create_draft_vote(app: &TestApp, token: &str, org_id: Uuid, building_id: Uuid) -> Uuid {
    let end_at = (Utc::now() + Duration::days(7)).to_rfc3339();
    let payload = json!({
        "building_id": building_id,
        "title": "Test Vote",
        "description": "Integration test vote",
        "start_at": null,
        "end_at": end_at,
        "quorum_type": "simple_majority",
        "quorum_percentage": 50,
        "allow_delegation": false,
        "anonymous_voting": false
    });
    let response = app
        .post("/api/v1/voting")
        .bearer(token)
        .tenant(org_id)
        .json(&payload)
        .build();
    let r = app.execute(response).await;
    assert_eq!(
        r.status,
        StatusCode::CREATED,
        "create draft vote: {}",
        r.text()
    );
    let v = r.json_value();
    Uuid::parse_str(v["id"].as_str().expect("id")).expect("uuid")
}

/// Add a yes/no question to a draft vote, returning its id.
async fn add_question(app: &TestApp, token: &str, org_id: Uuid, vote_id: Uuid) -> Uuid {
    let payload = json!({
        "question_text": "Do you approve?",
        "description": null,
        "question_type": "yes_no",
        "options": [],
        "display_order": 1,
        "is_required": true
    });
    let r = app
        .post(&format!("/api/v1/voting/{}/questions", vote_id))
        .bearer(token)
        .tenant(org_id)
        .json(&payload)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add question: {}",
        resp.text()
    );
    let q = resp.json_value();
    Uuid::parse_str(q["id"].as_str().expect("question id")).expect("uuid")
}

/// Publish a draft vote to make it active.
async fn publish_vote(app: &TestApp, token: &str, org_id: Uuid, vote_id: Uuid) {
    let r = app
        .post(&format!("/api/v1/voting/{}/publish", vote_id))
        .bearer(token)
        .tenant(org_id)
        .json(json!({ "start_at": null }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "publish vote: {}", resp.text());
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_vote_title_succeeds(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "updvote").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;

    let r = app
        .put(&format!("/api/v1/voting/{}", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "title": "Updated Title" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "update vote: {}", resp.text());
    assert_eq!(resp.json_value()["title"].as_str(), Some("Updated Title"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_draft_vote_succeeds(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "delvote").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;

    let r = app
        .delete(&format!("/api/v1/voting/{}", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete vote: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_cancel_active_vote_succeeds(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "canvote").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let unit_id = seed_unit(&app.pool, building_id).await;
    seed_resident(&app.pool, unit_id, user_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;
    add_question(&app, &token, org_id, vote_id).await;
    publish_vote(&app, &token, org_id, vote_id).await;

    let r = app
        .post(&format!("/api/v1/voting/{}/cancel", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "reason": "Test cancellation" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "cancel vote: {}", resp.text());
    assert_eq!(resp.json_value()["status"].as_str(), Some("cancelled"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_close_active_vote_succeeds(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "clsvote").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let unit_id = seed_unit(&app.pool, building_id).await;
    seed_resident(&app.pool, unit_id, user_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;
    add_question(&app, &token, org_id, vote_id).await;
    publish_vote(&app, &token, org_id, vote_id).await;

    let r = app
        .post(&format!("/api/v1/voting/{}/close", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "close vote: {}", resp.text());
    assert_eq!(resp.json_value()["status"].as_str(), Some("closed"));
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_questions_for_vote(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "lstquot").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;
    add_question(&app, &token, org_id, vote_id).await;

    let r = app
        .get(&format!("/api/v1/voting/{}/questions", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list questions: {}",
        resp.text()
    );
    let questions = resp.json_value();
    assert!(
        questions.as_array().map(|a| a.len() == 1).unwrap_or(false),
        "expected 1 question"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_update_question_text(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "updqust").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;
    let question_id = add_question(&app, &token, org_id, vote_id).await;

    let r = app
        .put(&format!(
            "/api/v1/voting/{}/questions/{}",
            vote_id, question_id
        ))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "question_text": "Revised question?" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.json_value()["question_text"].as_str(),
        Some("Revised question?")
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_delete_question_from_draft_vote(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "delqust").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;
    let question_id = add_question(&app, &token, org_id, vote_id).await;

    let r = app
        .delete(&format!(
            "/api/v1/voting/{}/questions/{}",
            vote_id, question_id
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::NO_CONTENT,
        "delete question: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_check_eligibility_returns_result(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "eligvot").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let unit_id = seed_unit(&app.pool, building_id).await;
    seed_resident(&app.pool, unit_id, user_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;
    add_question(&app, &token, org_id, vote_id).await;
    publish_vote(&app, &token, org_id, vote_id).await;

    let r = app
        .get(&format!("/api/v1/voting/{}/eligibility", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "check eligibility: {}",
        resp.text()
    );
    let body = resp.json_value();
    assert!(body.get("can_vote").is_some(), "expected can_vote field");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_my_response_after_cast(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "myresvt").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let unit_id = seed_unit(&app.pool, building_id).await;
    seed_resident(&app.pool, unit_id, user_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;
    let question_id = add_question(&app, &token, org_id, vote_id).await;
    publish_vote(&app, &token, org_id, vote_id).await;

    let cast_payload = json!({
        "unit_id": unit_id,
        "delegation_id": null,
        "answers": { question_id.to_string(): true }
    });
    let r = app
        .post(&format!("/api/v1/voting/{}/cast", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .json(&cast_payload)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "cast vote: {}", resp.text());

    let r = app
        .get(&format!(
            "/api/v1/voting/{}/my-response?unit_id={}",
            vote_id, unit_id
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get my response: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_add_and_list_comments(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "cmtvote").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("/api/v1/voting/{}/comments", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "parent_id": null, "content": "A test comment", "ai_consent": false }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "add comment: {}",
        resp.text()
    );
    let comment_id =
        Uuid::parse_str(resp.json_value()["id"].as_str().expect("comment id")).expect("uuid");

    let r = app
        .get(&format!("/api/v1/voting/{}/comments", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list comments: {}",
        resp.text()
    );
    assert!(
        resp.json_value()
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "expected at least one comment"
    );

    // list replies to the top-level comment (should be empty)
    let r = app
        .get(&format!(
            "/api/v1/voting/{}/comments/{}/replies",
            vote_id, comment_id
        ))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "list replies: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_hide_comment(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "hidevot").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;

    let r = app
        .post(&format!("/api/v1/voting/{}/comments", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "parent_id": null, "content": "To hide", "ai_consent": false }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::CREATED);
    let comment_id =
        Uuid::parse_str(resp.json_value()["id"].as_str().expect("comment id")).expect("uuid");

    let r = app
        .post(&format!(
            "/api/v1/voting/{}/comments/{}/hide",
            vote_id, comment_id
        ))
        .bearer(&token)
        .tenant(org_id)
        .json(json!({ "reason": "Inappropriate content" }))
        .build();
    let resp = app.execute(r).await;
    assert_eq!(resp.status, StatusCode::OK, "hide comment: {}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_get_audit_log(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "auditvt").await;
    let building_id = seed_building(&app.pool, org_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;

    let r = app
        .get(&format!("/api/v1/voting/{}/audit", vote_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get audit log: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn test_list_active_by_building(pool: PgPool) {
    let app = TestApp::new(pool).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "actbldv").await;
    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("user id");
    seed_membership(&app.pool, org_id, user_id).await;
    let building_id = seed_building(&app.pool, org_id).await;
    let unit_id = seed_unit(&app.pool, building_id).await;
    seed_resident(&app.pool, unit_id, user_id).await;
    let vote_id = create_draft_vote(&app, &token, org_id, building_id).await;
    add_question(&app, &token, org_id, vote_id).await;
    publish_vote(&app, &token, org_id, vote_id).await;

    let r = app
        .get(&format!("/api/v1/voting/building/{}/active", building_id))
        .bearer(&token)
        .tenant(org_id)
        .build();
    let resp = app.execute(r).await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list active by building: {}",
        resp.text()
    );
    assert!(
        resp.json_value()
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "expected at least one active vote"
    );
}
