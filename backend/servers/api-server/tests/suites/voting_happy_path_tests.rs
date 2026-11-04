//! Happy-path (2xx) coverage for the voting endpoints (UC-04, BIT-340 Wave 5).
//!
//! The existing `voting_tests.rs` only asserts the negative auth paths
//! (`RlsConnection` rejects requests without a Bearer token / tenant context).
//!
//! This file drives the success paths end-to-end: it provisions an org, a
//! member, and a building, mints a real HS256 access token, and sets
//! `X-Tenant-ID` so `ValidatedTenantExtractor` behind `RlsConnection` resolves
//! the caller's organization. Each test then asserts an HTTP-level 2xx on a
//! voting handler.

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, TestApp, TestConfig};

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
    .bind(format!("Voting Org {slug}"))
    .bind(format!("voting-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@voting-hp.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Voting User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

fn mint_token(user_id: Uuid, email: &str, org_id: Uuid) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "Voting User", Some(org_id), None)
        .expect("mint access token")
}

/// Provisioned voting fixture: an org-admin member with a building, ready to
/// drive the voting endpoints.
struct Ctx {
    app: TestApp,
    org_id: Uuid,
    building_id: Uuid,
    token: String,
}

async fn setup(pool: PgPool, tag: &str) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, tag).await;
    let email = format!("{tag}-{}@voting-hp.test", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let building_id = seed_building(&pool, org_id, tag).await;
    let token = mint_token(user_id, &email, org_id);
    Ctx {
        app,
        org_id,
        building_id,
        token,
    }
}

impl Ctx {
    fn tenant(&self) -> String {
        self.org_id.to_string()
    }

    /// Create a draft vote via the API and return its id.
    async fn create_vote(&self) -> Uuid {
        let body = json!({
            "building_id": self.building_id,
            "title": "Roof renovation budget",
            "description": "Approve the 2026 roof renovation budget",
            "end_at": (Utc::now() + Duration::days(7)).to_rfc3339(),
            "quorum_type": "simple_majority"
        });
        let resp = self
            .app
            .execute(
                self.app
                    .post("/api/v1/voting")
                    .bearer(&self.token)
                    .header("X-Tenant-ID", &self.tenant())
                    .json(body)
                    .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::CREATED,
            "create_vote should 201: {}",
            resp.text()
        );
        resp.json_value()
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .expect("vote id")
    }

    /// Add a yes/no question to `vote_id`; return the question id.
    async fn add_question(&self, vote_id: Uuid) -> Uuid {
        let body = json!({
            "question_text": "Do you approve the budget?",
            "question_type": "yes_no",
            "options": [
                {"id": Uuid::new_v4(), "text": "Yes", "order": 1},
                {"id": Uuid::new_v4(), "text": "No", "order": 2}
            ]
        });
        let resp = self
            .app
            .execute(
                self.app
                    .post(&format!("/api/v1/voting/{vote_id}/questions"))
                    .bearer(&self.token)
                    .header("X-Tenant-ID", &self.tenant())
                    .json(body)
                    .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::CREATED,
            "add_question should 201: {}",
            resp.text()
        );
        resp.json_value()
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .expect("question id")
    }

    /// Add a comment to `vote_id`; return the comment id.
    async fn add_comment(&self, vote_id: Uuid) -> Uuid {
        let body = json!({ "content": "Looks good to me", "ai_consent": false });
        let resp = self
            .app
            .execute(
                self.app
                    .post(&format!("/api/v1/voting/{vote_id}/comments"))
                    .bearer(&self.token)
                    .header("X-Tenant-ID", &self.tenant())
                    .json(body)
                    .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::CREATED,
            "add_comment should 201: {}",
            resp.text()
        );
        resp.json_value()
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .expect("comment id")
    }

    /// Publish `vote_id` immediately (start_at = now → status active).
    async fn publish_now(&self, vote_id: Uuid) {
        let body = json!({ "start_at": Utc::now().to_rfc3339() });
        let resp = self
            .app
            .execute(
                self.app
                    .post(&format!("/api/v1/voting/{vote_id}/publish"))
                    .bearer(&self.token)
                    .header("X-Tenant-ID", &self.tenant())
                    .json(body)
                    .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "publish_vote should 200: {}",
            resp.text()
        );
    }
}

