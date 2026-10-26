//! Regression (Epic 66, Story 66.2): `POST /api/v1/migration/import/upload`
//! must persist the uploaded bytes to object storage and fail closed when no
//! storage service is configured.
//!
//! Before the fix, `upload_import_file` read the multipart body into a
//! discarded `_file_data` buffer, fabricated a `file_path` string that pointed
//! at no real object, and returned `200 OK` with a freshly-created import_job
//! row — silently losing every uploaded file (data loss on bulk import). The
//! downstream validate/approve flow then operated on a storage key that never
//! existed.
//!
//! The default `TestApp` is built with `storage_service: None`, so this test
//! exercises the fail-closed branch end to end: with no storage configured the
//! handler MUST reject the upload with `503 SERVICE_UNAVAILABLE` and MUST NOT
//! create a dangling import_job row.
//!
//! This is the "fails-before-fix" proof: before the fix the stub ignored
//! `storage_service` entirely and returned `200` with a persisted job row; the
//! two assertions below (status == 503, zero job rows) both failed. After the
//! fix they pass.

#![allow(dead_code)]

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user, seed_membership, seed_org, TestApp, TestUser};

/// Mint an access token carrying the `platform_admin` role and a `tenant_id`
/// claim pinned to `org_id` (mirrors `migration_db_tests::mint_platform_admin_token`).
fn mint_platform_admin_token(user_id: Uuid, org_id: Uuid, email: &str) -> String {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::Serialize;

    #[derive(Serialize)]
    struct Claims {
        sub: String,
        exp: i64,
        iat: i64,
        token_type: String,
        tenant_id: String,
        role: String,
        email: String,
        name: String,
    }

    let now = chrono::Utc::now().timestamp();
    encode(
        &Header::default(),
        &Claims {
            sub: user_id.to_string(),
            exp: now + 900,
            iat: now,
            token_type: "access".into(),
            tenant_id: org_id.to_string(),
            role: "platform_admin".into(),
            email: email.into(),
            name: "Migration Upload Admin".into(),
        },
        &EncodingKey::from_secret(
            b"test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes",
        ),
    )
    .expect("mint platform-admin token")
}

/// Register + org-seed a platform-admin member so `RlsConnection` resolves the
/// tenant, and return `(token, org_id)`.
async fn create_platform_admin(app: &TestApp, user: &TestUser, slug: &str) -> (String, Uuid) {
    create_authenticated_user(app, user).await;

    let user_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&user.email)
        .fetch_one(&app.pool)
        .await
        .expect("resolve user id");

    let org_id = seed_org(&app.pool, slug).await;
    seed_membership(&app.pool, org_id, user_id, "platform_admin").await;

    let token = mint_platform_admin_token(user_id, org_id, &user.email);
    (token, org_id)
}

/// Grab a seeded system template id to drive a well-formed upload request.
async fn first_system_template_id(app: &TestApp, token: &str, org_id: Uuid) -> Uuid {
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/v1/migration/templates/system")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .body(Body::empty())
        .unwrap();
    let resp = app.execute(req).await;
    assert_eq!(resp.status, StatusCode::OK, "list system templates");
    let json = resp.json_value();
    Uuid::parse_str(json[0]["id"].as_str().expect("system template id")).unwrap()
}

/// With no storage service configured, the upload must fail closed (503) and
/// leave no import_job row behind.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_import_file_fails_closed_without_storage(pool: PgPool) {
    // Default TestApp carries `storage_service: None`.
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_platform_admin(&app, &user, "migupload").await;
    let template_id = first_system_template_id(&app, &token, org_id).await;

    let csv = "name,address\nTest Building,1 Main St\n";
    let boundary = "bit420migrationuploadboundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"template_id\"\r\n\
         \r\n\
         {template_id}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"buildings.csv\"\r\n\
         Content-Type: text/csv\r\n\
         \r\n\
         {csv}\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/migration/import/upload")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("build multipart upload request");

    let resp = app.execute(req).await;

    // Fails-before-fix proof #1: the stub returned 200 OK here.
    assert_eq!(
        resp.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "upload must fail closed with 503 when storage is unconfigured, got {}",
        resp.status
    );

    // Fails-before-fix proof #2: the stub persisted a dangling job row.
    let job_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM import_jobs WHERE organization_id = $1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        job_count.0, 0,
        "no import_job row may be created when the file was not stored"
    );
}

