//! Tests for guest ID-document upload + OCR extract (Epic 18, Story 18.2, #1687).
//!
//! Covers the Stage-A seam end-to-end against the default app state (no S3
//! storage, stub OCR provider — fully offline-safe):
//!   * upload → 201, sets the guest `id_document_url` to an `id-documents/` key
//!     and records a `rental_guest_id_documents` row;
//!   * extract → 501 `OCR_NOT_CONFIGURED` (the not-configured stub);
//!   * cross-org guest → 404 (org-scoped lookup, no existence leak);
//!   * unsupported MIME → 400; oversize → 413;
//!   * non-manager → 403 on BOTH endpoints (manager-role gate).
//!
//! Auth mirrors `rental_connection_token_leak_tests.rs`: a minted access JWT +
//! `X-Tenant-ID` header, with `organization_members` seeded so the manager gate
//! resolves a real role.

#![allow(dead_code)]

use axum::body::Body;
use axum::http::{header, Method, Request, StatusCode};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{seed_membership, seed_org, TestApp};

// Must match `TestConfig::default().jwt_secret` in tests/common/mod.rs.
const JWT_SECRET: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";

const MULTIPART_BOUNDARY: &str = "----ppt1687boundary";

#[derive(Serialize)]
struct AccessClaims {
    sub: Uuid,
    exp: i64,
    iat: i64,
    token_type: String,
    tenant_id: Option<Uuid>,
    role: Option<String>,
    email: String,
    name: String,
}

fn mint_access_token(user_id: Uuid, tenant_id: Uuid) -> String {
    let now = Utc::now();
    let claims = AccessClaims {
        sub: user_id,
        iat: now.timestamp(),
        exp: (now + Duration::hours(1)).timestamp(),
        token_type: "access".to_string(),
        tenant_id: Some(tenant_id),
        role: Some("manager".to_string()),
        email: "guest-iddoc@test.local".to_string(),
        name: "Guest IdDoc Test".to_string(),
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .expect("mint access token")
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'Guest IdDoc User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_building(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building")
}

async fn seed_unit(pool: &PgPool, building_id: Uuid, designation: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (building_id, designation, floor)
        VALUES ($1, $2, 1) RETURNING id
        "#,
    )
    .bind(building_id)
    .bind(designation)
    .fetch_one(pool)
    .await
    .expect("seed unit")
}

async fn seed_booking(pool: &PgPool, org_id: Uuid, unit_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_bookings (
            organization_id, unit_id, platform, guest_name, check_in, check_out
        )
        VALUES ($1, $2, 'airbnb', 'Booking Guest', CURRENT_DATE, CURRENT_DATE + 2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .fetch_one(pool)
    .await
    .expect("seed booking")
}

async fn seed_guest(pool: &PgPool, org_id: Uuid, booking_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO rental_guests (
            organization_id, booking_id, first_name, last_name
        )
        VALUES ($1, $2, 'Jane', 'Doe')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(booking_id)
    .fetch_one(pool)
    .await
    .expect("seed guest")
}

/// Seed a full org + manager user + booking + guest, returning
/// `(token, org_id, guest_id)`.
async fn seed_manager_with_guest(pool: &PgPool, slug: &str) -> (String, Uuid, Uuid) {
    let org = seed_org(pool, slug).await;
    let user = seed_user(pool, &format!("{slug}-mgr-{}@iddoc.test", Uuid::new_v4())).await;
    seed_membership(pool, org, user, "manager").await;
    let building = seed_building(pool, org, slug).await;
    let unit = seed_unit(pool, building, &format!("{slug}-U1")).await;
    let booking = seed_booking(pool, org, unit).await;
    let guest = seed_guest(pool, org, booking).await;
    (mint_access_token(user, org), org, guest)
}

// ---------------------------------------------------------------------------
// Request builders
// ---------------------------------------------------------------------------

/// Build a `multipart/form-data` body with a single `file` part.
fn multipart_file(filename: &str, content_type: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{MULTIPART_BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{MULTIPART_BOUNDARY}--\r\n").as_bytes());
    body
}

fn upload_request(guest_id: Uuid, token: &str, org: Uuid, body: Vec<u8>) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v1/rentals/guests/{guest_id}/id-document"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

fn extract_request(guest_id: Uuid, token: &str, org: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri(format!(
            "/api/v1/rentals/guests/{guest_id}/id-document/extract"
        ))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .body(Body::empty())
        .unwrap()
}

// ---------------------------------------------------------------------------
// (1) upload → 201, sets id-documents/ url + records a row
// ---------------------------------------------------------------------------

#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-574)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_succeeds_sets_url_and_records_row(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org, guest) = seed_manager_with_guest(&pool, "upok").await;

    let body = multipart_file(
        "passport.png",
        "image/png",
        &[0x89, b'P', b'N', b'G', 1, 2, 3],
    );
    let resp = app.execute(upload_request(guest, &token, org, body)).await;

    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "upload must be 201, got {}: {}",
        resp.status,
        resp.text()
    );

    // The guest record now points at an `id-documents/` storage key.
    let url: Option<String> =
        sqlx::query_scalar("SELECT id_document_url FROM rental_guests WHERE id = $1")
            .bind(guest)
            .fetch_one(&pool)
            .await
            .expect("read guest url");
    let url = url.expect("id_document_url should be set");
    assert!(
        url.starts_with("id-documents/"),
        "id_document_url should be under id-documents/, got {url}"
    );

    // And exactly one document row exists for this guest.
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM rental_guest_id_documents WHERE guest_id = $1 AND organization_id = $2",
    )
    .bind(guest)
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("count docs");
    assert_eq!(count, 1, "exactly one id-document row should be recorded");
}

