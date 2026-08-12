//! Batch 5 — success-path (200/201) integration tests for:
//!   • ai/workflows.rs  — trigger, execution/{id}, execution/{id}/steps, events (4 endpoints)
//!   • ai/llm.rs        — lease templates, listing descriptions, escalation config,
//!                        photo enhancement, voice devices, statistics, requests (16 endpoints)
//!
//! Note: LLM tables (llm_generation_requests, photo_enhancements, voice_assistant_devices,
//! ai_escalation_configs, llm_prompt_templates, generated_listing_descriptions) are NOT in
//! db::MIGRATOR — they are provisioned out-of-band. Tests that touch these tables create
//! minimal schemas inline, matching the pattern in ai_llm_cross_tenant_idor_tests.rs.
//!
//! Skipped (require live external provider calls):
//!   • POST /api/v1/ai/llm/lease/generate     (LLM call)
//!   • POST /api/v1/ai/llm/listing/description (LLM call)
//!   • POST /api/v1/ai/llm/voice/devices       (OAuth exchange)
//!
//! Note: POST /api/v1/ai/llm/chat/enhanced is covered below as a fail-closed
//! (501 NOT_IMPLEMENTED) regression test — the handler no longer fabricates a
//! success payload, so it needs no live provider.

use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::common::{create_authenticated_user_with_org, TestApp, TestUser};

// =============================================================================
// Inline table-creation helpers (LLM tables not in db::MIGRATOR)
// =============================================================================

