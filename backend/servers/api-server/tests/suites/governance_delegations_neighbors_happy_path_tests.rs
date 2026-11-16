//! Happy-path (2xx) coverage for the governance delegation + neighbor surfaces
//! (Epic 3 Story 3.4 delegations, Epic 6 Story 6.6 neighbors — BIT-407 Wave 8
//! governance backfill, batch 2).
//!
//! Wave 5 (BIT-340) covered `voting.rs` + `board_meetings.rs`; this batch picks
//! up the remaining governance modules. Each handler acquires either an
//! [`AuthUser`] (user-scoped) or an [`RlsConnection`] (tenant-scoped) — under
//! `TestApp` no `host_tenant_middleware` is mounted, so the `X-Tenant-ID`
//! header is the only tenant source. We set it to the caller's seeded org.
//!
//! `delegations.rs` is fully exercisable end-to-end with two seeded users in
//! one org: an owner creates the delegation, the delegate accepts/declines it,
//! the owner revokes it. `neighbors.rs` privacy endpoints are user-scoped;
//! `list_neighbors` needs a building owned by the caller's org (returns 200
//! with an empty list when the caller is not yet a resident).

use crate::common;

use axum::http::StatusCode;
use serde_json::{json, Value};
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
    .bind(format!("Gov Org {slug}"))
    .bind(format!("gov-org-{slug}-{}", Uuid::new_v4()))
    .bind(format!("{slug}-{}@gov-hp.test", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Gov User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, 'Governance Street 1', 'Bratislava', '81101', 'Slovakia')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, 'G-1', 1) RETURNING id",
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_unit_owner(pool: &PgPool, unit_id: Uuid, user_id: Uuid) {
    sqlx::query("INSERT INTO unit_owners (unit_id, user_id, status) VALUES ($1, $2, 'active')")
        .bind(unit_id)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("seed unit owner");
}

fn mint_token(user_id: Uuid, email: &str, org_id: Uuid) -> String {
    use api_server::services::JwtService;
    let config = TestConfig::default();
    let jwt = JwtService::new(&config.jwt_secret).expect("jwt service");
    jwt.generate_access_token(user_id, email, "Gov User", Some(org_id), None)
        .expect("mint access token")
}

/// Two-user governance context: an `owner` (org admin) and a `delegate`
/// (regular member) inside one org.
struct Ctx {
    app: TestApp,
    org_id: Uuid,
    owner_id: Uuid,
    owner_token: String,
    /// A unit owned by `owner_id` (same org). Used by the cross-owner
    /// authorization regression: a non-owner submitting this `unit_id` must be
    /// rejected.
    owner_unit_id: Uuid,
    delegate_id: Uuid,
    delegate_token: String,
}

async fn setup(pool: PgPool, tag: &str) -> Ctx {
    let app = TestApp::new(pool.clone()).await;
    let org_id = seed_org(&pool, tag).await;

    let owner_email = format!("{tag}-owner-{}@gov-hp.test", Uuid::new_v4());
    let owner_id = seed_user(&pool, &owner_email).await;
    seed_membership(&pool, org_id, owner_id, "org_admin").await;
    let owner_token = mint_token(owner_id, &owner_email, org_id);

    // The owner must actually own a unit: an all-units (null unit_id) delegation
    // is only permitted for a caller who owns at least one unit.
    let building_id = seed_building(&pool, org_id).await;
    let unit_id = seed_unit(&pool, building_id).await;
    seed_unit_owner(&pool, unit_id, owner_id).await;

    let delegate_email = format!("{tag}-delegate-{}@gov-hp.test", Uuid::new_v4());
    let delegate_id = seed_user(&pool, &delegate_email).await;
    seed_membership(&pool, org_id, delegate_id, "member").await;
    let delegate_token = mint_token(delegate_id, &delegate_email, org_id);

    Ctx {
        app,
        org_id,
        owner_id,
        owner_token,
        owner_unit_id: unit_id,
        delegate_id,
        delegate_token,
    }
}

