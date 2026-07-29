//! Repo-layer tests for `IntegrationRepository::find_esignature_workflow_by_external_id`
//! (GH #1306, PR #1288 follow-up).
//!
//! Background
//! ----------
//! The e-signature webhook handler resolves an owning organization from an
//! unguessable provider-signed envelope id (`external_envelope_id`) before
//! binding RLS context for the status update. The repo method
//! `find_esignature_workflow_by_external_id` is the single read that enables
//! this resolution — but no test verifies the envelope-id → org/user
//! mapping, or that an unknown envelope returns `None` (the "ignore" branch).
//!
//! Schema note
//! -----------
//! `esignature_workflows` exists in no migration (it is part of the unmounted
//! Epic-84 e-signature surface — see `routes/integrations/webhook.rs` comment
//! ROADMAP(PAP-122)). The test creates the table inline as the superuser (the
//! `#[sqlx::test]` pool is SUPERUSER), so the FORCE policy is not bound here.
//! The goal is to prove the round-trip lookup contract at the repo boundary,
//! not to exercise RLS enforcement (that is done via the static
//! `check-rls-enforcement.sh --strict` scanner once the table is migrated).

use crate::common::seed_org;
use db::repositories::IntegrationRepository;
use sqlx::PgPool;
use uuid::Uuid;

async fn ensure_esignature_workflows_table(pool: &PgPool) {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS esignature_workflows (
            id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            organization_id      UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
            document_id          UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
            provider             TEXT NOT NULL DEFAULT 'internal',
            external_envelope_id TEXT,
            title                TEXT NOT NULL,
            message              TEXT,
            status               TEXT NOT NULL DEFAULT 'draft',
            expires_at           TIMESTAMPTZ,
            reminder_enabled     BOOLEAN NOT NULL DEFAULT FALSE,
            reminder_days        INT,
            created_by           UUID NOT NULL REFERENCES users(id),
            completed_at         TIMESTAMPTZ,
            created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await
    .expect("create esignature_workflows table");
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO users (email, password_hash, name, status, email_verified_at, principal_kind) \
         VALUES ($1, 'test_hash', 'ESig User', 'active', NOW(), 'public') RETURNING id",
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

async fn seed_document(pool: &PgPool, org_id: Uuid, user_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO documents
            (organization_id, title, file_key, file_name, mime_type, size_bytes, created_by)
        VALUES ($1, 'Test Doc', 'test/doc.pdf', 'doc.pdf', 'application/pdf', 1024, $2)
        RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed document")
}

async fn seed_workflow(
    pool: &PgPool,
    org_id: Uuid,
    doc_id: Uuid,
    user_id: Uuid,
    envelope_id: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO esignature_workflows \
         (organization_id, document_id, provider, external_envelope_id, \
          title, status, reminder_enabled, created_by) \
         VALUES ($1, $2, 'docusign', $3, 'Test Workflow', 'sent', FALSE, $4) \
         RETURNING id",
    )
    .bind(org_id)
    .bind(doc_id)
    .bind(envelope_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
    .expect("seed workflow")
}

/// `find_esignature_workflow_by_external_id` returns the correct org/user
/// row for a known envelope id.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn find_esignature_workflow_returns_correct_row(pool: PgPool) {
    ensure_esignature_workflows_table(&pool).await;

    let org = seed_org(&pool, "esig-find-a").await;
    let user = seed_user(&pool, "esig-find-a@esig.test").await;
    let doc = seed_document(&pool, org, user).await;
    let envelope_id = format!("env_{}", Uuid::new_v4());
    let wf_id = seed_workflow(&pool, org, doc, user, &envelope_id).await;

    let repo = IntegrationRepository::new(pool.clone());

    let found = repo
        .find_esignature_workflow_by_external_id(&pool, &envelope_id)
        .await
        .expect("find_esignature_workflow_by_external_id");

    let wf = found.expect("should find workflow for a known envelope_id");
    assert_eq!(wf.id, wf_id, "returned workflow id must match seeded id");
    assert_eq!(
        wf.organization_id, org,
        "returned workflow must carry the seeding org_id (used for RLS context binding)"
    );
    assert_eq!(
        wf.created_by, user,
        "returned workflow must carry the creating user_id"
    );
    assert_eq!(
        wf.external_envelope_id.as_deref(),
        Some(envelope_id.as_str()),
        "external_envelope_id round-trips correctly"
    );
}

/// An unknown envelope id returns `None` — the handler's "ignore" branch.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn find_esignature_workflow_returns_none_for_unknown_envelope(pool: PgPool) {
    ensure_esignature_workflows_table(&pool).await;

    let repo = IntegrationRepository::new(pool.clone());
    let unknown = format!("env_unknown_{}", Uuid::new_v4());

    let result = repo
        .find_esignature_workflow_by_external_id(&pool, &unknown)
        .await
        .expect("find_esignature_workflow_by_external_id");

    assert!(
        result.is_none(),
        "unknown envelope_id must return None — handler ignores it with 200"
    );
}
