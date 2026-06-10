//! Regression tests for the cross-tenant IDOR fix on the AI / LLM
//! read-and-mutate-by-id paths (issues #766 and #816).
//!
//! Audit history: several handlers on `/api/v1/ai/*` extracted a verified
//! `RequestPrincipal` but then called repository methods that took only the
//! row id and applied NO `organization_id` predicate, so a caller in org B
//! could read/mutate org A's rows by guessing (or enumerating) the UUID:
//!
//! - `get_lease_template`       → `find_prompt_template(id)`     (template leak)
//! - `get_generation_request`   → `find_generation_request(id)`  (prompt/result/cost leak)
//! - `get_photo_enhancement`    → `find_photo_enhancement(id)`   (photo record leak)
//! - `publish_description`      → `publish_description(id)`       (cross-tenant mutate)
//! - `list_listing_descriptions`→ `list_listing_descriptions(listing_id)` (cross-tenant read)
//! - `provide_feedback`         → `add_feedback(user_id, …)`     (cross-tenant write / training poison)
//! - `generate_lease`           → no `unit_id` ↔ tenant validation
//!
//! The fix adds `_for_org` repository variants (and a `unit_belongs_to_org`
//! check) that thread the principal's `effective_org` into the SQL WHERE
//! clause. The workflow router already used this `_for_org` idiom (#791);
//! these tests pin the same contract for the AI/LLM repo methods.
//!
//! Why repo-layer and not HTTP-layer: `TestApp` mounts the router WITHOUT
//! `host_tenant_middleware`, so `RequestPrincipal` can never resolve an
//! `effective_org` in tests (see `equipment_cross_tenant_idor_tests.rs`'s
//! caveat — those tests can only assert a 4xx rejection and pass even on the
//! pre-fix code). The IDOR fix itself lives in the repository's WHERE clause,
//! so the security contract is asserted there directly: the scoped method
//! must NOT return another org's row even though the raw (pre-fix) query
//! would. Each test also runs the unscoped query to demonstrate the leak the
//! `_for_org` variant closes.
//!
//! These tests use the `ai_*` tables that ship migrations (`00042_create_ai_chat.sql`)
//! so they run against `db::MIGRATOR` deterministically.

use db::models::ProvideFeedback;
use db::repositories::{AiChatRepository, LlmDocumentRepository};
use sqlx::PgPool;
use uuid::Uuid;

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
    .bind(format!("AiIDOR Org {slug}"))
    .bind(format!("ai-idor-org-{slug}"))
    .bind(format!("{slug}@ai-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'AiIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_session(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ai_chat_sessions (organization_id, user_id, title)
        VALUES ($1, $2, 'cross-tenant session') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed session")
}

async fn seed_message(pool: &PgPool, session_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ai_chat_messages (session_id, role, content)
        VALUES ($1, 'user', 'sensitive question') RETURNING id
        "#,
    )
    .bind(session_id)
    .fetch_one(pool)
    .await
    .expect("seed message")
}

// ---------------------------------------------------------------------------
// (1) Chat session read is tenant-scoped — org B cannot read org A's session.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_session_by_id_blocks_cross_tenant_read(pool: PgPool) {
    let repo = AiChatRepository::new(pool.clone());

    let org_a = seed_org(&pool, "sess-a").await;
    let org_b = seed_org(&pool, "sess-b").await;
    let user_a = seed_user(&pool, "sess-a@ai-idor.test").await;
    let session_in_a = seed_session(&pool, org_a, user_a).await;

    // Same-org read succeeds.
    let same_org = repo
        .find_session_by_id(session_in_a, org_a)
        .await
        .expect("query ok");
    assert!(
        same_org.is_some(),
        "org A must be able to read its own chat session"
    );

    // Cross-org read returns None (the IDOR is blocked).
    let cross_org = repo
        .find_session_by_id(session_in_a, org_b)
        .await
        .expect("query ok");
    assert!(
        cross_org.is_none(),
        "org B must NOT be able to read org A's chat session"
    );

    // Demonstrate the leak the org predicate closes: the unscoped lookup the
    // pre-fix handler effectively performed returns the row regardless of org.
    let unscoped: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM ai_chat_sessions WHERE id = $1")
            .bind(session_in_a)
            .fetch_optional(&pool)
            .await
            .expect("query ok");
    assert_eq!(
        unscoped,
        Some(session_in_a),
        "sanity: the unscoped query (the vulnerable pre-fix path) does leak the row"
    );
}

