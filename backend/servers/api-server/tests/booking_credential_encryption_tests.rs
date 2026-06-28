//! BIT-98 — Booking.com basic-auth credentials must be encrypted at rest.
//!
//! # Why this file exists
//!
//! Booking.com Connectivity authenticates machine-to-machine with a provisioned
//! OTA Supply-XML `username` / `password` (no consumer OAuth — see BIT-97). The
//! connect handler reuses the `rental_platform_connections.access_token` /
//! `.refresh_token` columns to store those credentials. Before BIT-98 they were
//! written in **plaintext**, while the Airbnb path already encrypts (#765).
//!
//! These tests pin the at-rest security contract that `connect_booking`
//! (`routes/integrations/install.rs`) and the read-back sites now enforce:
//!
//!   1. Credentials are encrypted before they hit the DB — the raw column value
//!      is ciphertext (`enc:` prefix), never the plaintext password.
//!   2. The read path round-trips ciphertext back to the original plaintext.
//!   3. Encryption is **mandatory** — with no key configured the encrypt helper
//!      fails closed, which is what makes `connect_booking` return
//!      500 `ENCRYPTION_REQUIRED` instead of silently persisting plaintext.
//!   4. The read path stays tolerant of **legacy plaintext** rows during rollout
//!      (no data migration required; `decrypt_if_available` passes them through).
//!
//! # Scope / why not a full HTTP route test
//!
//! `connect_booking` calls `BookingClient::fetch_property` (a live POST to the
//! production Booking.com OTA URL with no client timeout) before storing. A
//! happy-path route test would therefore depend on outbound network and is
//! flaky — the sibling `booking_oauth_routes_tests.rs` documents the same
//! limitation. Instead these tests drive the exact crypto + repository path the
//! handler uses (`encrypt_required` → `create_or_update_booking_connection` →
//! `find_booking_connection_by_org` → `decrypt_if_available`), so the at-rest
//! contract is verified deterministically. The handler-wiring guards
//! (auth/IDOR/route-mounted) are covered in `booking_oauth_routes_tests.rs`.

#![allow(dead_code)]

mod common;

use sqlx::PgPool;
use uuid::Uuid;

use db::repositories::RentalRepository;
use integrations::{decrypt_if_available, encrypt_required, IntegrationCrypto};

// A valid 32-byte (64 hex char) AES-256 key for the test process.
const TEST_KEY: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const HOTEL_ID: &str = "12345";
const PLAINTEXT_USERNAME: &str = "booking-supply-user";
const PLAINTEXT_PASSWORD: &str = "super-secret-booking-password-DO-NOT-LEAK";

fn with_crypto_key() -> Option<IntegrationCrypto> {
    std::env::set_var("INTEGRATION_ENCRYPTION_KEY", TEST_KEY);
    let crypto = IntegrationCrypto::try_from_env();
    assert!(
        crypto.is_some(),
        "INTEGRATION_ENCRYPTION_KEY must yield a usable crypto instance for the test"
    );
    crypto
}

