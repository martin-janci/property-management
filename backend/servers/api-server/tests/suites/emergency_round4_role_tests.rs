//! Round-4 regressions (audit4-operations): emergency protocol / contact /
//! drill mutations, incident update/resolve/close and the broadcast
//! acknowledgment roster are manager-only. A plain `resident` member gets 403
//! and nothing is written; a manager succeeds; a manager of another org cannot
//! reach a foreign row (404 under their own tenant).

#![allow(dead_code)]

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp, TestConfig};

async fn seed_user(pool: &PgPool) -> (Uuid, String) {
    let email = format!("em-r4-{}@test.internal", Uuid::new_v4());
    let id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'Emergency R4', 'active', NOW()) RETURNING id"#,
    )
    .bind(&email)
    .fetch_one(pool)
    .await
    .unwrap();
    (id, email)
}

fn mint_token(user_id: Uuid, email: &str) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "Emergency R4", None, None)
        .expect("mint access token")
}

async fn member(pool: &PgPool, org: Uuid, role: &str) -> String {
    let (id, email) = seed_user(pool).await;
    seed_membership(pool, org, id, role).await;
    mint_token(id, &email)
}

async fn seed_protocol(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO emergency_protocols (organization_id, name, protocol_type)
           VALUES ($1, 'Fire Evacuation', 'fire') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn protocol_count(pool: &PgPool, id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM emergency_protocols WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn contact_body(org: Uuid) -> serde_json::Value {
    json!({
        "organization_id": org,
        "name": "Fire brigade",
        "role": "fire",
        "contact_type": "fire_department",
        "phone": "150",
    })
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn resident_cannot_mutate_emergency_setup(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "emr4r").await;
    let token = member(&pool, org, "resident").await;
    let protocol = seed_protocol(&pool, org).await;
    let tenant = org.to_string();

    let fake = Uuid::new_v4();
    let cases = vec![
        app.delete(&format!(
            "/api/v1/emergency/protocols/{protocol}?organization_id={org}"
        )),
        app.post("/api/v1/emergency/contacts")
            .json(contact_body(org)),
        app.post("/api/v1/emergency/drills").json(json!({
            "organization_id": org,
            "building_id": fake,
            "drill_type": "fire",
            "title": "Drill",
            "scheduled_at": "2030-01-01T10:00:00Z",
        })),
        app.delete(&format!(
            "/api/v1/emergency/drills/{fake}?organization_id={org}"
        )),
        app.put(&format!("/api/v1/emergency/incidents/{fake}"))
            .json(json!({ "organization_id": org, "title": "x" })),
        app.get(&format!(
            "/api/v1/emergency/broadcasts/{fake}/acknowledgments?organization_id={org}"
        )),
    ];
    for rb in cases {
        let req = rb.bearer(&token).header("X-Tenant-ID", &tenant).build();
        let uri = req.uri().to_string();
        let resp = app.execute(req).await;
        assert_eq!(
            resp.status,
            StatusCode::FORBIDDEN,
            "{uri} must be manager-only: {}",
            resp.text()
        );
    }

    assert_eq!(protocol_count(&pool, protocol).await, 1);
    let contacts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM emergency_contacts WHERE organization_id = $1")
            .bind(org)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(contacts, 0);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn manager_can_create_contact_and_delete_own_protocol(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org = seed_org(&pool, "emr4m").await;
    let token = member(&pool, org, "manager").await;
    let protocol = seed_protocol(&pool, org).await;
    let tenant = org.to_string();

    let resp = app
        .execute(
            app.post("/api/v1/emergency/contacts")
                .json(contact_body(org))
                .bearer(&token)
                .header("X-Tenant-ID", &tenant)
                .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::CREATED, "{}", resp.text());

    let resp = app
        .execute(
            app.delete(&format!(
                "/api/v1/emergency/protocols/{protocol}?organization_id={org}"
            ))
            .bearer(&token)
            .header("X-Tenant-ID", &tenant)
            .build(),
        )
        .await;
    assert!(resp.status.is_success(), "{}", resp.text());
    assert_eq!(protocol_count(&pool, protocol).await, 0);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cross_org_manager_cannot_delete_foreign_protocol(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let org_a = seed_org(&pool, "emr4a").await;
    let org_b = seed_org(&pool, "emr4b").await;
    let token_b = member(&pool, org_b, "manager").await;
    let protocol_a = seed_protocol(&pool, org_a).await;

    let resp = app
        .execute(
            app.delete(&format!(
                "/api/v1/emergency/protocols/{protocol_a}?organization_id={org_b}"
            ))
            .bearer(&token_b)
            .header("X-Tenant-ID", &org_b.to_string())
            .build(),
        )
        .await;
    assert_eq!(resp.status, StatusCode::NOT_FOUND, "{}", resp.text());
    assert_eq!(protocol_count(&pool, protocol_a).await, 1);
}
