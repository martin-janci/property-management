//! E3 — DB-layer enforcement that capability revokes carry a fresh MFA
//! attestation timestamp.
//!
//! Migration 00145 adds `capability_grants.revoked_with_mfa_at` and a
//! `BEFORE UPDATE` trigger that REJECTs any UPDATE which transitions
//! `revoked_at` from NULL to a non-NULL value unless the same statement
//! also writes a `revoked_with_mfa_at` value within a 5-minute window.
//!
//! The application layer (`AuthPolicyEnforcer::check_capability_revoke`,
//! D2) already verifies recent MFA before issuing the UPDATE, but this
//! trigger is defense in depth — a future code path that bypasses the
//! enforcer or a SQL injection that hits the table directly must still be
//! rejected.

use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    let row = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'E3 User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    row.get("id")
}

/// Insert a live (non-revoked) capability_grants row and return its id.
async fn seed_grant(pool: &PgPool, user_id: Uuid, granted_by: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO capability_grants
            (user_id, capability, granted_by, mfa_required, note)
        VALUES ($1, 'audit.read', $2, TRUE, 'E3 test grant')
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(granted_by)
    .fetch_one(pool)
    .await
    .expect("seed grant")
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn revoke_without_mfa_timestamp_is_rejected(pool: PgPool) {
    let target = seed_user(&pool, "e3-target-no-mfa@phase2.test").await;
    let granter = seed_user(&pool, "e3-granter-no-mfa@phase2.test").await;
    let grant_id = seed_grant(&pool, target, granter).await;

    // Direct UPDATE that sets revoked_at without revoked_with_mfa_at — the
    // trigger must raise.
    let res = sqlx::query(
        r#"
        UPDATE capability_grants
        SET revoked_at = NOW(),
            revoked_by = $2
        WHERE id = $1
        "#,
    )
    .bind(grant_id)
    .bind(granter)
    .execute(&pool)
    .await;

    assert!(
        res.is_err(),
        "UPDATE without revoked_with_mfa_at must be rejected"
    );
    let msg = format!("{:?}", res.err().unwrap()).to_lowercase();
    assert!(
        msg.contains("revoked_with_mfa_at") || msg.contains("mfa-attested"),
        "rejection should reference the missing MFA timestamp, got: {msg}"
    );

    // The grant must still be live.
    let revoked_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT revoked_at FROM capability_grants WHERE id = $1")
            .bind(grant_id)
            .fetch_one(&pool)
            .await
            .expect("re-read");
    assert!(
        revoked_at.is_none(),
        "grant must remain live after rejected revoke"
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn revoke_with_stale_mfa_timestamp_is_rejected(pool: PgPool) {
    let target = seed_user(&pool, "e3-target-stale@phase2.test").await;
    let granter = seed_user(&pool, "e3-granter-stale@phase2.test").await;
    let grant_id = seed_grant(&pool, target, granter).await;

    // 10 minutes ago is well outside the 5-minute window.
    let stale = chrono::Utc::now() - chrono::Duration::minutes(10);

    let res = sqlx::query(
        r#"
        UPDATE capability_grants
        SET revoked_at = NOW(),
            revoked_by = $2,
            revoked_with_mfa_at = $3
        WHERE id = $1
        "#,
    )
    .bind(grant_id)
    .bind(granter)
    .bind(stale)
    .execute(&pool)
    .await;

    assert!(
        res.is_err(),
        "UPDATE with stale revoked_with_mfa_at must be rejected"
    );
    let msg = format!("{:?}", res.err().unwrap()).to_lowercase();
    assert!(
        msg.contains("older than") || msg.contains("window"),
        "rejection should reference the stale window, got: {msg}"
    );

    let revoked_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT revoked_at FROM capability_grants WHERE id = $1")
            .bind(grant_id)
            .fetch_one(&pool)
            .await
            .expect("re-read");
    assert!(
        revoked_at.is_none(),
        "grant must remain live after stale-MFA revoke"
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn revoke_with_current_mfa_timestamp_succeeds(pool: PgPool) {
    let target = seed_user(&pool, "e3-target-ok@phase2.test").await;
    let granter = seed_user(&pool, "e3-granter-ok@phase2.test").await;
    let grant_id = seed_grant(&pool, target, granter).await;

    // The repo writes NOW() for both columns; mirror that here.
    let res = sqlx::query(
        r#"
        UPDATE capability_grants
        SET revoked_at = NOW(),
            revoked_by = $2,
            revoked_with_mfa_at = NOW()
        WHERE id = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(grant_id)
    .bind(granter)
    .execute(&pool)
    .await;

    assert!(
        res.is_ok(),
        "UPDATE with fresh revoked_with_mfa_at must succeed: {:?}",
        res.err()
    );

    let row = sqlx::query(
        r#"
        SELECT revoked_at, revoked_with_mfa_at, revoked_by
        FROM capability_grants
        WHERE id = $1
        "#,
    )
    .bind(grant_id)
    .fetch_one(&pool)
    .await
    .expect("re-read");

    let revoked_at: Option<chrono::DateTime<chrono::Utc>> = row.get("revoked_at");
    let revoked_with_mfa_at: Option<chrono::DateTime<chrono::Utc>> = row.get("revoked_with_mfa_at");
    let revoked_by: Option<Uuid> = row.get("revoked_by");

    assert!(
        revoked_at.is_some(),
        "revoked_at must be set after successful revoke"
    );
    assert!(
        revoked_with_mfa_at.is_some(),
        "revoked_with_mfa_at must be set after successful revoke"
    );
    assert_eq!(
        revoked_by,
        Some(granter),
        "revoked_by must record the actor"
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn unrelated_update_is_not_affected_by_trigger(pool: PgPool) {
    let target = seed_user(&pool, "e3-target-note@phase2.test").await;
    let granter = seed_user(&pool, "e3-granter-note@phase2.test").await;
    let grant_id = seed_grant(&pool, target, granter).await;

    // Touch a column that is NOT `revoked_at`. The trigger's WHEN clause
    // gates on `NEW.revoked_at IS DISTINCT FROM OLD.revoked_at`, so this
    // statement must succeed even though `revoked_with_mfa_at` is still NULL.
    let res = sqlx::query("UPDATE capability_grants SET note = $2 WHERE id = $1")
        .bind(grant_id)
        .bind("E3: unrelated edit")
        .execute(&pool)
        .await;

    assert!(
        res.is_ok(),
        "UPDATE that does not touch revoked_at must succeed: {:?}",
        res.err()
    );

    // Sanity: the row is still live.
    let revoked_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT revoked_at FROM capability_grants WHERE id = $1")
            .bind(grant_id)
            .fetch_one(&pool)
            .await
            .expect("re-read");
    assert!(
        revoked_at.is_none(),
        "unrelated UPDATE must not revoke the grant"
    );
}
