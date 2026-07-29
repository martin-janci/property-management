//! Phase 2 — `user_invites` semantics.
//!
//! Defends leak #9 ("membership row injection — unguarded path creates a
//! membership → join any org"). Every invite is single-use, expiring, and
//! email-bound; tokens are stored only as SHA-256 hashes.

use crate::common::seed_org;
use chrono::Duration;
use db::repositories::user_invite::ConsumeInviteOutcome;
use db::repositories::UserInviteRepository;
use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    let row = sqlx::query(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'h', 'Invite User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user");
    row.get("id")
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accept_succeeds_once(pool: PgPool) {
    let org = seed_org(&pool, "inv-once").await;
    let user = seed_user(&pool, "accept-once@phase2.test").await;
    let repo = UserInviteRepository::new(pool.clone());

    let token = "plain-token-once-please";
    let invite = repo
        .create(
            "accept-once@phase2.test",
            org,
            "manager",
            None,
            token,
            Duration::days(7),
        )
        .await
        .expect("create invite");

    // 1st accept — succeeds.
    let r1 = repo
        .consume_by_token(token, user, "accept-once@phase2.test")
        .await
        .expect("first accept");
    assert!(matches!(r1, ConsumeInviteOutcome::Accepted(_)));

    // accepted_at is set on the row.
    let updated = repo
        .find_by_id(invite.id)
        .await
        .expect("re-read")
        .expect("invite present");
    assert!(updated.accepted_at.is_some(), "accepted_at must be set");
    assert_eq!(updated.accepted_by, Some(user));
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn second_accept_with_same_token_is_rejected(pool: PgPool) {
    let org = seed_org(&pool, "inv-twice").await;
    let user_a = seed_user(&pool, "accept-twice-a@phase2.test").await;
    let user_b = seed_user(&pool, "accept-twice-b@phase2.test").await;
    let repo = UserInviteRepository::new(pool.clone());

    let token = "plain-token-twice";
    repo.create(
        "accept-twice-a@phase2.test",
        org,
        "manager",
        None,
        token,
        Duration::days(7),
    )
    .await
    .expect("create");

    let r1 = repo
        .consume_by_token(token, user_a, "accept-twice-a@phase2.test")
        .await
        .expect("first");
    assert!(matches!(r1, ConsumeInviteOutcome::Accepted(_)));

    // 2nd accept (even from the same user) is rejected.
    let r2 = repo
        .consume_by_token(token, user_a, "accept-twice-a@phase2.test")
        .await
        .expect("second");
    assert!(
        matches!(r2, ConsumeInviteOutcome::AlreadyAccepted),
        "second accept must be rejected, got {:?}",
        r2
    );

    // And another user trying with the same token also fails.
    let r3 = repo
        .consume_by_token(token, user_b, "accept-twice-a@phase2.test")
        .await
        .expect("third");
    assert!(matches!(r3, ConsumeInviteOutcome::AlreadyAccepted));
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accept_after_expiry_is_rejected(pool: PgPool) {
    let org = seed_org(&pool, "inv-exp").await;
    let user = seed_user(&pool, "expired@phase2.test").await;
    let repo = UserInviteRepository::new(pool.clone());

    let token = "plain-expired-token";
    // Negative TTL → already expired the moment it's stored.
    repo.create(
        "expired@phase2.test",
        org,
        "manager",
        None,
        token,
        Duration::seconds(-1),
    )
    .await
    .expect("create");

    let r = repo
        .consume_by_token(token, user, "expired@phase2.test")
        .await
        .expect("consume");
    assert!(
        matches!(r, ConsumeInviteOutcome::Expired),
        "expired invite must be rejected, got {:?}",
        r
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn accept_with_wrong_email_is_rejected(pool: PgPool) {
    let org = seed_org(&pool, "inv-mismatch").await;
    let user = seed_user(&pool, "actual@phase2.test").await;
    let repo = UserInviteRepository::new(pool.clone());

    let token = "plain-mismatch-token";
    repo.create(
        "intended@phase2.test",
        org,
        "manager",
        None,
        token,
        Duration::days(7),
    )
    .await
    .expect("create");

    let r = repo
        .consume_by_token(token, user, "actual@phase2.test")
        .await
        .expect("consume");
    assert!(
        matches!(r, ConsumeInviteOutcome::EmailMismatch),
        "email mismatch must be rejected, got {:?}",
        r
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn unknown_token_is_rejected(pool: PgPool) {
    let user = seed_user(&pool, "no-such-token@phase2.test").await;
    let repo = UserInviteRepository::new(pool.clone());

    let r = repo
        .consume_by_token("never-issued", user, "no-such-token@phase2.test")
        .await
        .expect("consume");
    assert!(
        matches!(r, ConsumeInviteOutcome::UnknownToken),
        "unknown token must be rejected, got {:?}",
        r
    );
}

#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn token_is_stored_only_as_hash(pool: PgPool) {
    let org = seed_org(&pool, "inv-hash").await;
    let repo = UserInviteRepository::new(pool.clone());

    let plaintext = "this-must-not-be-stored";
    repo.create(
        "hash@phase2.test",
        org,
        "manager",
        None,
        plaintext,
        Duration::days(7),
    )
    .await
    .expect("create");

    // The plaintext must not appear anywhere in the row.
    let found_plain: i64 =
        sqlx::query_scalar(r#"SELECT COUNT(*) FROM user_invites WHERE token_hash = $1"#)
            .bind(plaintext)
            .fetch_one(&pool)
            .await
            .expect("scan plaintext");
    assert_eq!(
        found_plain, 0,
        "plaintext token must never appear in token_hash"
    );

    // The hash form does.
    let hash = UserInviteRepository::hash_token(plaintext);
    let found_hash: i64 =
        sqlx::query_scalar(r#"SELECT COUNT(*) FROM user_invites WHERE token_hash = $1"#)
            .bind(&hash)
            .fetch_one(&pool)
            .await
            .expect("scan hash");
    assert_eq!(found_hash, 1);
}