async fn seed_org(pool: &PgPool, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO organizations (name, slug, contact_email, status)
        VALUES ($1, $2, $3, 'active') RETURNING id
        "#,
    )
    .bind(format!("Booking Enc Org {slug}"))
    .bind(format!(
        "booking-enc-org-{slug}-{}",
        Uuid::new_v4().simple()
    ))
    .bind(format!("{slug}@booking-enc.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

/// Read the raw stored column values straight from the DB, bypassing the model
/// helpers, so we observe exactly what is persisted at rest.
async fn read_raw_credentials(pool: &PgPool, org_id: Uuid) -> (Option<String>, Option<String>) {
    sqlx::query_as::<_, (Option<String>, Option<String>)>(
        r#"
        SELECT access_token, refresh_token
        FROM rental_platform_connections
        WHERE organization_id = $1 AND platform = 'booking'
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("read raw credentials")
}

// ---------------------------------------------------------------------------
// (1) + (2): ciphertext at rest, round-trips back to the original plaintext.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_credentials_are_encrypted_at_rest_and_round_trip(pool: PgPool) {
    let crypto = with_crypto_key();
    let repo = RentalRepository::new(pool.clone());
    let org_id = seed_org(&pool, "round-trip").await;

    // Mirror connect_booking: encrypt with the mandatory helper, then store.
    let stored_username =
        encrypt_required(crypto.as_ref(), PLAINTEXT_USERNAME).expect("encrypt username");
    let stored_password =
        encrypt_required(crypto.as_ref(), PLAINTEXT_PASSWORD).expect("encrypt password");
    repo.create_or_update_booking_connection(org_id, HOTEL_ID, &stored_username, &stored_password)
        .await
        .expect("store booking connection");

    // (1) The raw column values must be ciphertext, never the plaintext secret.
    let (raw_user, raw_pass) = read_raw_credentials(&pool, org_id).await;
    let raw_user = raw_user.expect("access_token present");
    let raw_pass = raw_pass.expect("refresh_token present");

    assert_ne!(
        raw_pass, PLAINTEXT_PASSWORD,
        "password must NOT be stored in plaintext at rest"
    );
    assert_ne!(
        raw_user, PLAINTEXT_USERNAME,
        "username must NOT be stored in plaintext at rest"
    );
    assert!(
        raw_pass.starts_with("enc:") && raw_user.starts_with("enc:"),
        "stored credentials must carry the enc: ciphertext prefix; got user={raw_user}, pass={raw_pass}"
    );
    assert!(
        !raw_pass.contains(PLAINTEXT_PASSWORD),
        "ciphertext must not embed the plaintext password substring"
    );

    // (2) The read path (find_booking_connection_by_org + decrypt_if_available)
    //     recovers the original plaintext.
    let connection = repo
        .find_booking_connection_by_org(org_id)
        .await
        .expect("query connection")
        .expect("connection exists");

    let decrypted_user = decrypt_if_available(
        crypto.as_ref(),
        connection.access_token.as_deref().expect("access_token"),
    );
    let decrypted_pass = decrypt_if_available(
        crypto.as_ref(),
        connection.refresh_token.as_deref().expect("refresh_token"),
    );
    assert_eq!(
        decrypted_user, PLAINTEXT_USERNAME,
        "username must round-trip"
    );
    assert_eq!(
        decrypted_pass, PLAINTEXT_PASSWORD,
        "password must round-trip"
    );
}

// ---------------------------------------------------------------------------
// (3): encryption is mandatory — no key ⇒ fail closed (drives the route's 500).
// ---------------------------------------------------------------------------

#[test]
fn encrypt_required_fails_closed_without_a_key() {
    // No crypto configured: the exact helper connect_booking calls must error,
    // which is what makes the handler return 500 ENCRYPTION_REQUIRED rather than
    // persisting the credential in plaintext.
    let result = encrypt_required(None, PLAINTEXT_PASSWORD);
    assert!(
        result.is_err(),
        "encrypt_required must refuse to encrypt without a configured key (fail closed)"
    );
}

// ---------------------------------------------------------------------------
// (4): rollout tolerance — a legacy plaintext row still reads back correctly,
//      so no destructive data migration is required.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn read_path_tolerates_legacy_plaintext_rows(pool: PgPool) {
    let crypto = with_crypto_key();
    let repo = RentalRepository::new(pool.clone());
    let org_id = seed_org(&pool, "legacy-plain").await;

    // Simulate a row written before BIT-98: plaintext credentials, no enc: prefix.
    repo.create_or_update_booking_connection(
        org_id,
        HOTEL_ID,
        PLAINTEXT_USERNAME,
        PLAINTEXT_PASSWORD,
    )
    .await
    .expect("store legacy plaintext connection");

    let connection = repo
        .find_booking_connection_by_org(org_id)
        .await
        .expect("query connection")
        .expect("connection exists");

    // decrypt_if_available passes plaintext (no enc: prefix) through unchanged,
    // so the read sites keep working during rollout even with a key configured.
    let user = decrypt_if_available(
        crypto.as_ref(),
        connection.access_token.as_deref().expect("access_token"),
    );
    let pass = decrypt_if_available(
        crypto.as_ref(),
        connection.refresh_token.as_deref().expect("refresh_token"),
    );
    assert_eq!(
        user, PLAINTEXT_USERNAME,
        "legacy plaintext username must read through"
    );
    assert_eq!(
        pass, PLAINTEXT_PASSWORD,
        "legacy plaintext password must read through"
    );
}
