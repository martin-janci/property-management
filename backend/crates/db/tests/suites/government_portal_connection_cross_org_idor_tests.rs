//! Cross-tenant IDOR regression tests for government portal connections (#2413).
//!
//! Before the fix, the single-row repo methods keyed only on `id`, so any
//! authenticated user of any org could read/mutate/delete/probe another org's
//! portal connection — exposing and hijacking portal credentials
//! (`portal_username`, `oauth_client_id`, `api_endpoint`) — by enumerating
//! UUIDs. The handlers in `api-server/src/routes/government_portal.rs` now
//! thread the verified `TenantExtractor.tenant_id` and the repo methods scope
//! every query with `AND organization_id = $N`.
//!
//! The CI pool runs as a superuser that bypasses (FORCE) RLS, so these tests
//! exercise the application-layer ownership keying directly — exactly the gap
//! RLS would not catch in CI, and the reason the app-level scope is the primary
//! fix for this table (its RLS policy keys on the never-set `app.current_tenant_id`
//! GUC and the repo connects via a context-less owner pool).
//!
//! Run with:
//! ```bash
//! cargo test -p db --test government_portal_connection_cross_org_idor_tests
//! ```

use crate::common::seed_org;
use db::models::{CreatePortalConnection, GovernmentPortalType};
use db::repositories::GovernmentPortalRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind)
        VALUES ($1, 'test_hash', 'Gov Portal User', 'active', NOW(), 'staff')
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn sample_connection() -> CreatePortalConnection {
    CreatePortalConnection {
        portal_type: GovernmentPortalType::TaxAuthority,
        portal_name: "Org B Tax Authority".to_string(),
        portal_code: Some("SK-TAX".to_string()),
        country_code: "SK".to_string(),
        api_endpoint: Some("https://secret-b.example/api".to_string()),
        portal_username: Some("org-b-secret-user".to_string()),
        oauth_client_id: Some("org-b-oauth-client".to_string()),
        auto_submit: false,
        test_mode: true,
    }
}

#[sqlx::test]
async fn connection_by_id_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "idor-a").await;
    let org_b = seed_org(&pool, "idor-b").await;
    let user_b = seed_user(&pool, "gov-b@idor.test").await;

    let repo = GovernmentPortalRepository::new(pool.clone());

    // org_b owns a connection holding sensitive credentials.
    let conn = repo
        .create_connection(org_b, sample_connection(), user_b)
        .await
        .expect("create connection for org_b");

    // --- get: org_a must NOT be able to read org_b's connection ---
    let leaked = repo
        .get_connection(conn.id, org_a)
        .await
        .expect("get_connection (cross-org) query ok");
    assert!(
        leaked.is_none(),
        "cross-org get_connection leaked another org's portal credentials"
    );

    // The owner can still read it.
    let owned = repo
        .get_connection(conn.id, org_b)
        .await
        .expect("get_connection (owner) query ok");
    assert!(
        owned.is_some(),
        "owner get_connection should return the row"
    );

    // --- update: org_a must NOT be able to overwrite org_b's connection ---
    let hijacked = repo
        .update_connection(
            conn.id,
            org_a,
            Some("HIJACKED"),
            Some("https://attacker.example/api"),
            Some("attacker"),
            Some("attacker-client"),
            Some(false),
            Some(true),
            Some(false),
        )
        .await
        .expect("update_connection (cross-org) query ok");
    assert!(
        hijacked.is_none(),
        "cross-org update_connection overwrote another org's connection"
    );

    // Confirm org_b's data is untouched after the failed cross-org update.
    let after = repo
        .get_connection(conn.id, org_b)
        .await
        .expect("get_connection (owner) after update ok")
        .expect("owner row still exists");
    assert_eq!(after.portal_name, "Org B Tax Authority");
    assert_eq!(after.portal_username.as_deref(), Some("org-b-secret-user"));

    // --- record_connection_test: org_a must NOT be able to probe it ---
    let tested_cross = repo
        .record_connection_test(conn.id, true, org_a)
        .await
        .expect("record_connection_test (cross-org) query ok");
    assert!(
        !tested_cross,
        "cross-org record_connection_test touched another org's connection"
    );

    // --- delete: org_a must NOT be able to delete org_b's connection ---
    let deleted_cross = repo
        .delete_connection(conn.id, org_a)
        .await
        .expect("delete_connection (cross-org) query ok");
    assert!(
        !deleted_cross,
        "cross-org delete_connection removed another org's connection"
    );

    // The owner can delete it.
    let deleted_owner = repo
        .delete_connection(conn.id, org_b)
        .await
        .expect("delete_connection (owner) query ok");
    assert!(deleted_owner, "owner delete_connection should succeed");
}