// ---------------------------------------------------------------------------
// (2) extract against the stub → 501 OCR_NOT_CONFIGURED
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn extract_with_stub_returns_501(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org, guest) = seed_manager_with_guest(&pool, "extract").await;

    // Upload a document first so the lookup finds a row (not a 404).
    let body = multipart_file("id.jpg", "image/jpeg", &[0xFF, 0xD8, 0xFF, 1, 2]);
    let up = app.execute(upload_request(guest, &token, org, body)).await;
    assert_eq!(
        up.status,
        StatusCode::CREATED,
        "upload precondition: {}",
        up.text()
    );

    let resp = app.execute(extract_request(guest, &token, org)).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_IMPLEMENTED,
        "extract with stub must be 501, got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("OCR_NOT_CONFIGURED"),
        "501 body should carry the OCR_NOT_CONFIGURED code: {}",
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// (2b) extract with no document on file → 404
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn extract_without_document_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org, guest) = seed_manager_with_guest(&pool, "nodoc").await;

    let resp = app.execute(extract_request(guest, &token, org)).await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "extract with no document must be 404, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// (3) cross-org guest → 404 on upload
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_for_cross_org_guest_is_404(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Org A owns the guest; an org-B manager tries to upload to it.
    let (_token_a, _org_a, guest_a) = seed_manager_with_guest(&pool, "xorg-a").await;

    let org_b = seed_org(&pool, "xorg-b").await;
    let user_b = seed_user(&pool, &format!("xorg-b-{}@iddoc.test", Uuid::new_v4())).await;
    seed_membership(&pool, org_b, user_b, "manager").await;
    let token_b = mint_access_token(user_b, org_b);

    let body = multipart_file("p.png", "image/png", &[1, 2, 3]);
    let resp = app
        .execute(upload_request(guest_a, &token_b, org_b, body))
        .await;
    assert_eq!(
        resp.status,
        StatusCode::NOT_FOUND,
        "cross-org upload must be 404, got {}: {}",
        resp.status,
        resp.text()
    );

    // No row should have been recorded for the victim guest.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rental_guest_id_documents WHERE guest_id = $1")
            .bind(guest_a)
            .fetch_one(&pool)
            .await
            .expect("count docs");
    assert_eq!(count, 0, "cross-org upload must not record a document");
}

// ---------------------------------------------------------------------------
// (4) unsupported MIME → 400
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_unsupported_mime_is_400(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org, guest) = seed_manager_with_guest(&pool, "badmime").await;

    let body = multipart_file("evil.exe", "application/octet-stream", &[1, 2, 3]);
    let resp = app.execute(upload_request(guest, &token, org, body)).await;
    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "unsupported MIME must be 400, got {}: {}",
        resp.status,
        resp.text()
    );
}

// ---------------------------------------------------------------------------
// (5) oversize → 413
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_oversize_is_413(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org, guest) = seed_manager_with_guest(&pool, "big").await;

    // 10 MiB + 1 byte of PNG-typed payload.
    let big = vec![0u8; 10 * 1024 * 1024 + 1];
    let body = multipart_file("huge.png", "image/png", &big);
    let resp = app.execute(upload_request(guest, &token, org, body)).await;
    assert_eq!(
        resp.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "oversize upload must be 413, got {}",
        resp.status
    );
}