impl Ctx {
    fn tenant(&self) -> String {
        self.org_id.to_string()
    }

    fn get(&self, uri: &str, token: &str) -> common::RequestBuilder {
        self.app
            .get(uri)
            .bearer(token)
            .header("X-Tenant-ID", &self.tenant())
    }

    fn post(&self, uri: &str, token: &str, body: Value) -> common::RequestBuilder {
        self.app
            .post(uri)
            .bearer(token)
            .header("X-Tenant-ID", &self.tenant())
            .json(body)
    }

    fn put(&self, uri: &str, token: &str, body: Value) -> common::RequestBuilder {
        self.app
            .put(uri)
            .bearer(token)
            .header("X-Tenant-ID", &self.tenant())
            .json(body)
    }

    fn delete(&self, uri: &str, token: &str) -> common::RequestBuilder {
        self.app
            .delete(uri)
            .bearer(token)
            .header("X-Tenant-ID", &self.tenant())
    }

    fn id_of(resp_body: Value) -> Uuid {
        resp_body
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .expect("id field")
    }

    /// Owner creates an all-units delegation (no unit_id) for the delegate and
    /// returns its id. The owner owns a seeded unit, so the null-unit owner
    /// check passes.
    async fn create_delegation(&self, scope: &str) -> Uuid {
        let body = json!({
            "delegate_user_id": self.delegate_id,
            "scopes": [scope],
        });
        let resp = self
            .app
            .execute(
                self.post("/api/v1/delegations", &self.owner_token, body)
                    .build(),
            )
            .await;
        assert_eq!(
            resp.status,
            StatusCode::CREATED,
            "create_delegation should 201: {}",
            resp.text()
        );
        Self::id_of(resp.json_value())
    }
}

// ---------------------------------------------------------------------------
// delegations.rs  (mount: /api/v1/delegations)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_delegation_returns_201(pool: PgPool) {
    let ctx = setup(pool, "create-del").await;
    // create_delegation already asserts 201 internally.
    let id = ctx.create_delegation("voting").await;
    assert_ne!(id, Uuid::nil());
}

/// Regression (authorization): creating an all-units delegation (no `unit_id`)
/// must enforce the owner check just like the unit-scoped path. A caller who
/// owns no unit must be rejected with 403 instead of silently creating a
/// building-wide delegation. The `delegate` user is a plain member who owns no
/// unit, so it stands in for the non-owner here.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_delegation_without_unit_rejects_non_owner(pool: PgPool) {
    let ctx = setup(pool, "nonowner-nulldel").await;

    // delegate (owns no unit) tries to create an all-units delegation targeting
    // the owner — must be forbidden, not created.
    let body = json!({
        "delegate_user_id": ctx.owner_id,
        "scopes": ["voting"],
    });
    let resp = ctx
        .app
        .execute(
            ctx.post("/api/v1/delegations", &ctx.delegate_token, body)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "all-units delegation by a non-owner must be 403, got: {}",
        resp.text()
    );
}

