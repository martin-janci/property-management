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
//! IDOR-footgun cleanup: the six unscoped `LlmDocumentRepository` methods
//! named above (`find_generation_request`, `find_prompt_template`,
//! `find_listing_description`, `list_listing_descriptions`,
//! `publish_description`, `find_photo_enhancement`) have been **removed**
//! from the repository entirely — they had zero remaining production call
//! sites (every handler already called the `_for_org` twin) and existed only
//! as a reachable-by-mistake footgun for future callers. The `_for_org`
//! variants are now the only public surface for these lookups.
//!
//! Coverage in this file: chat-session read, feedback write, listing-description
//! list read, listing-description PUBLISH (the cross-tenant mutate), and
//! photo-enhancement read — the full LLM-document IDOR cluster from the
//! `security-llm-doc-idor` plan.
//!
//! Issue #2279 extends this with WITHIN-tenant per-user isolation for the
//! by-session-id AI-chat handlers (`get_session`, `list_messages`,
//! `delete_session`, `send_message`): AI chat sessions are per-user private,
//! so the by-id repo path must scope by `user_id` as well as `organization_id`
//! — otherwise any colleague in the same org can read/delete/post into another
//! member's private session. Tests (1b)–(1d) pin that owner predicate.
//!
//! Why repo-layer and not HTTP-layer: `TestApp` mounts the router WITHOUT
//! `host_tenant_middleware`, so `RequestPrincipal` can never resolve an
//! `effective_org` in tests (see `equipment_cross_tenant_idor_tests.rs`'s
//! caveat — those tests can only assert a 4xx rejection and pass even on the
//! pre-fix code). The IDOR fix itself lives in the repository's WHERE clause,
//! so the security contract is asserted there directly: the scoped method
//! must NOT return another org's row even though the raw (pre-fix) query
//! would. Where the pre-fix unscoped repo method still exists (chat
//! sessions, whose unscoped path is a raw SQL query rather than a repo
//! method — see (1)), the test also demonstrates the leak directly; for the
//! `LlmDocumentRepository` methods the unscoped twins have since been
//! deleted (see module note above), so the `_for_org` assertions alone pin
//! the contract.
//!
//! These tests use the `ai_*` tables that ship migrations (`00042_create_ai_chat.sql`)
//! so they run against `db::MIGRATOR` deterministically.

#![allow(dead_code)]

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

    // Same-org read succeeds (correct org + owner).
    let same_org = repo
        .find_session_by_id(&pool, session_in_a, org_a, user_a)
        .await
        .expect("query ok");
    assert!(
        same_org.is_some(),
        "org A must be able to read its own chat session"
    );

    // Cross-org read returns None (the IDOR is blocked).
    let cross_org = repo
        .find_session_by_id(&pool, session_in_a, org_b, user_a)
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
// (1b) Chat session read is OWNER-scoped WITHIN a tenant (issue #2279) — a
//      colleague in the same org cannot read another member's private session
//      by supplying its UUID. AI chat sessions are per-user private
//      (`create_session` stamps the owner; `list_user_sessions` only returns
//      the caller's own sessions), but the by-id path used to filter by
//      `organization_id` alone. This pins the added `user_id` predicate.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_session_by_id_blocks_within_tenant_cross_user_read(pool: PgPool) {
    let repo = AiChatRepository::new(pool.clone());

    let org = seed_org(&pool, "wt-sess").await;
    let owner = seed_user(&pool, "wt-owner@ai-idor.test").await;
    let attacker = seed_user(&pool, "wt-attacker@ai-idor.test").await;
    let session = seed_session(&pool, org, owner).await;

    // Owner read (correct org + owner) succeeds.
    let as_owner = repo
        .find_session_by_id(&pool, session, org, owner)
        .await
        .expect("query ok");
    assert!(
        as_owner.is_some(),
        "the owning user must be able to read their own chat session"
    );

    // Same-org, different-user read returns None (the within-tenant IDOR is blocked).
    let as_attacker = repo
        .find_session_by_id(&pool, session, org, attacker)
        .await
        .expect("query ok");
    assert!(
        as_attacker.is_none(),
        "issue #2279: a colleague in the same org must NOT read another \
         member's private chat session"
    );

    // Demonstrate the leak the owner predicate closes: the org-only lookup the
    // pre-fix handler performed returns the row regardless of the caller.
    let org_only: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM ai_chat_sessions WHERE id = $1 AND organization_id = $2",
    )
    .bind(session)
    .bind(org)
    .fetch_optional(&pool)
    .await
    .expect("query ok");
    assert_eq!(
        org_only,
        Some(session),
        "sanity: the org-only query (the vulnerable pre-fix path) does leak \
         the row to any member of the same org"
    );
}

