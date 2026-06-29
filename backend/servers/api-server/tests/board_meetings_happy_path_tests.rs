//! Happy-path (2xx) coverage for the board-meeting / governance endpoints
//! (Epic 143, BIT-340 Wave 5).
//!
//! The board-meeting handlers all acquire an [`RlsConnection`] (PAP-137), which
//! validates tenant membership and sets the org/user GUCs. Under `TestApp` no
//! `host_tenant_middleware` is mounted, so the `X-Tenant-ID` header is the only
//! tenant source — we set it to the caller's seeded org. Each test provisions an
//! org + org-admin member, mints a real HS256 access token, and drives a board
//! endpoint asserting an HTTP-level 2xx.
//!
//! `motions/{id}/vote` (`cast_vote`) is intentionally omitted: the handler uses
//! the motion id as a placeholder `board_member_id`, so a real 2xx requires
//! production board-member resolution that does not exist yet.

#[allow(dead_code)]
mod common;

use axum::http::StatusCode;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, TestApp, TestConfig};

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
    .bind(format!("Board Org {slug}"))
    .bind(format!("board-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@board-hp.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Board User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn mint_token(user_id: Uuid, email: &str, org_id: Uuid) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "Board User", Some(org_id), None)
        .expect("mint access token")
}

struct Ctx {
    app: TestApp,
    org_id: Uuid,
    user_id: Uuid,
    token: String,
}

async fn setup(pool: PgPool, tag: &str) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, tag).await;
    let email = format!("{tag}-{}@board-hp.test", Uuid::new_v4());
    let user_id = seed_user(&pool, &email).await;
    seed_membership(&pool, org_id, user_id, "org_admin").await;
    let token = mint_token(user_id, &email, org_id);
    Ctx {
        app,
        org_id,
        user_id,
        token,
    }
}

impl Ctx {
    fn tenant(&self) -> String {
        self.org_id.to_string()
    }