async fn create_llm_tables(pool: &PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS llm_generation_requests (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            organization_id UUID NOT NULL,
            user_id UUID NOT NULL,
            request_type TEXT NOT NULL,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            prompt_template_id UUID,
            status TEXT NOT NULL DEFAULT 'pending',
            input_data JSONB NOT NULL DEFAULT '{}'::jsonb,
            result JSONB,
            tokens_used INTEGER,
            latency_ms INTEGER,
            cost_cents INTEGER,
            error_message TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            completed_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create llm_generation_requests");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS llm_prompt_templates (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            organization_id UUID,
            request_type TEXT NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            system_prompt TEXT NOT NULL DEFAULT '',
            user_prompt_template TEXT NOT NULL DEFAULT '',
            variables JSONB NOT NULL DEFAULT '[]'::jsonb,
            provider TEXT,
            model TEXT,
            temperature REAL,
            max_tokens INTEGER,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            is_system BOOLEAN NOT NULL DEFAULT FALSE,
            version INTEGER NOT NULL DEFAULT 1,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create llm_prompt_templates");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS photo_enhancements (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            organization_id UUID NOT NULL,
            listing_id UUID,
            user_id UUID NOT NULL,
            original_photo_url TEXT NOT NULL,
            enhanced_photo_url TEXT,
            thumbnail_url TEXT,
            enhancement_type TEXT NOT NULL DEFAULT 'auto_enhance',
            status TEXT NOT NULL DEFAULT 'pending',
            error_message TEXT,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            processing_time_ms INTEGER,
            cost_cents INTEGER,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            completed_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create photo_enhancements");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS ai_escalation_configs (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            organization_id UUID NOT NULL UNIQUE,
            confidence_threshold DOUBLE PRECISION NOT NULL DEFAULT 0.7,
            escalation_email TEXT,
            escalation_webhook_url TEXT,
            auto_escalate_topics JSONB NOT NULL DEFAULT '[]'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create ai_escalation_configs");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS voice_assistant_devices (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            organization_id UUID NOT NULL,
            user_id UUID NOT NULL,
            unit_id UUID,
            platform TEXT NOT NULL,
            device_id TEXT NOT NULL UNIQUE,
            device_name TEXT,
            linked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_used_at TIMESTAMPTZ,
            access_token_encrypted TEXT,
            refresh_token_encrypted TEXT,
            token_expires_at TIMESTAMPTZ,
            is_active BOOLEAN NOT NULL DEFAULT TRUE,
            capabilities JSONB NOT NULL DEFAULT '[]'::jsonb,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create voice_assistant_devices");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS voice_command_history (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            device_id UUID NOT NULL,
            user_id UUID NOT NULL,
            command_text TEXT NOT NULL,
            intent_detected TEXT,
            response_text TEXT NOT NULL DEFAULT '',
            action_taken TEXT,
            success BOOLEAN NOT NULL DEFAULT TRUE,
            error_message TEXT,
            processing_time_ms INTEGER NOT NULL DEFAULT 0,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create voice_command_history");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS generated_listing_descriptions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            organization_id UUID NOT NULL,
            listing_id UUID,
            user_id UUID NOT NULL,
            language TEXT NOT NULL DEFAULT 'en',
            original_description TEXT NOT NULL DEFAULT '',
            property_details JSONB NOT NULL DEFAULT '{}'::jsonb,
            photo_analysis JSONB,
            generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            edited_description TEXT,
            edited_at TIMESTAMPTZ,
            edited_by UUID,
            is_published BOOLEAN NOT NULL DEFAULT FALSE,
            generation_request_id UUID NOT NULL DEFAULT gen_random_uuid(),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create generated_listing_descriptions");
}

// =============================================================================
// Seed helpers
// =============================================================================

async fn seed_workflow(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO workflows (organization_id, name, trigger_type, enabled, trigger_count, created_by)
        VALUES ($1, 'Batch5 Test Workflow', 'manual', TRUE, 0, $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed workflow")
}

async fn seed_workflow_execution(pool: &PgPool, workflow_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO workflow_executions (workflow_id, trigger_event, status, started_at)
        VALUES ($1, '{"type":"manual"}'::jsonb, 'completed', NOW())
        RETURNING id
        "#,
    )
    .bind(workflow_id)
    .fetch_one(pool)
    .await
    .expect("seed workflow execution")
}

async fn seed_photo_enhancement(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO photo_enhancements (organization_id, user_id, original_photo_url, enhancement_type, status)
        VALUES ($1, $2, 'https://example.com/photo.jpg', 'auto_enhance', 'pending')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed photo enhancement")
}

async fn seed_voice_device(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO voice_assistant_devices (organization_id, user_id, platform, device_id, capabilities)
        VALUES ($1, $2, 'google_assistant', $3, '["check_balance","report_fault"]'::jsonb)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .bind(format!("google_assistant_{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("seed voice device")
}

async fn seed_generation_request(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO llm_generation_requests (organization_id, user_id, request_type, provider, model, status, input_data)
        VALUES ($1, $2, 'lease_generation', 'openai', 'gpt-4', 'completed', '{}'::jsonb)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed generation request")
}

async fn seed_prompt_template(pool: &PgPool) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO llm_prompt_templates (request_type, name, system_prompt, user_prompt_template, is_system, provider, model)
        VALUES ('lease_generation', 'Standard Lease Template', 'You are a lease generator.', 'Generate a lease for {{unit}}', TRUE, 'openai', 'gpt-4')
        RETURNING id
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("seed prompt template")
}

async fn user_id_for(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("resolve user id")
}

// =============================================================================
// ai/workflows.rs — remaining 4 endpoints
// =============================================================================

// POST /api/v1/ai/workflows/{id}/trigger → 201
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn trigger_workflow_returns_created(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "wf-trigger").await;
    let session = app.session(token, org_id);
    let user_id = user_id_for(&pool, &user.email).await;

    let workflow_id = seed_workflow(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            session
                .post(&format!("/api/v1/ai/workflows/{workflow_id}/trigger"))
                .json(json!({
                    "workflow_id": workflow_id,
                    "trigger_event": { "type": "manual" },
                    "context": null
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "trigger workflow must return 201; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert!(
        body.get("execution_id").is_some(),
        "trigger response must include execution_id"
    );
}

// GET /api/v1/ai/workflows/executions/{id} → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_workflow_execution_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "wf-exec-get").await;
    let session = app.session(token, org_id);
    let user_id = user_id_for(&pool, &user.email).await;

    let workflow_id = seed_workflow(&pool, org_id, user_id).await;
    let execution_id = seed_workflow_execution(&pool, workflow_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/workflows/executions/{execution_id}"))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get execution must return 200; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert!(
        body.get("execution").is_some(),
        "execution response must include execution key"
    );
}

// GET /api/v1/ai/workflows/executions/{id}/steps → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_workflow_execution_steps_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "wf-exec-steps").await;
    let session = app.session(token, org_id);
    let user_id = user_id_for(&pool, &user.email).await;

    let workflow_id = seed_workflow(&pool, org_id, user_id).await;
    let execution_id = seed_workflow_execution(&pool, workflow_id).await;

    let resp = app
        .execute(
            session
                .get(&format!(
                    "/api/v1/ai/workflows/executions/{execution_id}/steps"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list execution steps must return 200; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert!(
        body.get("steps").is_some(),
        "steps response must include steps key"
    );
}

// POST /api/v1/ai/workflows/events → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn handle_workflow_event_returns_ok(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "wf-events").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/ai/workflows/events")
                .json(json!({
                    "event_type": "fault.reported",
                    "data": { "severity": "low" },
                    "building_id": null
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "handle workflow event must return 200; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert!(
        body.get("triggered_workflows").is_some(),
        "event response must include triggered_workflows count"
    );
}

// =============================================================================
// ai/llm.rs — lease templates (2 endpoints)
// =============================================================================

// GET /api/v1/ai/llm/lease/templates → 200 (empty list OK)
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_lease_templates_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-lease-tpls").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/llm/lease/templates").build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list lease templates must return 200; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert!(
        body.get("templates").is_some(),
        "templates response must include templates key"
    );
}

// GET /api/v1/ai/llm/lease/templates/{id} → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_lease_template_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "llm-lease-tpl-get").await;
    let session = app.session(token, org_id);

    let template_id = seed_prompt_template(&pool).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/llm/lease/templates/{template_id}"))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get lease template must return 200; body={}",
        resp.text()
    );
}

// =============================================================================
// ai/llm.rs — listing descriptions (2 testable endpoints)
// =============================================================================

// GET /api/v1/ai/llm/listing/descriptions/{listing_id} → 200 (empty list OK)
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_listing_descriptions_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-listing-desc").await;
    let session = app.session(token, org_id);
    let listing_id = Uuid::new_v4();

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/llm/listing/descriptions/{listing_id}"))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list listing descriptions must return 200; body={}",
        resp.text()
    );
}