// ---------------------------------------------------------------------------
// (1c) Message-transcript read is OWNER-scoped within a tenant (issue #2279) —
//      a colleague in the same org cannot read another member's conversation.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn list_session_messages_blocks_within_tenant_cross_user_read(pool: PgPool) {
    let repo = AiChatRepository::new(pool.clone());

    let org = seed_org(&pool, "wt-msg").await;
    let owner = seed_user(&pool, "wt-msg-owner@ai-idor.test").await;
    let attacker = seed_user(&pool, "wt-msg-attacker@ai-idor.test").await;
    let session = seed_session(&pool, org, owner).await;
    let _msg = seed_message(&pool, session).await;

    // Owner sees the transcript.
    let as_owner = repo
        .list_session_messages(&pool, session, org, owner, 100, 0)
        .await
        .expect("query ok");
    assert_eq!(
        as_owner.len(),
        1,
        "the owning user must be able to read their own session transcript"
    );

    // Same-org, different-user read returns nothing (the IDOR is blocked).
    let as_attacker = repo
        .list_session_messages(&pool, session, org, attacker, 100, 0)
        .await
        .expect("query ok");
    assert!(
        as_attacker.is_empty(),
        "issue #2279: a colleague in the same org must NOT read another \
         member's conversation transcript"
    );

    // Sanity: the org-only join (the pre-fix path) leaks the transcript to any
    // member of the same org.
    let org_only: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM ai_chat_messages m
        JOIN ai_chat_sessions s ON s.id = m.session_id
        WHERE m.session_id = $1 AND s.organization_id = $2
        "#,
    )
    .bind(session)
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("query ok");
    assert_eq!(
        org_only, 1,
        "sanity: the org-only join (the vulnerable pre-fix path) does leak the \
         transcript to any member of the same org"
    );
}