    fn get(&self, uri: &str) -> common::RequestBuilder {
        self.app
            .get(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
    }

    fn post(&self, uri: &str, body: Value) -> common::RequestBuilder {
        self.app
            .post(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
            .json(body)
    }

    fn put(&self, uri: &str, body: Value) -> common::RequestBuilder {
        self.app
            .put(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
            .json(body)
    }

    fn delete(&self, uri: &str) -> common::RequestBuilder {
        self.app
            .delete(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
    }

    fn id_of(resp_body: Value) -> Uuid {
        resp_body
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .expect("id field")
    }

    /// Create a meeting via the API and return its id.
    async fn create_meeting(&self) -> Uuid {
        let body = json!({
            "title": "Q1 Board Meeting",
            "meeting_type": "regular",
            "scheduled_start": (Utc::now() + Duration::days(3)).to_rfc3339()
        });
        let resp = self
            .app
            .execute(self.post("/api/v1/board-meetings", body).build())
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "create_meeting should 200: {}",
            resp.text()
        );
        Self::id_of(resp.json_value())
    }

    async fn create_member(&self) -> Uuid {
        let body = json!({
            "user_id": self.user_id,
            "role": "president",
            "term_start": "2026-01-01"
        });
        let resp = self
            .app
            .execute(self.post("/api/v1/board-meetings/members", body).build())
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "create_board_member should 200: {}",
            resp.text()
        );
        Self::id_of(resp.json_value())
    }

    async fn create_agenda(&self, meeting_id: Uuid) -> Uuid {
        let body = json!({
            "meeting_id": meeting_id,
            "item_number": "1",
            "title": "Approve prior minutes"
        });
        let resp = self
            .app
            .execute(
                self.post(&format!("/api/v1/board-meetings/{meeting_id}/agenda"), body)
                    .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "add_agenda_item should 200: {}",
            resp.text()
        );
        Self::id_of(resp.json_value())
    }

    async fn create_motion(&self, meeting_id: Uuid) -> Uuid {
        let body = json!({
            "meeting_id": meeting_id,
            "title": "Motion to approve budget",
            "motion_text": "Resolved, that the 2026 budget be approved."
        });
        let resp = self
            .app
            .execute(
                self.post(
                    &format!("/api/v1/board-meetings/{meeting_id}/motions"),
                    body,
                )
                .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "create_motion should 200: {}",
            resp.text()
        );
        Self::id_of(resp.json_value())
    }

    async fn create_minutes(&self, meeting_id: Uuid) -> Uuid {
        let body = json!({
            "meeting_id": meeting_id,
            "call_to_order": "Meeting called to order at 18:00"
        });
        let resp = self
            .app
            .execute(
                self.post(
                    &format!("/api/v1/board-meetings/{meeting_id}/minutes"),
                    body,
                )
                .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "create_minutes should 200: {}",
            resp.text()
        );
        Self::id_of(resp.json_value())
    }

    async fn create_action(&self, meeting_id: Uuid) -> Uuid {
        let body = json!({
            "meeting_id": meeting_id,
            "title": "Obtain three roofing quotes"
        });
        let resp = self
            .app
            .execute(
                self.post(
                    &format!("/api/v1/board-meetings/{meeting_id}/actions"),
                    body,
                )
                .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "create_action_item should 200: {}",
            resp.text()
        );
        Self::id_of(resp.json_value())
    }

    async fn upload_document(&self, meeting_id: Uuid) -> Uuid {
        let body = json!({
            "meeting_id": meeting_id,
            "document_type": "agenda",
            "title": "Board pack"
        });
        let resp = self
            .app
            .execute(
                self.post(
                    &format!("/api/v1/board-meetings/{meeting_id}/documents"),
                    body,
                )
                .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::OK,
            "upload_document should 200: {}",
            resp.text()
        );
        Self::id_of(resp.json_value())
    }

    async fn assert_ok(&self, req: common::RequestBuilder) {
        let resp = self.app.execute(req.build()).await;
        assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
    }

    async fn assert_no_content(&self, req: common::RequestBuilder) {
        let resp = self.app.execute(req.build()).await;
        assert_eq!(resp.status, StatusCode::NO_CONTENT, "{}", resp.text());
    }
}

// ---------------------------------------------------------------------------
// Dashboard + statistics
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_dashboard_succeeds(pool: PgPool) {
    let ctx = setup(pool, "dash").await;
    ctx.assert_ok(ctx.get("/api/v1/board-meetings/dashboard"))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn meeting_type_counts_succeeds(pool: PgPool) {
    let ctx = setup(pool, "stattypes").await;
    ctx.assert_ok(ctx.get("/api/v1/board-meetings/statistics/types"))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn motion_status_counts_succeeds(pool: PgPool) {
    let ctx = setup(pool, "statmotions").await;
    ctx.assert_ok(ctx.get("/api/v1/board-meetings/statistics/motions"))
        .await;
}

// ---------------------------------------------------------------------------
// Board members
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_board_members_succeeds(pool: PgPool) {
    let ctx = setup(pool, "memlist").await;
    ctx.assert_ok(ctx.get("/api/v1/board-meetings/members"))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_board_member_succeeds(pool: PgPool) {
    let ctx = setup(pool, "memcreate").await;
    ctx.create_member().await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_board_member_succeeds(pool: PgPool) {
    let ctx = setup(pool, "memget").await;
    let id = ctx.create_member().await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/members/{id}")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_board_member_succeeds(pool: PgPool) {
    let ctx = setup(pool, "memupd").await;
    let id = ctx.create_member().await;
    let body = json!({ "title": "Treasurer" });
    ctx.assert_ok(ctx.put(&format!("/api/v1/board-meetings/members/{id}"), body))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_board_member_succeeds(pool: PgPool) {
    let ctx = setup(pool, "memdel").await;
    let id = ctx.create_member().await;
    ctx.assert_no_content(ctx.delete(&format!("/api/v1/board-meetings/members/{id}")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn member_attendance_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mematt").await;
    let id = ctx.create_member().await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/members/{id}/attendance")))
        .await;
}

// ---------------------------------------------------------------------------
// Meetings
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_meetings_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mtglist").await;
    ctx.assert_ok(ctx.get("/api/v1/board-meetings")).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_meeting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mtgcreate").await;
    ctx.create_meeting().await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_meeting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mtgget").await;
    let id = ctx.create_meeting().await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/{id}")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_meeting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mtgupd").await;
    let id = ctx.create_meeting().await;
    let body = json!({ "title": "Q1 Board Meeting (rescheduled)" });
    ctx.assert_ok(ctx.put(&format!("/api/v1/board-meetings/{id}"), body))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_meeting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mtgdel").await;
    let id = ctx.create_meeting().await;
    ctx.assert_no_content(ctx.delete(&format!("/api/v1/board-meetings/{id}")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_meeting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mtgstart").await;
    let id = ctx.create_meeting().await;
    ctx.assert_ok(ctx.post(&format!("/api/v1/board-meetings/{id}/start"), json!({})))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn end_meeting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mtgend").await;
    let id = ctx.create_meeting().await;
    ctx.assert_ok(ctx.post(&format!("/api/v1/board-meetings/{id}/end"), json!({})))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cancel_meeting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mtgcancel").await;
    let id = ctx.create_meeting().await;
    ctx.assert_ok(ctx.post(&format!("/api/v1/board-meetings/{id}/cancel"), json!({})))
        .await;
}

// ---------------------------------------------------------------------------
// Agenda items
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_agenda_items_succeeds(pool: PgPool) {
    let ctx = setup(pool, "aglist").await;
    let meeting = ctx.create_meeting().await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/{meeting}/agenda")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_agenda_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "agadd").await;
    let meeting = ctx.create_meeting().await;
    ctx.create_agenda(meeting).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_agenda_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "agget").await;
    let meeting = ctx.create_meeting().await;
    let item = ctx.create_agenda(meeting).await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/agenda/{item}")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_agenda_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "agupd").await;
    let meeting = ctx.create_meeting().await;
    let item = ctx.create_agenda(meeting).await;
    let body = json!({ "title": "Approve prior minutes (amended)" });
    ctx.assert_ok(ctx.put(&format!("/api/v1/board-meetings/agenda/{item}"), body))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn complete_agenda_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "agcomplete").await;
    let meeting = ctx.create_meeting().await;
    let item = ctx.create_agenda(meeting).await;
    let body = json!({ "outcome": "Minutes approved unanimously" });
    ctx.assert_ok(ctx.post(
        &format!("/api/v1/board-meetings/agenda/{item}/complete"),
        body,
    ))
    .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_agenda_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "agdel").await;
    let meeting = ctx.create_meeting().await;
    let item = ctx.create_agenda(meeting).await;
    ctx.assert_no_content(ctx.delete(&format!("/api/v1/board-meetings/agenda/{item}")))
        .await;
}

// ---------------------------------------------------------------------------
// Motions
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_meeting_motions_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motmlist").await;
    let meeting = ctx.create_meeting().await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/{meeting}/motions")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_motion_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motcreate").await;
    let meeting = ctx.create_meeting().await;
    ctx.create_motion(meeting).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_motions_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motlist").await;
    ctx.assert_ok(ctx.get("/api/v1/board-meetings/motions"))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_motion_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motget").await;
    let meeting = ctx.create_meeting().await;
    let motion = ctx.create_motion(meeting).await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/motions/{motion}")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_motion_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motupd").await;
    let meeting = ctx.create_meeting().await;
    let motion = ctx.create_motion(meeting).await;
    let body = json!({ "title": "Motion to approve revised budget" });
    ctx.assert_ok(ctx.put(&format!("/api/v1/board-meetings/motions/{motion}"), body))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn second_motion_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motsecond").await;
    let meeting = ctx.create_meeting().await;
    let motion = ctx.create_motion(meeting).await;
    ctx.assert_ok(ctx.post(
        &format!("/api/v1/board-meetings/motions/{motion}/second"),
        json!({}),
    ))
    .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn start_motion_voting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motstart").await;
    let meeting = ctx.create_meeting().await;
    let motion = ctx.create_motion(meeting).await;
    ctx.assert_ok(ctx.post(
        &format!("/api/v1/board-meetings/motions/{motion}/start-voting"),
        json!({}),
    ))
    .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn end_motion_voting_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motend").await;
    let meeting = ctx.create_meeting().await;
    let motion = ctx.create_motion(meeting).await;
    ctx.assert_ok(ctx.post(
        &format!("/api/v1/board-meetings/motions/{motion}/end-voting"),
        json!({}),
    ))
    .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_motion_succeeds(pool: PgPool) {
    let ctx = setup(pool, "motdel").await;
    let meeting = ctx.create_meeting().await;
    let motion = ctx.create_motion(meeting).await;
    ctx.assert_no_content(ctx.delete(&format!("/api/v1/board-meetings/motions/{motion}")))
        .await;
}

// ---------------------------------------------------------------------------
// Attendance
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_attendance_succeeds(pool: PgPool) {
    let ctx = setup(pool, "attlist").await;
    let meeting = ctx.create_meeting().await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/{meeting}/attendance")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn record_attendance_succeeds(pool: PgPool) {
    let ctx = setup(pool, "attrec").await;
    let meeting = ctx.create_meeting().await;
    let body = json!({
        "meeting_id": meeting,
        "user_id": ctx.user_id,
        "status": "present"
    });
    ctx.assert_ok(ctx.post(
        &format!("/api/v1/board-meetings/{meeting}/attendance"),
        body,
    ))
    .await;
}

// ---------------------------------------------------------------------------
// Minutes
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_minutes_succeeds(pool: PgPool) {
    let ctx = setup(pool, "mincreate").await;
    let meeting = ctx.create_meeting().await;
    ctx.create_minutes(meeting).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_minutes_succeeds(pool: PgPool) {
    let ctx = setup(pool, "minget").await;
    let meeting = ctx.create_meeting().await;
    ctx.create_minutes(meeting).await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/{meeting}/minutes")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_minutes_succeeds(pool: PgPool) {
    let ctx = setup(pool, "minupd").await;
    let meeting = ctx.create_meeting().await;
    let minutes = ctx.create_minutes(meeting).await;
    let body = json!({ "reports": "Treasurer's report presented" });
    ctx.assert_ok(ctx.put(&format!("/api/v1/board-meetings/minutes/{minutes}"), body))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn approve_minutes_succeeds(pool: PgPool) {
    let ctx = setup(pool, "minapprove").await;
    let meeting = ctx.create_meeting().await;
    let minutes = ctx.create_minutes(meeting).await;
    ctx.assert_ok(ctx.post(
        &format!("/api/v1/board-meetings/minutes/{minutes}/approve"),
        json!({}),
    ))
    .await;
}

// ---------------------------------------------------------------------------
// Action items
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_meeting_action_items_succeeds(pool: PgPool) {
    let ctx = setup(pool, "actmlist").await;
    let meeting = ctx.create_meeting().await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/{meeting}/actions")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_action_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "actcreate").await;
    let meeting = ctx.create_meeting().await;
    ctx.create_action(meeting).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_action_items_succeeds(pool: PgPool) {
    let ctx = setup(pool, "actlist").await;
    ctx.assert_ok(ctx.get("/api/v1/board-meetings/actions"))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_action_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "actget").await;
    let meeting = ctx.create_meeting().await;
    let action = ctx.create_action(meeting).await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/actions/{action}")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_action_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "actupd").await;
    let meeting = ctx.create_meeting().await;
    let action = ctx.create_action(meeting).await;
    let body = json!({ "title": "Obtain five roofing quotes" });
    ctx.assert_ok(ctx.put(&format!("/api/v1/board-meetings/actions/{action}"), body))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn complete_action_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "actcomplete").await;
    let meeting = ctx.create_meeting().await;
    let action = ctx.create_action(meeting).await;
    let body = json!({ "completion_notes": "Quotes obtained" });
    ctx.assert_ok(ctx.post(
        &format!("/api/v1/board-meetings/actions/{action}/complete"),
        body,
    ))
    .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_action_item_succeeds(pool: PgPool) {
    let ctx = setup(pool, "actdel").await;
    let meeting = ctx.create_meeting().await;
    let action = ctx.create_action(meeting).await;
    ctx.assert_no_content(ctx.delete(&format!("/api/v1/board-meetings/actions/{action}")))
        .await;
}

// ---------------------------------------------------------------------------
// Documents
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_documents_succeeds(pool: PgPool) {
    let ctx = setup(pool, "doclist").await;
    let meeting = ctx.create_meeting().await;
    ctx.assert_ok(ctx.get(&format!("/api/v1/board-meetings/{meeting}/documents")))
        .await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_document_succeeds(pool: PgPool) {
    let ctx = setup(pool, "docupload").await;
    let meeting = ctx.create_meeting().await;
    ctx.upload_document(meeting).await;
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_document_succeeds(pool: PgPool) {
    let ctx = setup(pool, "docdel").await;
    let meeting = ctx.create_meeting().await;
    let doc = ctx.upload_document(meeting).await;
    ctx.assert_no_content(ctx.delete(&format!("/api/v1/board-meetings/documents/{doc}")))
        .await;
}