// POST /api/v1/ai/llm/listing/descriptions/{id}/publish → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_listing_description_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-listing-pub").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let session = app.session(token, org_id);

    // Seed a listing description row to publish
    let desc_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO generated_listing_descriptions
            (organization_id, listing_id, user_id, language, original_description,
             property_details, generation_request_id)
        VALUES ($1, $2, $3, 'en', 'A lovely apartment', '{}'::jsonb, gen_random_uuid())
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(Uuid::new_v4())
    .bind(user_id)
    .fetch_one(&pool)
    .await
    .expect("seed listing description");

    let resp = app
        .execute(
            session
                .post(&format!(
                    "/api/v1/ai/llm/listing/descriptions/{desc_id}/publish"
                ))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "publish listing description must return 200; body={}",
        resp.text()
    );
}

// =============================================================================
// ai/llm.rs — escalation config (2 endpoints)
// =============================================================================

// GET /api/v1/ai/llm/chat/escalation-config → 200 (auto-creates default)
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_escalation_config_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "llm-esc-config-get").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/llm/chat/escalation-config").build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get escalation config must return 200; body={}",
        resp.text()
    );
}

// PUT /api/v1/ai/llm/chat/escalation-config → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn update_escalation_config_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "llm-esc-config-put").await;
    // UPDATE … RETURNING * returns RowNotFound when no row exists; seed a default.
    sqlx::query(
        "INSERT INTO ai_escalation_configs (organization_id) VALUES ($1) ON CONFLICT DO NOTHING",
    )
    .bind(org_id)
    .execute(&pool)
    .await
    .expect("seed escalation config");
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .put("/api/v1/ai/llm/chat/escalation-config")
                .json(json!({
                    "confidence_threshold": 0.8,
                    "escalation_email": "escalate@example.com",
                    "escalation_webhook_url": null,
                    "auto_escalate_topics": ["billing", "legal"]
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "update escalation config must return 200; body={}",
        resp.text()
    );
}

// =============================================================================
// ai/llm.rs — enhanced (RAG) chat
// =============================================================================

// POST /api/v1/ai/llm/chat/enhanced → 501 NOT_IMPLEMENTED
//
// Regression guard (code-review-api-handlers-enhanced-chat-stub): this handler
// used to return a 200 success payload with fabricated metrics (a canned echo
// response, a hardcoded confidence of 0.85, tokens_used: 150, empty sources,
// and an escalated flag derived from the fake confidence) despite doing no
// embedding search and no LLM call. It now fails closed with 501 so clients
// never act on invented data. Assert both the status AND the absence of the
// fabricated fields.
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn enhanced_chat_fails_closed_not_implemented(pool: PgPool) {
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "llm-enhanced-chat").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/ai/llm/chat/enhanced")
                .json(json!({
                    "message": "How do I report a leak in my unit?",
                    "language": "en"
                }))
                .build(),
        )
        .await;

    assert_eq!(
        resp.status,
        StatusCode::NOT_IMPLEMENTED,
        "enhanced chat must fail closed with 501, not return fabricated success; body={}",
        resp.text()
    );

    // The old stub leaked these invented metrics in a 200 body — none of them
    // must ever appear in the fail-closed response.
    let body = resp.text();
    assert!(
        !body.contains("\"confidence\""),
        "response must not carry a fabricated confidence score; body={body}"
    );
    assert!(
        !body.contains("\"tokens_used\""),
        "response must not carry a fabricated token count; body={body}"
    );
    assert!(
        !body.contains("\"escalated\""),
        "response must not carry a fabricated escalation flag; body={body}"
    );
}