// ---------------------------------------------------------------------------
// Vote CRUD
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_vote_succeeds(pool: PgPool) {
    let ctx = setup(pool, "create").await;
    // create_vote() asserts the 201 internally.
    ctx.create_vote().await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_votes_succeeds(pool: PgPool) {
    let ctx = setup(pool, "list").await;
    ctx.create_vote().await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get("/api/v1/voting")
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_vote_succeeds(pool: PgPool) {
    let ctx = setup(pool, "get").await;
    let vote_id = ctx.create_vote().await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!("/api/v1/voting/{vote_id}"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_vote_succeeds(pool: PgPool) {
    let ctx = setup(pool, "update").await;
    let vote_id = ctx.create_vote().await;
    let body = json!({ "title": "Roof renovation budget (revised)" });
    let resp = ctx
        .app
        .execute(
            ctx.app
                .put(&format!("/api/v1/voting/{vote_id}"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .json(body)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_vote_succeeds(pool: PgPool) {
    let ctx = setup(pool, "delete").await;
    let vote_id = ctx.create_vote().await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .delete(&format!("/api/v1/voting/{vote_id}"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "{}", resp.text());
}

// ---------------------------------------------------------------------------
// Questions
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_question_succeeds(pool: PgPool) {
    let ctx = setup(pool, "addq").await;
    let vote_id = ctx.create_vote().await;
    ctx.add_question(vote_id).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_questions_succeeds(pool: PgPool) {
    let ctx = setup(pool, "listq").await;
    let vote_id = ctx.create_vote().await;
    ctx.add_question(vote_id).await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!("/api/v1/voting/{vote_id}/questions"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_question_succeeds(pool: PgPool) {
    let ctx = setup(pool, "updq").await;
    let vote_id = ctx.create_vote().await;
    let question_id = ctx.add_question(vote_id).await;
    let body = json!({ "question_text": "Do you approve the revised budget?" });
    let resp = ctx
        .app
        .execute(
            ctx.app
                .put(&format!("/api/v1/voting/{vote_id}/questions/{question_id}"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .json(body)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_question_succeeds(pool: PgPool) {
    let ctx = setup(pool, "delq").await;
    let vote_id = ctx.create_vote().await;
    let question_id = ctx.add_question(vote_id).await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .delete(&format!("/api/v1/voting/{vote_id}/questions/{question_id}"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NO_CONTENT, "{}", resp.text());
}

// ---------------------------------------------------------------------------
// Workflow: publish / cancel / close
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_vote_succeeds(pool: PgPool) {
    let ctx = setup(pool, "publish").await;
    let vote_id = ctx.create_vote().await;
    ctx.publish_now(vote_id).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancel_vote_succeeds(pool: PgPool) {
    let ctx = setup(pool, "cancel").await;
    let vote_id = ctx.create_vote().await;
    let body = json!({ "reason": "Superseded by a new proposal" });
    let resp = ctx
        .app
        .execute(
            ctx.app
                .post(&format!("/api/v1/voting/{vote_id}/cancel"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .json(body)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn close_vote_succeeds(pool: PgPool) {
    let ctx = setup(pool, "close").await;
    let vote_id = ctx.create_vote().await;
    ctx.publish_now(vote_id).await; // draft -> active
    let resp = ctx
        .app
        .execute(
            ctx.app
                .post(&format!("/api/v1/voting/{vote_id}/close"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

// ---------------------------------------------------------------------------
// Eligibility / my-response
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn check_eligibility_succeeds(pool: PgPool) {
    let ctx = setup(pool, "elig").await;
    let vote_id = ctx.create_vote().await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!("/api/v1/voting/{vote_id}/eligibility"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_my_response_succeeds(pool: PgPool) {
    let ctx = setup(pool, "myresp").await;
    let vote_id = ctx.create_vote().await;
    // No ballot cast yet -> the handler returns `null` with a 200.
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!(
                    "/api/v1/voting/{vote_id}/my-response?unit_id={}",
                    Uuid::new_v4()
                ))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

// ---------------------------------------------------------------------------
// Comments
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_comment_succeeds(pool: PgPool) {
    let ctx = setup(pool, "addc").await;
    let vote_id = ctx.create_vote().await;
    ctx.add_comment(vote_id).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_comments_succeeds(pool: PgPool) {
    let ctx = setup(pool, "listc").await;
    let vote_id = ctx.create_vote().await;
    ctx.add_comment(vote_id).await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!("/api/v1/voting/{vote_id}/comments"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_replies_succeeds(pool: PgPool) {
    let ctx = setup(pool, "listr").await;
    let vote_id = ctx.create_vote().await;
    let comment_id = ctx.add_comment(vote_id).await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!(
                    "/api/v1/voting/{vote_id}/comments/{comment_id}/replies"
                ))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn hide_comment_succeeds(pool: PgPool) {
    let ctx = setup(pool, "hidec").await;
    let vote_id = ctx.create_vote().await;
    let comment_id = ctx.add_comment(vote_id).await;
    let body = json!({ "reason": "Off-topic" });
    let resp = ctx
        .app
        .execute(
            ctx.app
                .post(&format!(
                    "/api/v1/voting/{vote_id}/comments/{comment_id}/hide"
                ))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .json(body)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

// ---------------------------------------------------------------------------
// Results / report / audit / building-scoped
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_results_succeeds(pool: PgPool) {
    let ctx = setup(pool, "results").await;
    let vote_id = ctx.create_vote().await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!("/api/v1/voting/{vote_id}/results"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_report_data_succeeds(pool: PgPool) {
    let ctx = setup(pool, "report").await;
    let vote_id = ctx.create_vote().await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!("/api/v1/voting/{vote_id}/report"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_audit_log_succeeds(pool: PgPool) {
    let ctx = setup(pool, "audit").await;
    let vote_id = ctx.create_vote().await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!("/api/v1/voting/{vote_id}/audit"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_active_by_building_succeeds(pool: PgPool) {
    let ctx = setup(pool, "active").await;
    let vote_id = ctx.create_vote().await;
    ctx.publish_now(vote_id).await; // make it active so it appears in the filter
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!(
                    "/api/v1/voting/building/{}/active",
                    ctx.building_id
                ))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

// ---------------------------------------------------------------------------
// Round-3 audit #5 / #6: ballot payload contract
// ---------------------------------------------------------------------------

impl Ctx {
    async fn post_question(
        &self,
        vote_id: Uuid,
        body: serde_json::Value,
    ) -> crate::common::TestResponse {
        self.app
            .execute(
                self.app
                    .post(&format!("/api/v1/voting/{vote_id}/questions"))
                    .bearer(&self.token)
                    .header("X-Tenant-ID", &self.tenant())
                    .json(body)
                    .build(),
            )
            .await
    }
}

/// #5: `GET /voting/{id}` must carry `questions` (with options) so the ballot
/// page can render and residents can vote.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_vote_includes_questions_with_options(pool: PgPool) {
    let ctx = setup(pool, "getq").await;
    let vote_id = ctx.create_vote().await;
    let question_id = ctx.add_question(vote_id).await;

    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!("/api/v1/voting/{vote_id}"))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let body = resp.json_value();
    let questions = body["questions"].as_array().expect("questions array");
    assert_eq!(questions.len(), 1);
    assert_eq!(body["question_count"], 1);
    assert_eq!(questions[0]["id"], question_id.to_string());
    assert_eq!(questions[0]["options"].as_array().map(Vec::len), Some(2));
}

/// #6: options may omit `id` / `order`; the server generates ids and uses the
/// list position as the order.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_question_generates_missing_option_ids(pool: PgPool) {
    let ctx = setup(pool, "optids").await;
    let vote_id = ctx.create_vote().await;

    let resp = ctx
        .post_question(
            vote_id,
            json!({
                "question_text": "Which contractor?",
                "question_type": "single_choice",
                "options": [{"text": "Alpha"}, {"text": "Beta"}]
            }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let options = resp.json_value()["options"]
        .as_array()
        .cloned()
        .expect("options");
    assert_eq!(options.len(), 2);
    for (i, o) in options.iter().enumerate() {
        let id = o["id"].as_str().expect("generated id");
        assert!(
            Uuid::parse_str(id).is_ok(),
            "option id must be a UUID: {id}"
        );
        assert_eq!(o["order"], i as i64);
    }
    assert_ne!(options[0]["id"], options[1]["id"]);

    // A yes/no question with no options gets the Yes/No pair results rely on.
    let resp = ctx
        .post_question(
            vote_id,
            json!({ "question_text": "Approve?", "question_type": "yes_no" }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let options = resp.json_value()["options"]
        .as_array()
        .cloned()
        .expect("options");
    let texts: Vec<_> = options.iter().map(|o| o["text"].clone()).collect();
    assert_eq!(texts, [json!("Yes"), json!("No")]);

    // Supplied UUIDs are kept as-is.
    let keep = Uuid::new_v4();
    let resp = ctx
        .post_question(
            vote_id,
            json!({
                "question_text": "Keep ids?",
                "question_type": "single_choice",
                "options": [{"id": keep, "text": "A", "order": 5}, {"text": "B"}]
            }),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());
    let options = resp.json_value()["options"]
        .as_array()
        .cloned()
        .expect("options");
    assert_eq!(options[0]["id"], keep.to_string());
    assert_eq!(options[0]["order"], 5);
}

/// The vote list carries `question_count` (VoteCard renders it), on both the
/// org list and the per-building active list.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_votes_includes_question_count(pool: PgPool) {
    let ctx = setup(pool, "qcount").await;
    let with_two = ctx.create_vote().await;
    ctx.add_question(with_two).await;
    ctx.add_question(with_two).await;
    let empty = ctx.create_vote().await;

    let resp = ctx
        .app
        .execute(
            ctx.app
                .get("/api/v1/voting")
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let body = resp.json_value();
    let list = body
        .as_array()
        .or_else(|| body["votes"].as_array())
        .or_else(|| body["items"].as_array())
        .unwrap_or_else(|| panic!("unexpected list shape: {body}"));
    let count_of = |id: Uuid| {
        list.iter()
            .find(|v| v["id"] == id.to_string())
            .map(|v| v["question_count"].clone())
            .unwrap_or_else(|| panic!("vote {id} missing from list"))
    };
    assert_eq!(count_of(with_two), 2);
    assert_eq!(count_of(empty), 0);

    ctx.publish_now(with_two).await;
    let resp = ctx
        .app
        .execute(
            ctx.app
                .get(&format!(
                    "/api/v1/voting/building/{}/active",
                    ctx.building_id
                ))
                .bearer(&ctx.token)
                .header("X-Tenant-ID", &ctx.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let active = resp.json_value();
    let active = active.as_array().expect("active list");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0]["question_count"], 2);
}

// ---------------------------------------------------------------------------
// Cross-org isolation of every `/voting/{id}/*` sub-route
// ---------------------------------------------------------------------------

/// A caller from org B must get 404 on every sub-route of org A's vote (the
/// vote tables only ENABLE RLS, so the handlers check the org explicitly).
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn vote_sub_routes_are_404_for_another_org(pool: PgPool) {
    let a = setup(pool.clone(), "xorg-a").await;
    let b = setup(pool, "xorg-b").await;
    let vote = a.create_vote().await;
    let question = a.add_question(vote).await;
    let comment = a.add_comment(vote).await;
    let unit = Uuid::new_v4();

    use axum::http::Method;
    let cases: Vec<(Method, String, Option<serde_json::Value>)> = vec![
        (Method::GET, format!("/api/v1/voting/{vote}"), None),
        (
            Method::PUT,
            format!("/api/v1/voting/{vote}"),
            Some(json!({})),
        ),
        (
            Method::GET,
            format!("/api/v1/voting/{vote}/questions"),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/voting/{vote}/questions"),
            Some(json!({ "question_text": "x", "question_type": "yes_no" })),
        ),
        (
            Method::PUT,
            format!("/api/v1/voting/{vote}/questions/{question}"),
            Some(json!({ "question_text": "hijacked" })),
        ),
        (
            Method::DELETE,
            format!("/api/v1/voting/{vote}/questions/{question}"),
            None,
        ),
        (
            Method::GET,
            format!("/api/v1/voting/{vote}/eligibility"),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/voting/{vote}/cast"),
            Some(json!({ "unit_id": unit, "answers": {} })),
        ),
        (
            Method::GET,
            format!("/api/v1/voting/{vote}/my-response?unit_id={unit}"),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/voting/{vote}/comments"),
            Some(json!({ "content": "spam", "ai_consent": false })),
        ),
        (Method::GET, format!("/api/v1/voting/{vote}/comments"), None),
        (
            Method::GET,
            format!("/api/v1/voting/{vote}/comments/{comment}/replies"),
            None,
        ),
        (
            Method::POST,
            format!("/api/v1/voting/{vote}/comments/{comment}/hide"),
            Some(json!({ "reason": "x" })),
        ),
        (Method::GET, format!("/api/v1/voting/{vote}/results"), None),
        (Method::GET, format!("/api/v1/voting/{vote}/report"), None),
        (
            Method::GET,
            format!("/api/v1/voting/{vote}/report.pdf"),
            None,
        ),
        (Method::GET, format!("/api/v1/voting/{vote}/audit"), None),
        (
            Method::POST,
            format!("/api/v1/voting/{vote}/publish"),
            Some(json!({})),
        ),
        (
            Method::POST,
            format!("/api/v1/voting/{vote}/cancel"),
            Some(json!({ "reason": "x" })),
        ),
        (Method::POST, format!("/api/v1/voting/{vote}/close"), None),
        (Method::DELETE, format!("/api/v1/voting/{vote}"), None),
    ];

    for (method, uri, body) in cases {
        let builder = match method {
            Method::GET => b.app.get(&uri),
            Method::POST => b.app.post(&uri),
            Method::PUT => b.app.put(&uri),
            Method::DELETE => b.app.delete(&uri),
            _ => unreachable!(),
        }
        .bearer(&b.token)
        .header("X-Tenant-ID", &b.tenant());
        let builder = match body {
            Some(j) => builder.json(j),
            None => builder,
        };
        let resp = b.app.execute(builder.build()).await;
        assert_eq!(
            resp.status,
            StatusCode::NOT_FOUND,
            "{method} {uri} from another org: {}",
            resp.text()
        );
    }

    // Org A's data is untouched and still visible to org A.
    let resp = a
        .app
        .execute(
            a.app
                .get(&format!("/api/v1/voting/{vote}"))
                .bearer(&a.token)
                .header("X-Tenant-ID", &a.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    let body = resp.json_value();
    assert_eq!(body["status"], "draft");
    assert_eq!(
        body["questions"][0]["question_text"],
        "Do you approve the budget?"
    );
    assert_eq!(body["comment_count"], 1);

    // The building's active list from org B never includes org A's votes.
    a.publish_now(vote).await;
    let resp = b
        .app
        .execute(
            b.app
                .get(&format!("/api/v1/voting/building/{}/active", a.building_id))
                .bearer(&b.token)
                .header("X-Tenant-ID", &b.tenant())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    assert_eq!(resp.json_value().as_array().map(Vec::len), Some(0));
}

/// A question or comment id from a different vote (same org) is a 404, not a
/// write against the other vote.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn question_and_comment_ids_must_belong_to_the_vote(pool: PgPool) {
    let ctx = setup(pool, "child").await;
    let vote = ctx.create_vote().await;
    let other = ctx.create_vote().await;
    let other_question = ctx.add_question(other).await;
    let other_comment = ctx.add_comment(other).await;

    let send = |b: crate::common::RequestBuilder| {
        b.bearer(&ctx.token).header("X-Tenant-ID", &ctx.tenant())
    };
    let resp = ctx
        .app
        .execute(
            send(
                ctx.app
                    .put(&format!("/api/v1/voting/{vote}/questions/{other_question}")),
            )
            .json(json!({ "question_text": "moved" }))
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    let resp = ctx
        .app
        .execute(
            send(
                ctx.app
                    .delete(&format!("/api/v1/voting/{vote}/questions/{other_question}")),
            )
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    let resp = ctx
        .app
        .execute(
            send(ctx.app.post(&format!(
                "/api/v1/voting/{vote}/comments/{other_comment}/hide"
            )))
            .json(json!({ "reason": "x" }))
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
}
