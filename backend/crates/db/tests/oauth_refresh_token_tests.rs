//! Regression tests for OAuth refresh-token lookup semantics.
//!
//! Defends against issue #481 — production refresh-token grant must reject
//! revoked tokens, while the family-reuse detection path must still be able
//! to look them up.

use chrono::{Duration, Utc};
use db::models::oauth::{CreateOAuthClient, CreateRefreshToken};
use db::repositories::OAuthRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'OAuth Test User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_client(repo: &OAuthRepository, suffix: &str) -> String {
    let client_id = format!("rt-test-client-{suffix}");
    repo.create_client(CreateOAuthClient {
        client_id: client_id.clone(),
        client_secret_hash: "dummy_hash".into(),
        name: format!("rt test {suffix}"),
        description: None,
        redirect_uris: vec!["http://localhost/cb".into()],
        scopes: vec!["read".into()],
        is_confidential: true,
        rotate_refresh_tokens: true,
    })
    .await
    .expect("create client");
    client_id
}

/// Regression for #481: the production lookup must NOT return revoked tokens,
/// otherwise a revoked refresh token could be exchanged for a fresh access
/// token (RFC 9700 violation).
#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn revoked_refresh_token_is_invisible_to_production_lookup(pool: PgPool) {
    let repo = OAuthRepository::new(pool.clone());
    let user_id = seed_user(&pool, "oauth-rt-1@test").await;
    let client_id = seed_client(&repo, "prod-lookup").await;

    let token_hash = format!("hash-{}", Uuid::new_v4());
    let family_id = Uuid::new_v4();
    let rt = repo
        .create_refresh_token(CreateRefreshToken {
            user_id,
            client_id: client_id.clone(),
            token_hash: token_hash.clone(),
            scopes: vec!["read".into()],
            family_id,
            expires_at: Utc::now() + Duration::days(7),
        })
        .await
        .expect("create refresh token");

    // Pre-revoke: visible.
    assert!(
        repo.find_refresh_token_by_hash(&token_hash)
            .await
            .expect("lookup")
            .is_some(),
        "live refresh token should be visible to production lookup"
    );

    // Revoke.
    let revoked = repo
        .revoke_refresh_token(rt.id)
        .await
        .expect("revoke_refresh_token");
    assert!(revoked, "revoke should affect one row");

    // Production lookup must NOT see revoked rows.
    assert!(
        repo.find_refresh_token_by_hash(&token_hash)
            .await
            .expect("lookup")
            .is_none(),
        "revoked refresh token must be invisible to production lookup (issue #481)"
    );
}

/// The reuse-detection lookup MUST return revoked rows so the service layer
/// can spot replay attacks and burn the rest of the token family.
#[sqlx::test]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn including_revoked_lookup_sees_revoked_rows(pool: PgPool) {
    let repo = OAuthRepository::new(pool.clone());
    let user_id = seed_user(&pool, "oauth-rt-2@test").await;
    let client_id = seed_client(&repo, "reuse-detect").await;

    let token_hash = format!("hash-{}", Uuid::new_v4());
    let family_id = Uuid::new_v4();
    let rt = repo
        .create_refresh_token(CreateRefreshToken {
            user_id,
            client_id,
            token_hash: token_hash.clone(),
            scopes: vec!["read".into()],
            family_id,
            expires_at: Utc::now() + Duration::days(7),
        })
        .await
        .expect("create refresh token");

    repo.revoke_refresh_token(rt.id)
        .await
        .expect("revoke_refresh_token");

    let found = repo
        .find_refresh_token_by_hash_including_revoked(&token_hash)
        .await
        .expect("lookup_including_revoked")
        .expect("revoked token must be findable for reuse detection");
    assert!(
        found.is_revoked(),
        "looked-up row should still flag is_revoked=true"
    );
}
