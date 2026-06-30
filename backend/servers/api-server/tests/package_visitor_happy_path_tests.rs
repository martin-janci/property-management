//! Happy-path (2xx) coverage for the governance `package_visitor.rs` endpoints
//! — packages + visitors (BIT-408 Wave 8, governance batch 3).
//!
//! Every handler uses `TenantExtractor` (needs an `X-Tenant-ID` header plus an
//! active `organization_members` row) and `AuthUser` (the staff-only mutators
//! — receive / check-in / check-out — read the JWT `role` claim). We mint a raw
//! HS256 token with `role = "manager"` and seed a building + unit so the
//! required `unit_id` FK resolves.

#![allow(dead_code)]

mod common;

use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use common::{seed_membership, seed_org, TestApp, TestConfig, TestResponse};

#[derive(Serialize)]
struct TestClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_token(user_id: Uuid, org_id: Uuid) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = TestClaims {
        sub: user_id,
        exp: now + 3600,
        iat: now,
        token_type: "access".to_string(),
        tenant_id: Some(org_id),
        role: Some("manager".to_string()),
        email: "pkgvis-hp@test.test".to_string(),
        name: "PkgVisitor HP User".to_string(),
    };
    let secret = TestConfig::default().jwt_secret;
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("encode test JWT")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO users (email, password_hash, name, status, email_verified_at)
           VALUES ($1, 'test_hash', 'PkgVisitor User', 'active', NOW()) RETURNING id"#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO buildings (organization_id, street, city, postal_code, country)
           VALUES ($1, 'Package St 1', 'Bratislava', '81101', 'Slovakia') RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO units (building_id, designation) VALUES ($1, 'R-1') RETURNING id"#,
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

struct Ctx {
    app: TestApp,
    org_id: Uuid,
    building_id: Uuid,
    unit_id: Uuid,
    token: String,
}

async fn setup(pool: PgPool, tag: &str) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, tag).await;
    let user_id = seed_user(&pool, &format!("{tag}-{}@pkgvis-hp.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_id, user_id, "manager").await;
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    let token = mint_token(user_id, org_id);
    Ctx {
        app,
        org_id,
        building_id,
        unit_id,
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
    fn post_empty(&self, uri: &str) -> common::RequestBuilder {
        self.app
            .post(uri)
            .bearer(&self.token)
            .header("X-Tenant-ID", &self.tenant())
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

    async fn create_package(&self) -> Uuid {
        let resp = self
            .post(
                "/api/v1/packages",
                json!({ "unit_id": self.unit_id, "carrier": "ups" }),
            )
            .send()
            .await;
        ok(&resp, "create_package");
        nested_id(&resp.json_value(), "package")
    }

    async fn create_visitor(&self) -> (Uuid, String) {
        let resp = self
            .post(
                "/api/v1/visitors",
                json!({
                    "unit_id": self.unit_id,
                    "visitor_name": "Jane Visitor",
                    "purpose": "guest",
                    "expected_arrival": (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339()
                }),
            )
            .send()
            .await;
        ok(&resp, "create_visitor");
        let v = resp.json_value();
        let id = nested_id(&v, "visitor");
        let code = v["visitor"]["access_code"]
            .as_str()
            .expect("access_code")
            .to_string();
        (id, code)
    }
}

fn ok(resp: &TestResponse, what: &str) {
    assert!(
        resp.status.is_success(),
        "{what} should 2xx, got {}: {}",
        resp.status,
        resp.text()
    );
}

fn nested_id(v: &Value, key: &str) -> Uuid {
    v[key]["id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("expected {key}.id in {v}"))
}

// ---------------------------------------------------------------------------
// Packages
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn packages_happy_path(pool: PgPool) {
    let ctx = setup(pool, "pkg").await;
    let bid = ctx.building_id;
    let id = ctx.create_package().await;

    ok(&ctx.get("/api/v1/packages").send().await, "list_packages");
    ok(
        &ctx.get(&format!("/api/v1/packages/{id}")).send().await,
        "get_package",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/packages/{id}"),
            json!({ "notes": "Front desk" }),
        )
        .send()
        .await,
        "update_package",
    );
    ok(
        &ctx.post(&format!("/api/v1/packages/{id}/receive"), json!({}))
            .send()
            .await,
        "receive_package",
    );
    ok(
        &ctx.post(&format!("/api/v1/packages/{id}/pickup"), json!({}))
            .send()
            .await,
        "pickup_package",
    );

    ok(
        &ctx.get(&format!("/api/v1/packages/buildings/{bid}/settings"))
            .send()
            .await,
        "get_package_settings",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/packages/buildings/{bid}/settings"),
            json!({ "max_storage_days": 14 }),
        )
        .send()
        .await,
        "update_package_settings",
    );
    ok(
        &ctx.get(&format!("/api/v1/packages/buildings/{bid}/statistics"))
            .send()
            .await,
        "get_package_statistics",
    );

    // delete a fresh package so the lifecycle one above is untouched
    let del_id = ctx.create_package().await;
    ok(
        &ctx.delete(&format!("/api/v1/packages/{del_id}"))
            .send()
            .await,
        "delete_package",
    );
}

// ---------------------------------------------------------------------------
// Visitors
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-440: quarantined — fails on dev (workspace hostage); see BIT-440"]
async fn visitors_happy_path(pool: PgPool) {
    let ctx = setup(pool, "vis").await;
    let bid = ctx.building_id;
    let (id, code) = ctx.create_visitor().await;

    ok(&ctx.get("/api/v1/visitors").send().await, "list_visitors");
    ok(
        &ctx.post(
            "/api/v1/visitors/verify-code",
            json!({ "access_code": code, "building_id": bid }),
        )
        .send()
        .await,
        "verify_access_code",
    );
    ok(
        &ctx.get(&format!("/api/v1/visitors/{id}")).send().await,
        "get_visitor",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/visitors/{id}"),
            json!({ "notes": "Arriving soon" }),
        )
        .send()
        .await,
        "update_visitor",
    );
    ok(
        &ctx.post(&format!("/api/v1/visitors/{id}/check-in"), json!({}))
            .send()
            .await,
        "check_in_visitor",
    );
    ok(
        &ctx.post(&format!("/api/v1/visitors/{id}/check-out"), json!({}))
            .send()
            .await,
        "check_out_visitor",
    );

    // cancel needs a pending visitor → use a fresh one
    let (cancel_id, _) = ctx.create_visitor().await;
    ok(
        &ctx.post_empty(&format!("/api/v1/visitors/{cancel_id}/cancel"))
            .send()
            .await,
        "cancel_visitor",
    );

    // delete a fresh visitor
    let (del_id, _) = ctx.create_visitor().await;
    ok(
        &ctx.delete(&format!("/api/v1/visitors/{del_id}"))
            .send()
            .await,
        "delete_visitor",
    );

    ok(
        &ctx.get(&format!("/api/v1/visitors/buildings/{bid}/settings"))
            .send()
            .await,
        "get_visitor_settings",
    );
    ok(
        &ctx.put(
            &format!("/api/v1/visitors/buildings/{bid}/settings"),
            json!({ "code_length": 8 }),
        )
        .send()
        .await,
        "update_visitor_settings",
    );
    ok(
        &ctx.get(&format!("/api/v1/visitors/buildings/{bid}/statistics"))
            .send()
            .await,
        "get_visitor_statistics",
    );
}