// ---------------------------------------------------------------------------
// (6) non-manager → 403 on BOTH endpoints
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn non_manager_is_forbidden_on_both_endpoints(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Seed a guest in an org, but authenticate as a non-manager (resident).
    let org = seed_org(&pool, "noperm").await;
    let resident = seed_user(&pool, &format!("noperm-{}@iddoc.test", Uuid::new_v4())).await;
    seed_membership(&pool, org, resident, "resident").await;
    let building = seed_building(&pool, org, "noperm").await;
    let unit = seed_unit(&pool, building, "NP-1").await;
    let booking = seed_booking(&pool, org, unit).await;
    let guest = seed_guest(&pool, org, booking).await;
    let token = mint_access_token(resident, org);

    // upload → 403
    let body = multipart_file("p.png", "image/png", &[1, 2, 3]);
    let up = app.execute(upload_request(guest, &token, org, body)).await;
    assert_eq!(
        up.status,
        StatusCode::FORBIDDEN,
        "non-manager upload must be 403, got {}: {}",
        up.status,
        up.text()
    );

    // extract → 403
    let ex = app.execute(extract_request(guest, &token, org)).await;
    assert_eq!(
        ex.status,
        StatusCode::FORBIDDEN,
        "non-manager extract must be 403, got {}: {}",
        ex.status,
        ex.text()
    );

    // And the forbidden upload recorded nothing.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rental_guest_id_documents WHERE guest_id = $1")
            .bind(guest)
            .fetch_one(&pool)
            .await
            .expect("count docs");
    assert_eq!(count, 0, "forbidden upload must not record a document");
}

// ---------------------------------------------------------------------------
// (7) PII audit logging on upload + extract (#1760/#1783)
//
// IG3: on dev neither handler writes an audit_log row, so both COUNT(*)
// assertions fail (the `rental_guest_id_document_*` enum values don't even
// exist pre-migration). After the fix, upload and extract each emit an
// `audit_logs` row scoped to the org + guest.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_and_extract_write_audit_log_rows(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org, guest) = seed_manager_with_guest(&pool, "audit").await;

    // Upload a valid PNG-signed document.
    let body = multipart_file(
        "passport.png",
        "image/png",
        &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n', 1, 2, 3],
    );
    let up = app.execute(upload_request(guest, &token, org, body)).await;
    assert_eq!(
        up.status,
        StatusCode::CREATED,
        "upload precondition: {}",
        up.text()
    );

    // Trigger an extract (stub → 501, but the access is still audited).
    let ex = app.execute(extract_request(guest, &token, org)).await;
    assert_eq!(
        ex.status,
        StatusCode::NOT_IMPLEMENTED,
        "extract precondition (stub 501): {}",
        ex.text()
    );

    // An upload audit row scoped to this org exists.
    let upload_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs \
         WHERE org_id = $1 AND action::text = 'rental_guest_id_document_upload'",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("count upload audit rows");
    assert_eq!(
        upload_rows, 1,
        "ID-document upload must write exactly one audit_log row"
    );

    // An extract (PII access) audit row scoped to this org exists.
    let extract_rows: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_logs \
         WHERE org_id = $1 AND action::text = 'rental_guest_id_document_extract'",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("count extract audit rows");
    assert_eq!(
        extract_rows, 1,
        "ID-document extract must write exactly one audit_log row"
    );

    // The upload row carries the guest_id in details so the access is
    // attributable to a specific guest.
    let detail_guest: Option<Uuid> = sqlx::query_scalar(
        "SELECT (details->>'guest_id')::uuid FROM audit_logs \
         WHERE org_id = $1 AND action::text = 'rental_guest_id_document_upload'",
    )
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("read upload audit details");
    assert_eq!(
        detail_guest,
        Some(guest),
        "upload audit row should record the guest_id"
    );
}

// ---------------------------------------------------------------------------
// (8) magic-byte content sniffing (#1760/#1783)
//
// IG3: a payload whose declared Content-Type is image/png but whose bytes are
// NOT a PNG/JPEG/PDF signature is stored on dev (only the declared header is
// checked → 201). After the fix it is rejected 400 UNSUPPORTED_FILE_TYPE and
// no row is recorded.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_rejects_content_type_spoof(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let (token, org, guest) = seed_manager_with_guest(&pool, "spoof").await;

    // Declared image/png, but the bytes are a GIF signature (not allowed).
    let body = multipart_file("not-really.png", "image/png", b"GIF89a\x00\x01evil");
    let resp = app.execute(upload_request(guest, &token, org, body)).await;
    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "content-type spoof must be 400, got {}: {}",
        resp.status,
        resp.text()
    );
    assert!(
        resp.text().contains("UNSUPPORTED_FILE_TYPE"),
        "400 body should carry UNSUPPORTED_FILE_TYPE: {}",
        resp.text()
    );

    // The spoofed upload recorded nothing and left the guest unlinked.
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rental_guest_id_documents WHERE guest_id = $1")
            .bind(guest)
            .fetch_one(&pool)
            .await
            .expect("count docs");
    assert_eq!(count, 0, "spoofed upload must not record a document");
}

