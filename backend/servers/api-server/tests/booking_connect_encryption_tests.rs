//! IG3 ciphertext-at-rest coverage for the Booking.com connect flow (BIT-99 / #1362).
//!
//! # Why this file exists
//!
//! `POST /organizations/{org_id}/booking/connect` (`connect_booking`) used to
//! persist the Booking.com Supply-XML basic-auth `username`/`password` in
//! plaintext, repurposing the `access_token`/`refresh_token` columns of
//! `rental_platform_connections`. BIT-99 hardened the write path to encrypt
//! both values via [`integrations::encrypt_required`] before persisting them,
//! failing closed when `INTEGRATION_ENCRYPTION_KEY` is absent — mirroring the
//! mandatory Airbnb token encryption (Issue #765). Every `BookingClient`
//! construction site now decrypts via [`integrations::decrypt_if_available`].
//!
//! These tests pin the storage-layer contract that the route depends on:
//!
//! - the persisted row holds AES-GCM ciphertext (the `enc:` envelope), never
//!   the plaintext password (the regression BIT-99 fixes);
//! - the read path (`decrypt_if_available`, used at every `BookingClient`
//!   construction site) round-trips the ciphertext back to the original
//!   credentials, and transparently passes legacy plaintext rows through so no
//!   data backfill migration is required;
//! - `encrypt_required` fails closed when no key is configured, so the route
//!   cannot silently fall back to persisting plaintext.
//!
//! # Scope / known limitations
//!
//! 1. **No HTTP handler call.** `connect_booking` makes an unconditional
//!    outbound call to the compile-time-constant Booking Supply-XML host
//!    (`fetch_property`) before it stores anything; that URL is not injectable,
//!    so a full route-level test would issue a real network request (the same
//!    reason `booking_oauth_routes_tests.rs` punts the live happy path). The
//!    encrypt step the handler performs is exactly
//!    `encrypt_required(crypto, &request.username/password)` — exercised here.
//! 2. **Row inserted directly, not via `create_or_update_booking_connection`.**
//!    That repo upsert targets `ON CONFLICT (organization_id, platform) WHERE
//!    unit_id = <nil>`, which infers a partial unique index that only ships in
//!    a later migration than this crate carries on `dev` (≤ 00180). Seeding a
//!    real unit and inserting the row directly keeps this at-rest test
//!    independent of that index; the encryption contract under test is
//!    orthogonal to the upsert's conflict target.

#![allow(dead_code)]

use db::repositories::RentalRepository;
use integrations::{decrypt_if_available, encrypt_required, IntegrationCrypto};
use sqlx::PgPool;
use uuid::Uuid;

/// A deterministic 32-byte (64 hex char) AES-256 key for tests. Never used in
/// production — `connect_booking` reads the real key from
/// `INTEGRATION_ENCRYPTION_KEY` via `IntegrationCrypto::try_from_env`.
const TEST_KEY_HEX: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";

const HOTEL_ID: &str = "123456";
const PLAINTEXT_USERNAME: &str = "booking-supply-user";
const PLAINTEXT_PASSWORD: &str = "sup3r-s3cret-supply-xml-passw0rd";

/// Seed a minimal active org → building → unit and return `(org_id, unit_id)`.
async fn seed_org_unit(pool: &PgPool, tag: &str) -> (Uuid, Uuid) {
    let org_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO organizations (name, slug, contact_email, status)
           VALUES ($1, $2, $3, 'active') RETURNING id"#,
    )
    .bind(format!("Test Org {tag}"))
    .bind(format!("test-org-{tag}-{}", Uuid::new_v4()))
    .bind(format!("{tag}-{}@test-org.internal", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed org");

    let building_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO buildings (organization_id, name, street, city, postal_code, country, status)
           VALUES ($1, 'Test Building', 'Street 1', 'City', '00000', 'Country', 'active')
           RETURNING id"#,
    )
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed building");

    let unit_id: Uuid = sqlx::query_scalar(
        "INSERT INTO units (building_id, designation, floor) VALUES ($1, 'A1', 1) RETURNING id",
    )
    .bind(building_id)
    .fetch_one(pool)
    .await
    .expect("seed unit");

    (org_id, unit_id)
}