// ---------------------------------------------------------------------------
// (1d) Session DELETE is OWNER-scoped within a tenant (issue #2279) — a
//      colleague in the same org cannot destroy another member's session.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn delete_session_blocks_within_tenant_cross_user_delete(pool: PgPool) {
    let repo = AiChatRepository::new(pool.clone());

    let org = seed_org(&pool, "wt-del").await;
    let owner = seed_user(&pool, "wt-del-owner@ai-idor.test").await;
    let attacker = seed_user(&pool, "wt-del-attacker@ai-idor.test").await;
    let session = seed_session(&pool, org, owner).await;

    // Same-org, different-user delete affects no rows (the IDOR is blocked).
    let attacker_deleted = repo
        .delete_session(&pool, session, org, attacker)
        .await
        .expect("query ok");
    assert!(
        !attacker_deleted,
        "issue #2279: a colleague in the same org must NOT delete another \
         member's session"
    );

    let still_present: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM ai_chat_sessions WHERE id = $1")
            .bind(session)
            .fetch_optional(&pool)
            .await
            .expect("query ok");
    assert_eq!(
        still_present,
        Some(session),
        "the session must survive a cross-user delete attempt"
    );

    // The owner can delete their own session.
    let owner_deleted = repo
        .delete_session(&pool, session, org, owner)
        .await
        .expect("query ok");
    assert!(
        owner_deleted,
        "the owning user must be able to delete their own session"
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
            &pool,
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
            &pool,
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
// (2b) Feedback write is OWNER-scoped WITHIN a tenant (issue #2317) — a
//      colleague in the SAME org cannot attach/overwrite training feedback on
//      another member's private message, nor use the write's success/failure as
//      an existence oracle for org-internal message UUIDs. AI chat messages are
//      per-user private (#2279/#2289); `add_feedback_for_org` used to guard by
//      `s.organization_id` alone. This pins the added `s.user_id = $2`
//      predicate.
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn add_feedback_for_org_blocks_within_tenant_cross_user_write(pool: PgPool) {
    let repo = AiChatRepository::new(pool.clone());

    let org = seed_org(&pool, "fb-wt").await;
    let owner = seed_user(&pool, "fb-wt-owner@ai-idor.test").await;
    let attacker = seed_user(&pool, "fb-wt-attacker@ai-idor.test").await;
    let session = seed_session(&pool, org, owner).await;
    let message = seed_message(&pool, session).await;

    // Same-org, different-user feedback (attacker targeting the owner's private
    // message) is refused: the repo returns None and writes nothing.
    let cross_user = repo
        .add_feedback_for_org(
            &pool,
            attacker,
            org,
            ProvideFeedback {
                message_id: message,
                rating: Some(1),
                helpful: Some(false),
                feedback_text: Some("poison".to_string()),
            },
        )
        .await
        .expect("query ok");
    assert!(
        cross_user.is_none(),
        "issue #2317: a colleague in the same org must NOT attach feedback to \
         another member's private message"
    );

    let count_after_cross: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_training_feedback WHERE message_id = $1")
            .bind(message)
            .fetch_one(&pool)
            .await
            .expect("query ok");
    assert_eq!(
        count_after_cross, 0,
        "no feedback row may be written by a same-org non-owner caller"
    );

    // The owner can attach feedback to their own message.
    let as_owner = repo
        .add_feedback_for_org(
            &pool,
            owner,
            org,
            ProvideFeedback {
                message_id: message,
                rating: Some(5),
                helpful: Some(true),
                feedback_text: Some("great".to_string()),
            },
        )
        .await
        .expect("query ok");
    assert!(
        as_owner.is_some(),
        "the owning user must be able to attach feedback to their own message"
    );

    // Sanity: the org-only EXISTS guard (the vulnerable pre-fix path) would have
    // accepted the attacker's write — the target message IS visible org-wide, so
    // only the owner predicate distinguishes the two callers.
    let org_only_visible: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM ai_chat_messages m
        JOIN ai_chat_sessions s ON s.id = m.session_id
        WHERE m.id = $1 AND s.organization_id = $2
        "#,
    )
    .bind(message)
    .bind(org)
    .fetch_one(&pool)
    .await
    .expect("query ok");
    assert_eq!(
        org_only_visible, 1,
        "sanity: the org-only EXISTS guard (the vulnerable pre-fix path) matches \
         the message for any member of the same org, so it would have let the \
         non-owner write through"
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

    // The pre-fix unscoped `list_listing_descriptions(listing_id)` repo method
    // (the IDOR footgun this demonstrated) has been removed entirely —
    // `list_listing_descriptions_for_org` is now the only reachable public
    // surface for this query, so the leak it closes can no longer be
    // reintroduced by a caller reaching for the "obvious" unscoped name.
}

// ---------------------------------------------------------------------------
// (4) Listing-description PUBLISH is tenant-scoped — org B cannot publish (make
//     public) org A's generated listing description by enumerating its id.
//     This is the highest-severity vector in the cluster: a state-mutating
//     cross-tenant write (the pre-fix `publish_description(id)` ran an
//     `UPDATE … SET is_published = TRUE WHERE id = $1` with no org predicate).
// ---------------------------------------------------------------------------

/// Insert a description and return its id so the publish path can target it.
async fn seed_listing_description_returning_id(
    pool: &PgPool,
    org_id: Uuid,
    listing_id: Uuid,
    user_id: Uuid,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO generated_listing_descriptions (
            organization_id, listing_id, user_id, language,
            original_description, property_details, generation_request_id
        )
        VALUES ($1, $2, $3, 'sk', 'secret copy', '{}'::jsonb, gen_random_uuid())
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(listing_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed listing description")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn publish_description_for_org_blocks_cross_tenant_mutate(pool: PgPool) {
    create_generated_listing_descriptions_table(&pool).await;
    let repo = LlmDocumentRepository::new(pool.clone());

    let org_a = seed_org(&pool, "pub-a").await;
    let org_b = seed_org(&pool, "pub-b").await;
    let user_a = seed_user(&pool, "pub-a@ai-idor.test").await;
    let listing_in_a = Uuid::new_v4();
    let desc_in_a = seed_listing_description_returning_id(&pool, org_a, listing_in_a, user_a).await;

    // Cross-org publish (org B targeting org A's description) returns None and
    // mutates nothing.
    let cross = repo
        .publish_description_for_org(&pool, desc_in_a, org_b)
        .await
        .expect("query ok");
    assert!(
        cross.is_none(),
        "org B must NOT be able to publish org A's listing description"
    );

    let still_unpublished: bool =
        sqlx::query_scalar("SELECT is_published FROM generated_listing_descriptions WHERE id = $1")
            .bind(desc_in_a)
            .fetch_one(&pool)
            .await
            .expect("query ok");
    assert!(
        !still_unpublished,
        "the description must remain unpublished after a cross-tenant publish attempt"
    );

    // Same-org publish (org A) succeeds and flips the flag.
    let same = repo
        .publish_description_for_org(&pool, desc_in_a, org_a)
        .await
        .expect("query ok");
    assert!(
        same.is_some(),
        "org A must be able to publish its own listing description"
    );

    let now_published: bool =
        sqlx::query_scalar("SELECT is_published FROM generated_listing_descriptions WHERE id = $1")
            .bind(desc_in_a)
            .fetch_one(&pool)
            .await
            .expect("query ok");
    assert!(
        now_published,
        "the description must be published after the same-org publish"
    );
}

// ---------------------------------------------------------------------------
// (5) Photo-enhancement read is tenant-scoped — org B cannot read org A's photo
//     enhancement record (original/enhanced URLs, cost, metadata) by guessing
//     its id. Pre-fix `find_photo_enhancement(id)` ran `SELECT * … WHERE id = $1`
//     with no org predicate.
//
// As with `generated_listing_descriptions`, the LLM-document `photo_enhancements`
// table is provisioned out-of-band (no migration in `db::MIGRATOR`), so we
// create the minimal shape the repository's `SELECT *` maps onto.
// ---------------------------------------------------------------------------

async fn create_photo_enhancements_table(pool: &PgPool) {
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
            enhancement_type TEXT NOT NULL,
            status TEXT NOT NULL,
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
}

async fn seed_photo_enhancement(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO photo_enhancements (
            organization_id, user_id, original_photo_url,
            enhancement_type, status
        )
        VALUES ($1, $2, 'https://secret/photo.jpg', 'auto_enhance', 'completed')
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed photo enhancement")
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn find_photo_enhancement_for_org_blocks_cross_tenant_read(pool: PgPool) {
    create_photo_enhancements_table(&pool).await;
    let repo = LlmDocumentRepository::new(pool.clone());

    let org_a = seed_org(&pool, "photo-a").await;
    let org_b = seed_org(&pool, "photo-b").await;
    let user_a = seed_user(&pool, "photo-a@ai-idor.test").await;
    let enh_in_a = seed_photo_enhancement(&pool, org_a, user_a).await;

    // Same-org read returns the row.
    let same_org = repo
        .find_photo_enhancement_for_org(&pool, enh_in_a, org_a)
        .await
        .expect("query ok");
    assert!(
        same_org.is_some(),
        "org A must be able to read its own photo enhancement"
    );

    // Cross-org read returns None (the IDOR is blocked).
    let cross_org = repo
        .find_photo_enhancement_for_org(&pool, enh_in_a, org_b)
        .await
        .expect("query ok");
    assert!(
        cross_org.is_none(),
        "org B must NOT be able to read org A's photo enhancement"
    );

    // The pre-fix unscoped `find_photo_enhancement(id)` repo method (the IDOR
    // footgun this demonstrated) has been removed entirely —
    // `find_photo_enhancement_for_org` is now the only reachable public
    // surface for this query, so the leak it closes can no longer be
    // reintroduced by a caller reaching for the "obvious" unscoped name.
}