// =============================================================================
// ai/llm.rs — photo enhancement (3 endpoints)
// =============================================================================

// POST /api/v1/ai/llm/photos/enhance → 201
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn enhance_photo_returns_created(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "llm-photo-enhance").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/ai/llm/photos/enhance")
                .json(json!({
                    "photo_url": "https://example.com/photo.jpg",
                    "listing_id": null,
                    "enhancement_type": "auto_enhance",
                    "options": null
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "enhance photo must return 201; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert!(
        body.get("id").is_some(),
        "enhance photo response must include id"
    );
}

// POST /api/v1/ai/llm/photos/enhance/batch → 201
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn batch_enhance_photos_returns_created(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-photo-batch").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(
            session
                .post("/api/v1/ai/llm/photos/enhance/batch")
                .json(json!({
                    "photo_urls": [
                        "https://example.com/photo1.jpg",
                        "https://example.com/photo2.jpg"
                    ],
                    "listing_id": null,
                    "enhancement_type": "auto_enhance"
                }))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::CREATED,
        "batch enhance photos must return 201; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert_eq!(
        body["total_photos"], 2,
        "batch response must reflect 2 photos"
    );
}

// GET /api/v1/ai/llm/photos/{id} → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_photo_enhancement_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-photo-get").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let session = app.session(token, org_id);

    let enhancement_id = seed_photo_enhancement(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/llm/photos/{enhancement_id}"))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get photo enhancement must return 200; body={}",
        resp.text()
    );
}

// =============================================================================
// ai/llm.rs — voice devices (2 testable endpoints — skip POST due to OAuth)
// =============================================================================

// GET /api/v1/ai/llm/voice/devices → 200 (list; may be empty)
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_voice_devices_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-voice-list").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/llm/voice/devices").build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list voice devices must return 200; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert!(
        body.get("devices").is_some(),
        "voice devices response must include devices key"
    );
}

// GET /api/v1/ai/llm/voice/commands/{device_id} → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_voice_commands_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-voice-cmds").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let session = app.session(token, org_id);

    let device_id = seed_voice_device(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/llm/voice/commands/{device_id}"))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list voice commands must return 200; body={}",
        resp.text()
    );
}

// DELETE /api/v1/ai/llm/voice/devices/{id} → 204
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_voice_device_returns_no_content(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-voice-del").await;
    let user_id = user_id_for(&pool, &user.email).await;
    let session = app.session(token, org_id);

    let device_id = seed_voice_device(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            session
                .delete(&format!("/api/v1/ai/llm/voice/devices/{device_id}"))
                .build(),
        )
        .await;
    assert!(
        resp.status == StatusCode::NO_CONTENT || resp.status == StatusCode::OK,
        "delete voice device must return 204 or 200; body={}",
        resp.text()
    );
}

// =============================================================================
// ai/llm.rs — statistics and requests (3 endpoints)
// =============================================================================

// GET /api/v1/ai/llm/statistics → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_ai_statistics_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-stats").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/llm/statistics").build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get AI statistics must return 200; body={}",
        resp.text()
    );
}

// GET /api/v1/ai/llm/requests → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_generation_requests_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) =
        create_authenticated_user_with_org(&app, &user, "llm-requests-list").await;
    let session = app.session(token, org_id);

    let resp = app
        .execute(session.get("/api/v1/ai/llm/requests").build())
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "list generation requests must return 200; body={}",
        resp.text()
    );
    let body: serde_json::Value = resp.json();
    assert!(
        body.get("requests").is_some(),
        "requests response must include requests key"
    );
}

// GET /api/v1/ai/llm/requests/{id} → 200
#[sqlx::test(migrator = "db::MIGRATOR")]
async fn get_generation_request_returns_ok(pool: PgPool) {
    create_llm_tables(&pool).await;
    let app = TestApp::new(pool.clone()).await;
    let user = TestUser::new();
    let (token, org_id) = create_authenticated_user_with_org(&app, &user, "llm-request-get").await;
    let session = app.session(token, org_id);
    let user_id = user_id_for(&pool, &user.email).await;

    let request_id = seed_generation_request(&pool, org_id, user_id).await;

    let resp = app
        .execute(
            session
                .get(&format!("/api/v1/ai/llm/requests/{request_id}"))
                .build(),
        )
        .await;
    assert_eq!(
        resp.status,
        StatusCode::OK,
        "get generation request must return 200; body={}",
        resp.text()
    );
}