/// Happy path: with a storage backend configured, the upload must persist the
/// exact uploaded bytes to object storage at the key recorded on the
/// `import_jobs` row, and that key must be downloadable.
///
/// This is the positive counterpart to the fail-closed test above. The
/// fail-closed case only proves the handler rejects when storage is absent; it
/// says nothing about whether the bytes actually reach storage on success. A
/// future refactor could re-introduce the original silent-data-loss bug
/// (upload to the wrong key, upload an empty/discarded buffer, or skip the
/// upload entirely) and the fail-closed test would still pass. This test
/// pins the "persist to storage" contract the PR is named for:
///
/// (a) `200 OK`,
/// (b) an `import_jobs` row exists with a non-empty `file_path`,
/// (c) the exact uploaded CSV bytes were PUT to object storage at that
///     `file_path` (asserted against the recorded S3 request body), and
/// (d) `storage.download(file_path)` returns those same bytes.
///
/// A wiremock `MockServer` stands in for S3 (the same hermetic pattern the
/// document-upload suite uses), so the test needs no live MinIO / dev-stack.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_import_file_persists_bytes_to_storage(pool: PgPool) {
    use api_server::services::{EmailService, JwtService};
    use api_server::state::AppState;
    use integrations::{StorageConfig, StorageService};
    use tower::ServiceExt;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const JWT: &str = "test-secret-key-that-is-at-least-64-characters-long-for-testing-purposes";
    static JWT_ONCE: std::sync::Once = std::sync::Once::new();
    JWT_ONCE.call_once(|| {
        if std::env::var("JWT_SECRET").is_err() {
            std::env::set_var("JWT_SECRET", JWT);
        }
        if std::env::var("RUST_ENV").is_err() {
            std::env::set_var("RUST_ENV", "development");
        }
    });

    let csv = "name,address\nTest Building,1 Main St\n";
    let csv_bytes = csv.as_bytes().to_vec();

    // Wiremock stands in for S3. Accept the PutObject (exactly one upload) and
    // echo the CSV back on any GetObject so `download` round-trips.
    let s3_mock = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path_regex(r".*"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&s3_mock)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r".*"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes(csv_bytes.clone())
                .insert_header("content-type", "text/csv"),
        )
        .mount(&s3_mock)
        .await;

    let config = StorageConfig::new("test-bucket", "us-east-1", "testkey", "testsecret")
        .with_endpoint(s3_mock.uri());
    let storage = StorageService::with_s3_client(config)
        .await
        .expect("StorageService construction must succeed");
    assert!(storage.has_s3_client(), "storage must have an S3 client");

    // Register + seed the platform-admin against the shared pool via a default
    // TestApp, then also use it to resolve a system template id.
    let bare_app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_platform_admin(&bare_app, &user, "miguploadok").await;
    let template_id = first_system_template_id(&bare_app, &token, org_id).await;

    // Build a storage-backed router (the default TestApp carries no storage).
    let email_service = EmailService::new("http://localhost:8080".to_string(), false);
    let jwt_service = JwtService::new(JWT).expect("jwt service");
    let tenant_cache = std::sync::Arc::new(api_core::middleware::TenantResolutionCache::new(
        300, 30, 10_000,
    ));
    let tenant_rate_limiters =
        std::sync::Arc::new(api_core::middleware::TenantRateLimiterSet::new());
    let state = AppState::new(
        pool.clone(),
        email_service,
        jwt_service,
        tenant_cache,
        tenant_rate_limiters,
    )
    .with_storage(storage.clone());
    let router =
        api_server::create_router(state).layer(axum::extract::connect_info::MockConnectInfo(
            std::net::SocketAddr::from(([127, 0, 0, 1], 0)),
        ));

    let boundary = "bit420migrationuploadokboundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"template_id\"\r\n\
         \r\n\
         {template_id}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"buildings.csv\"\r\n\
         Content-Type: text/csv\r\n\
         \r\n\
         {csv}\r\n\
         --{boundary}--\r\n"
    );
    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/migration/import/upload")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("build multipart upload request");

    let resp = router.oneshot(req).await.expect("request must not panic");
    let status = resp.status();
    let resp_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read response body");

    // (a) 200 OK.
    assert_eq!(
        status,
        StatusCode::OK,
        "upload must succeed with storage configured, got {status}. Body: {}",
        String::from_utf8_lossy(&resp_bytes)
    );

    // (b) import_jobs row exists with a non-empty file_path.
    let file_path: String =
        sqlx::query_scalar("SELECT file_path FROM import_jobs WHERE organization_id = $1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .expect("an import_jobs row must exist after a successful upload");
    assert!(
        !file_path.is_empty(),
        "import_jobs.file_path must reference a real storage key, got empty string"
    );

    // (c) the exact CSV bytes were PUT to storage at that key. Inspecting the
    // recorded S3 request guards against the original silent-data-loss bug,
    // where the handler discarded the body and fabricated a key: an empty /
    // wrong body or a mismatched key here would fail.
    let requests = s3_mock
        .received_requests()
        .await
        .expect("wiremock records requests");
    let put = requests
        .iter()
        .find(|r| r.method == Method::PUT)
        .expect("a PutObject request must have been made");
    assert!(
        put.url.path().ends_with(&file_path),
        "upload must PUT to the key recorded on import_jobs.file_path; \
         got PUT {} vs file_path {file_path}",
        put.url.path()
    );
    assert_eq!(
        put.body, csv_bytes,
        "the exact uploaded CSV bytes must be persisted to storage"
    );

    // (d) download(file_path) round-trips the stored bytes.
    let (downloaded, _content_type) = storage
        .download(&file_path)
        .await
        .expect("stored object must be downloadable at file_path");
    assert_eq!(
        downloaded, csv_bytes,
        "download(file_path) must return the exact uploaded bytes"
    );
}