// ---------------------------------------------------------------------------
// (9) update_guest write-path is manager-gated (#1783)
//
// IG3: on dev PUT /rentals/guests/{id} is TenantExtractor-only, so a
// non-manager member can write identity PII → 200. After the fix it is 403 and
// the guest's id_number is unchanged.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_guest_is_manager_gated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    // Seed a guest in an org, but authenticate as a non-manager (resident).
    let org = seed_org(&pool, "updperm").await;
    let resident = seed_user(&pool, &format!("updperm-{}@iddoc.test", Uuid::new_v4())).await;
    seed_membership(&pool, org, resident, "resident").await;
    let building = seed_building(&pool, org, "updperm").await;
    let unit = seed_unit(&pool, building, "UP-1").await;
    let booking = seed_booking(&pool, org, unit).await;
    let guest = seed_guest(&pool, org, booking).await;
    let token = mint_access_token(resident, org);

    let req = Request::builder()
        .method(Method::PUT)
        .uri(format!("/api/v1/rentals/guests/{guest}"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"id_number":"AB1234567"}"#))
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(
        resp.status,
        StatusCode::FORBIDDEN,
        "non-manager guest update must be 403, got {}: {}",
        resp.status,
        resp.text()
    );

    // The forbidden write must not have changed the guest's identity PII.
    let id_number: Option<String> =
        sqlx::query_scalar("SELECT id_number FROM rental_guests WHERE id = $1")
            .bind(guest)
            .fetch_one(&pool)
            .await
            .expect("read guest id_number");
    assert_eq!(
        id_number, None,
        "forbidden update must not write the guest id_number"
    );
}

// ---------------------------------------------------------------------------
// (8) Guest/booking PII READ paths are manager-gated (#1766)
// ---------------------------------------------------------------------------

fn get_request(uri: &str, token: &str, org: Uuid) -> Request<Body> {
    Request::builder()
        .method(Method::GET)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org.to_string())
        .body(Body::empty())
        .unwrap()
}

/// The four guest/booking-PII read endpoints
/// (`list_bookings` / `get_booking` / `get_booking_with_guests` / `get_guest`)
/// must reject a non-manager member with 403 — parity with the already-gated
/// write/upload paths — while a manager in the same org is allowed (2xx),
/// proving the rejection is the role gate and not a no-context pass.
#[ignore = "BIT-351 quarantine: schema/route not implemented (BIT-574)"]
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn guest_booking_pii_reads_are_manager_gated(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;

    let org = seed_org(&pool, "piiread").await;
    let building = seed_building(&pool, org, "piiread").await;
    let unit = seed_unit(&pool, building, "PR-1").await;
    let booking = seed_booking(&pool, org, unit).await;
    let guest = seed_guest(&pool, org, booking).await;

    let manager = seed_user(&pool, &format!("piiread-mgr-{}@iddoc.test", Uuid::new_v4())).await;
    seed_membership(&pool, org, manager, "manager").await;
    let mgr_token = mint_access_token(manager, org);

    let resident = seed_user(&pool, &format!("piiread-res-{}@iddoc.test", Uuid::new_v4())).await;
    seed_membership(&pool, org, resident, "resident").await;
    let res_token = mint_access_token(resident, org);

    let uris = [
        "/api/v1/rentals/bookings".to_string(),
        format!("/api/v1/rentals/bookings/{booking}"),
        format!("/api/v1/rentals/bookings/{booking}/guests"),
        format!("/api/v1/rentals/guests/{guest}"),
    ];

    for uri in &uris {
        // Non-manager member → 403.
        let res = app.execute(get_request(uri, &res_token, org)).await;
        assert_eq!(
            res.status,
            StatusCode::FORBIDDEN,
            "non-manager read of {uri} must be 403, got {}: {}",
            res.status,
            res.text()
        );

        // Manager → passes the role gate AND the endpoint's happy path. We now
        // assert a strict 2xx: the underlying repo query for each of the four
        // PII read paths must actually succeed for a manager in the org. The
        // latent `u.name` schema-drift 500 in `list_bookings` (and its sibling
        // rental queries) is fixed — the `units` identity column is
        // `designation`, not `name` — so this pins the real happy path rather
        // than merely "not 403" (issue #2028).
        let mgr = app.execute(get_request(uri, &mgr_token, org)).await;
        assert!(
            mgr.status.is_success(),
            "manager read of {uri} must succeed (2xx), got {}: {}",
            mgr.status,
            mgr.text()
        );
    }
}
