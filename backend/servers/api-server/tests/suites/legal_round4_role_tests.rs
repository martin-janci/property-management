//! Round-4 regressions (audit4-operations) for `/api/v1/legal`.
//!
//! * Legal / compliance mutations are manager-only (role from the validated
//!   membership, not the JWT): a `resident` member gets 403.
//! * `acknowledge_notice` lets a non-manager acknowledge only on their own
//!   behalf; forging another recipient's acknowledgment is 403.
//! * A manager of org B cannot delete org A's document (404 under B's tenant).

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

async fn member(pool: &PgPool, org: Uuid, role: &str) -> (String, Uuid) {
    use api_server::services::JwtService;
    let email = format!("legal-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Legal R4', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .unwrap();
    seed_membership(pool, org, id, role).await;
    let config = TestConfig::default();
    let token = JwtService::new(&config.jwt_secret)
        .unwrap()
        .generate_access_token(id, &email, "Legal R4", None, None)
        .unwrap();
    (token, id)
}

async fn seed_document(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO legal_documents (organization_id, document_type, title, created_by)
           VALUES ($1, 'contract', 'R4 Contract', $2) RETURNING id"#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_notice(pool: &PgPool, org_id: Uuid, created_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO legal_notices (organization_id, notice_type, subject, content, created_by)
           VALUES ($1, 'general', 'R4 Notice', 'Body', $2) RETURNING id"#,
    )
    .bind(org_id)
    .bind(created_by)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn seed_recipient(pool: &PgPool, notice_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"INSERT INTO legal_notice_recipients (notice_id, recipient_type, recipient_id)
           VALUES ($1, 'user', $2)"#,
    )
    .bind(notice_id)
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap();
}

async fn doc_count(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM legal_documents WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_mutate_legal_records(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "lgr4r").await;
    let (_tm, mgr) = member(&pool, org, "manager").await;
    let (token, _) = member(&pool, org, "resident").await;
    let doc = seed_document(&pool, org, mgr).await;
    let notice = seed_notice(&pool, org, mgr).await;
    let tenant = org.to_string();

    let cases = vec![
        app.delete(&format!("/api/v1/legal/documents/{doc}")),
        app.patch(&format!("/api/v1/legal/documents/{doc}"))
            .json(json!({ "title": "pwned" })),
        app.post(&format!("/api/v1/legal/notices/{notice}/send")),
        app.delete(&format!("/api/v1/legal/notices/{notice}")),
        app.post("/api/v1/legal/audit-trail").json(json!({
            "action": "fake",
        })),
    ];
    for rb in cases {
        let req = rb.bearer(&token).header("X-Tenant-ID", &tenant).build();
        let uri = req.uri().to_string();
        let resp = app.execute(req).await;
        assert!(
            resp.status == StatusCode::FORBIDDEN
                // The audit-trail body may fail JSON validation before the
                // handler runs; it must never succeed.
                || (uri.ends_with("/audit-trail") && resp.status.is_client_error()),
            "{uri} must be manager-only, got {}: {}",
            resp.status,
            resp.text()
        );
    }
    assert_eq!(doc_count(&pool, doc).await, 1);
    let title: String = sqlx::query_scalar("SELECT title FROM legal_documents WHERE id = $1")
        .bind(doc)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(title, "R4 Contract");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn member_can_only_acknowledge_own_notice(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "lgr4a").await;
    let (_tm, mgr) = member(&pool, org, "manager").await;
    let (token_r, resident) = member(&pool, org, "resident").await;
    let (_t2, other) = member(&pool, org, "resident").await;
    let notice = seed_notice(&pool, org, mgr).await;
    seed_recipient(&pool, notice, resident).await;
    seed_recipient(&pool, notice, other).await;
    let tenant = org.to_string();

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/legal/notices/{notice}/acknowledge/{other}"
            ))
            .json(json!({}))
            .bearer(&token_r)
            .header("X-Tenant-ID", &tenant)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::FORBIDDEN, "{}", resp.text());
    let acked: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
        "SELECT acknowledged_at FROM legal_notice_recipients WHERE notice_id = $1 AND recipient_id = $2",
    )
    .bind(notice)
    .bind(other)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(acked.is_none());

    let resp = app
        .execute(
            app.post(&format!(
                "/api/v1/legal/notices/{notice}/acknowledge/{resident}"
            ))
            .json(json!({}))
            .bearer(&token_r)
            .header("X-Tenant-ID", &tenant)
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::OK, "{}", resp.text());
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_scope_for_document_delete(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "lgr4x").await;
    let org_b = seed_org(&pool, "lgr4y").await;
    let (token_a, user_a) = member(&pool, org_a, "manager").await;
    let (token_b, _) = member(&pool, org_b, "manager").await;
    let doc_a = seed_document(&pool, org_a, user_a).await;

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/legal/documents/{doc_a}"))
                .bearer(&token_b)
                .header("X-Tenant-ID", &org_b.to_string())
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    assert_eq!(doc_count(&pool, doc_a).await, 1);

    let resp = app
        .execute(
            app.delete(&format!("/api/v1/legal/documents/{doc_a}"))
                .bearer(&token_a)
                .header("X-Tenant-ID", &org_a.to_string())
                .build(),
        )
        .await;
    assert!(resp.status.is_success(), "{}", resp.text());
}