/// Pre-persist rejection must not leave a dangling job row: an otherwise
/// well-formed upload that references a `template_id` which does not exist must
/// be rejected at the validation stage (`400 Template not found`) BEFORE the
/// handler reaches the storage write or the `create_import_job` insert.
///
/// This pins the *ordering* invariant the persist-to-storage fix depends on —
/// validation precedes both the storage upload and the row insert — on a
/// different branch than the fail-closed test. The default `TestApp` carries
/// no storage, so the storage guard (503) sits *after* this 400; if a future
/// refactor moved the row insert ahead of template validation the request
/// would 503/200 and persist a row instead of cleanly 400-ing, and this test
/// would catch it. The zero-row assertion mirrors the fail-closed test's
/// data-integrity contract.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_import_file_rejects_unknown_template_without_creating_job(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_platform_admin(&app, &user, "migupunk").await;

    // A random template id that was never seeded for this org.
    let template_id = Uuid::new_v4();

    let csv = "name,address\nTest Building,1 Main St\n";
    let boundary = "bit420migrationuploadunknownboundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"template_id\"\r\n\
         \r\n\
         {template_id}\r\n\
         --{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"buildings.csv\"\r\n\
         Content-Type: text/csv\r\n\
         \r\n\
         {csv}\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/migration/import/upload")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("build multipart upload request");

    let resp = app.execute(req).await;

    // Rejected at validation (400) before the storage guard (503) is reached.
    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "an unknown template_id must be rejected with 400 before storage, got {}",
        resp.status
    );

    // Data-integrity invariant: no dangling import_job row on early rejection.
    let job_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM import_jobs WHERE organization_id = $1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        job_count.0, 0,
        "no import_job row may be created when the template does not exist"
    );
}

/// Required-field guard: an upload whose multipart payload omits the
/// `template_id` field must be rejected with `400 template_id is required` and,
/// like every other pre-persist rejection, must not create an `import_jobs`
/// row. This covers the earliest rejection branch — it fires before the
/// template lookup, the storage write and the row insert — completing the
/// "reject before persist, leave no dangling row" coverage alongside the
/// unknown-template and fail-closed tests.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn upload_import_file_rejects_missing_template_id_without_creating_job(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_platform_admin(&app, &user, "migupmiss").await;

    // Only the file part — the `template_id` field is deliberately absent.
    let csv = "name,address\nTest Building,1 Main St\n";
    let boundary = "bit420migrationuploadmissingboundary";
    let body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"buildings.csv\"\r\n\
         Content-Type: text/csv\r\n\
         \r\n\
         {csv}\r\n\
         --{boundary}--\r\n"
    );

    let req = Request::builder()
        .method(Method::POST)
        .uri("/api/v1/migration/import/upload")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header("X-Tenant-ID", org_id.to_string())
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(Body::from(body))
        .expect("build multipart upload request");

    let resp = app.execute(req).await;

    assert_eq!(
        resp.status,
        StatusCode::BAD_REQUEST,
        "a missing template_id field must be rejected with 400, got {}",
        resp.status
    );

    let job_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM import_jobs WHERE organization_id = $1")
            .bind(org_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        job_count.0, 0,
        "no import_job row may be created when template_id is missing"
    );
}