/// Insert a `booking` connection row storing the credentials exactly as
/// `connect_booking` does: `encrypt_required(crypto, ..)` for both fields.
async fn insert_encrypted_booking_connection(
    pool: &PgPool,
    org_id: Uuid,
    unit_id: Uuid,
    crypto: &IntegrationCrypto,
) {
    let stored_username =
        encrypt_required(Some(crypto), PLAINTEXT_USERNAME).expect("encrypt username");
    let stored_password =
        encrypt_required(Some(crypto), PLAINTEXT_PASSWORD).expect("encrypt password");

    sqlx::query(
        r#"INSERT INTO rental_platform_connections (
               organization_id, unit_id, platform, external_property_id,
               access_token, refresh_token, is_active
           )
           VALUES ($1, $2, 'booking', $3, $4, $5, true)"#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(HOTEL_ID)
    .bind(&stored_username)
    .bind(&stored_password)
    .execute(pool)
    .await
    .expect("insert booking connection");
}

/// IG3: the stored connection row must hold ciphertext, not the plaintext
/// password — and the read path must round-trip it back.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn booking_connect_persists_ciphertext_not_plaintext(pool: PgPool) {
    let crypto = IntegrationCrypto::new(TEST_KEY_HEX).expect("valid test key");
    let (org_id, unit_id) = seed_org_unit(&pool, "bit99-cipher").await;
    insert_encrypted_booking_connection(&pool, org_id, unit_id, &crypto).await;

    // Read the raw row straight from the table — this is the at-rest state an
    // attacker with DB access would see.
    let (raw_username, raw_password): (Option<String>, Option<String>) = sqlx::query_as(
        r#"SELECT access_token, refresh_token
           FROM rental_platform_connections
           WHERE organization_id = $1 AND platform = 'booking'"#,
    )
    .bind(org_id)
    .fetch_one(&pool)
    .await
    .expect("fetch stored booking connection row");

    let raw_username = raw_username.expect("username column populated");
    let raw_password = raw_password.expect("password column populated");

    // The regression BIT-99 fixes: plaintext credentials must never appear.
    assert_ne!(
        raw_password, PLAINTEXT_PASSWORD,
        "password column must not hold plaintext"
    );
    assert_ne!(
        raw_username, PLAINTEXT_USERNAME,
        "username column must not hold plaintext"
    );
    assert!(
        !raw_password.contains(PLAINTEXT_PASSWORD),
        "password ciphertext must not embed the plaintext password"
    );
    assert!(
        raw_password.starts_with("enc:"),
        "password column must carry the `enc:` AES-GCM envelope, got: {raw_password}"
    );
    assert!(
        raw_username.starts_with("enc:"),
        "username column must carry the `enc:` AES-GCM envelope, got: {raw_username}"
    );

    // The read path used at every BookingClient construction site, exercised
    // through the real `find_booking_connection_by_org` repo method, must
    // recover the original credentials.
    let repo = RentalRepository::new(pool.clone());
    let conn = repo
        .find_booking_connection_by_org(org_id)
        .await
        .expect("find booking connection")
        .expect("connection exists");

    assert_eq!(
        decrypt_if_available(Some(&crypto), &conn.access_token.unwrap_or_default()),
        PLAINTEXT_USERNAME,
        "username must decrypt back to the original"
    );
    assert_eq!(
        decrypt_if_available(Some(&crypto), &conn.refresh_token.unwrap_or_default()),
        PLAINTEXT_PASSWORD,
        "password must decrypt back to the original"
    );
}

/// Rollout tolerance: a pre-existing plaintext row (no `enc:` prefix) reads back
/// unchanged through `decrypt_if_available`, so the read path is safe before any
/// backfill and no data migration is required.
#[test]
fn read_path_passes_legacy_plaintext_through() {
    let crypto = IntegrationCrypto::new(TEST_KEY_HEX).expect("valid test key");
    assert_eq!(
        decrypt_if_available(Some(&crypto), PLAINTEXT_PASSWORD),
        PLAINTEXT_PASSWORD,
        "legacy plaintext must pass through unchanged"
    );
}

/// The write path must fail closed: with no key configured, `encrypt_required`
/// errors instead of letting the route persist plaintext credentials.
#[test]
fn booking_connect_encryption_fails_closed_without_key() {
    let err = encrypt_required(None, PLAINTEXT_PASSWORD)
        .expect_err("encryption must fail closed when no key is configured");
    // `connect_booking` maps this error to a fail-closed `ENCRYPTION_REQUIRED`
    // response rather than storing plaintext.
    assert!(
        matches!(err, integrations::CryptoError::KeyNotConfigured(_)),
        "expected KeyNotConfigured, got: {err:?}"
    );
}