// ---------------------------------------------------------------------------
// (2) Feedback write is tenant-scoped — org B cannot attach feedback to a
//     message in org A's session (write IDOR + training-data poisoning).
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_feedback_for_org_blocks_cross_tenant_write(pool: PgPool) {
    let repo = AiChatRepository::new(pool.clone());

    let org_a = seed_org(&pool, "fb-a").await;
    let org_b = seed_org(&pool, "fb-b").await;
    let user_a = seed_user(&pool, "fb-a@ai-idor.test").await;
    let user_b = seed_user(&pool, "fb-b@ai-idor.test").await;
    let session_in_a = seed_session(&pool, org_a, user_a).await;
    let message_in_a = seed_message(&pool, session_in_a).await;

    // Cross-org feedback (user B in org B targeting org A's message) is refused:
    // the repo returns None and writes nothing.
    let cross = repo
        .add_feedback_for_org(
            user_b,
            org_b,
            ProvideFeedback {
                message_id: message_in_a,
                rating: Some(1),
                helpful: Some(false),
                feedback_text: Some("poison".to_string()),
            },
        )
        .await
        .expect("query ok");
    assert!(
        cross.is_none(),
        "org B must NOT be able to attach feedback to org A's message"
    );

    let count_after_cross: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_training_feedback WHERE message_id = $1")
            .bind(message_in_a)
            .fetch_one(&pool)
            .await
            .expect("query ok");
    assert_eq!(
        count_after_cross, 0,
        "no feedback row may be written by a cross-tenant caller"
    );

    // Same-org feedback (user A in org A) succeeds.
    let same = repo
        .add_feedback_for_org(
            user_a,
            org_a,
            ProvideFeedback {
                message_id: message_in_a,
                rating: Some(5),
                helpful: Some(true),
                feedback_text: Some("great".to_string()),
            },
        )
        .await
        .expect("query ok");
    assert!(
        same.is_some(),
        "org A must be able to attach feedback to its own message"
    );

    let count_after_same: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_training_feedback WHERE message_id = $1")
            .bind(message_in_a)
            .fetch_one(&pool)
            .await
            .expect("query ok");
    assert_eq!(
        count_after_same, 1,
        "exactly one feedback row exists after the same-org write"
    );
}

// ---------------------------------------------------------------------------
// (3) Listing-description list read is tenant-scoped — org B cannot read org
//     A's generated listing descriptions by enumerating a listing id.
//
// `generated_listing_descriptions` does not ship a migration in `db::MIGRATOR`
// (the LLM-document tables are provisioned out-of-band), so we create the
// minimal shape the repository's `SELECT *` maps onto. This pins the IDOR
// contract on `list_listing_descriptions_for_org` directly at the repo layer,
// matching the rationale documented at the top of this file.
// ---------------------------------------------------------------------------

async fn create_generated_listing_descriptions_table(pool: &PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS generated_listing_descriptions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            organization_id UUID NOT NULL,
            listing_id UUID,
            user_id UUID NOT NULL,
            language TEXT NOT NULL,
            original_description TEXT NOT NULL,
            property_details JSONB NOT NULL DEFAULT '{}'::jsonb,
            photo_analysis JSONB,
            generated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            edited_description TEXT,
            edited_at TIMESTAMPTZ,
            edited_by UUID,
            is_published BOOLEAN NOT NULL DEFAULT FALSE,
            generation_request_id UUID NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create generated_listing_descriptions");
}

async fn seed_listing_description(pool: &PgPool, org_id: Uuid, listing_id: Uuid, user_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO generated_listing_descriptions (
            organization_id, listing_id, user_id, language,
            original_description, property_details, generation_request_id
        )
        VALUES ($1, $2, $3, 'sk', 'secret copy', '{}'::jsonb, gen_random_uuid())
        "#,
    )
    .bind(org_id)
    .bind(listing_id)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed listing description");
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_listing_descriptions_for_org_blocks_cross_tenant_read(pool: PgPool) {
    create_generated_listing_descriptions_table(&pool).await;
    let repo = LlmDocumentRepository::new(pool.clone());

    let org_a = seed_org(&pool, "desc-a").await;
    let org_b = seed_org(&pool, "desc-b").await;
    let user_a = seed_user(&pool, "desc-a@ai-idor.test").await;
    let listing_in_a = Uuid::new_v4();
    seed_listing_description(&pool, org_a, listing_in_a, user_a).await;

    // Same-org read returns the row.
    let same_org = repo
        .list_listing_descriptions_for_org(&pool, listing_in_a, org_a)
        .await
        .expect("query ok");
    assert_eq!(
        same_org.len(),
        1,
        "org A must be able to read its own listing descriptions"
    );

    // Cross-org read returns nothing (the IDOR is blocked).
    let cross_org = repo
        .list_listing_descriptions_for_org(&pool, listing_in_a, org_b)
        .await
        .expect("query ok");
    assert!(
        cross_org.is_empty(),
        "org B must NOT be able to read org A's listing descriptions"
    );

    // Demonstrate the leak the org predicate closes: the unscoped lookup the
    // pre-fix handler performed returns the row regardless of org.
    let unscoped = repo
        .list_listing_descriptions(&pool, listing_in_a)
        .await
        .expect("query ok");
    assert_eq!(
        unscoped.len(),
        1,
        "sanity: the unscoped query (the vulnerable pre-fix path) does leak the row"
    );
}