/// Regression (authorization, cross-owner): creating a *unit-scoped* delegation
/// (`unit_id` present) for a unit owned by a **different** owner must be
/// rejected with 403, not silently created. This is the `Some(unit_id)` sibling
/// of `create_delegation_without_unit_rejects_non_owner` (which covers the
/// null-unit path): the owner-check in `create_delegation` must hold on both
/// branches. The `delegate` is a plain member who owns no unit; it targets the
/// owner's seeded unit (`owner_unit_id`) and must be forbidden.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn create_delegation_for_other_owner_unit_rejects_non_owner(pool: PgPool) {
    let ctx = setup(pool, "nonowner-unitdel").await;

    // delegate (not an owner of owner_unit_id) tries to create a delegation
    // scoped to the owner's unit — must be forbidden, not created.
    let body = json!({
        "delegate_user_id": ctx.owner_id,
        "unit_id": ctx.owner_unit_id,
        "scopes": ["voting"],
    });
    let resp = ctx
        .app
        .execute(
            ctx.post("/api/v1/delegations", &ctx.delegate_token, body)
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "unit-scoped delegation on another owner's unit must be 403, got: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_my_delegations_returns_200(pool: PgPool) {
    let ctx = setup(pool, "list-mine").await;
    ctx.create_delegation("voting").await;

    let resp = ctx
        .app
        .execute(ctx.get("/api/v1/delegations", &ctx.owner_token).build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_my_delegations should 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().as_array().is_some(),
        "expected a JSON array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_received_delegations_returns_200(pool: PgPool) {
    let ctx = setup(pool, "list-recv").await;
    ctx.create_delegation("voting").await;

    let resp = ctx
        .app
        .execute(
            ctx.get("/api/v1/delegations/received", &ctx.delegate_token)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_received_delegations should 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().as_array().is_some(),
        "expected a JSON array"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_delegation_returns_200(pool: PgPool) {
    let ctx = setup(pool, "get-del").await;
    let id = ctx.create_delegation("voting").await;

    let resp = ctx
        .app
        .execute(
            ctx.get(&format!("/api/v1/delegations/{id}"), &ctx.owner_token)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_delegation should 200: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn accept_delegation_returns_200(pool: PgPool) {
    let ctx = setup(pool, "accept-del").await;
    let id = ctx.create_delegation("voting").await;

    let resp = ctx
        .app
        .execute(
            ctx.post(
                &format!("/api/v1/delegations/{id}/accept"),
                &ctx.delegate_token,
                json!({}),
            )
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "accept_delegation should 200: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn decline_delegation_returns_200(pool: PgPool) {
    let ctx = setup(pool, "decline-del").await;
    let id = ctx.create_delegation("voting").await;

    let resp = ctx
        .app
        .execute(
            ctx.post(
                &format!("/api/v1/delegations/{id}/decline"),
                &ctx.delegate_token,
                json!({}),
            )
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "decline_delegation should 200: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn revoke_delegation_returns_200(pool: PgPool) {
    let ctx = setup(pool, "revoke-del").await;
    let id = ctx.create_delegation("voting").await;

    let resp = ctx
        .app
        .execute(
            ctx.delete(
                &format!("/api/v1/delegations/{id}/revoke"),
                &ctx.owner_token,
            )
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "revoke_delegation should 200: {}",
        resp.text()
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn check_delegation_returns_200(pool: PgPool) {
    let ctx = setup(pool, "check-del").await;
    let unit_id = Uuid::new_v4();

    let resp = ctx
        .app
        .execute(
            ctx.get(
                &format!("/api/v1/delegations/check/{unit_id}/voting"),
                &ctx.owner_token,
            )
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "check_delegation should 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().get("has_delegation").is_some(),
        "expected has_delegation field"
    );
}

// ---------------------------------------------------------------------------
// neighbors.rs  (mount: /api/v1)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_neighbors_returns_200(pool: PgPool) {
    let ctx = setup(pool.clone(), "list-neigh").await;
    let building_id = seed_building(&pool, ctx.org_id).await;

    let resp = ctx
        .app
        .execute(
            ctx.get(
                &format!("/api/v1/buildings/{building_id}/neighbors"),
                &ctx.owner_token,
            )
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list_neighbors should 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().get("neighbors").is_some(),
        "expected neighbors field"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_privacy_settings_returns_200(pool: PgPool) {
    let ctx = setup(pool, "get-priv").await;

    let resp = ctx
        .app
        .execute(
            ctx.get("/api/v1/users/me/privacy", &ctx.owner_token)
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get_privacy_settings should 200: {}",
        resp.text()
    );
    assert!(
        resp.json_value().get("settings").is_some(),
        "expected settings field"
    );
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_privacy_settings_returns_200(pool: PgPool) {
    let ctx = setup(pool, "upd-priv").await;

    let resp = ctx
        .app
        .execute(
            ctx.put(
                "/api/v1/users/me/privacy",
                &ctx.owner_token,
                json!({ "show_contact_info": true }),
            )
            .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update_privacy_settings should 200: {}",
        resp.text()
    );
}
