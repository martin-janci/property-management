//! Regression tests for the cross-tenant IDOR / PII leak fix on the enhanced
//! tenant screening endpoints (Epic 135 — issue #834, extends #760).
//!
//! Audit history: every get-by-id / get-by-screening repository method looked
//! its row up by UUID alone (`WHERE id = $1` / `WHERE screening_id = $1`),
//! while the route handlers fetched the caller's org from the JWT and then
//! discarded it (`let _ = get_org_id(...)`). A caller from another org could
//! therefore read a foreign org's risk model, provider config, AI risk score,
//! credit / background / eviction result, or aggregated report by guessing the
//! UUID — a disclosure of sensitive PII.
//!
//! The fix threads the JWT tenant into each repository call so the SQL WHERE
//! clause is `... AND organization_id = $N`, and the handlers pass the real
//! org id. Cross-org reads now return `None` (surfaced as 404); the owning
//! org still reads its own rows.
//!
//! These tests pin the contract at the repository layer (the authority the
//! handlers scope to), exercising both the positive case (owner reads its row)
//! and the negative case (foreign org gets nothing) without depending on the
//! TestApp auth harness — which cannot mint a JWT carrying `tenant_id` for the
//! happy path because `JwtService` emits an `org_id` claim under a different
//! name than the `AuthUser` extractor reads.

#![allow(dead_code)]

#[allow(dead_code)]
mod common;

use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use db::models::enhanced_tenant_screening::{
    CreateAiRiskScoringModel, CreateScreeningProviderConfig, ScreeningProviderType,
};
use db::repositories::EnhancedTenantScreeningRepository;

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
    .bind(format!("ScreeningIDOR Org {slug}"))
    .bind(format!("screening-idor-{slug}"))
    .bind(format!("{slug}@screening-idor.test"))
    .fetch_one(pool)
    .await
    .expect("seed org")
}

async fn seed_user(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO users (email, password_hash, name, status, email_verified_at)
        VALUES ($1, 'test_hash', 'ScreeningIDOR User', 'active', NOW())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("seed user")
}

fn sample_model(name: &str) -> CreateAiRiskScoringModel {
    CreateAiRiskScoringModel {
        name: name.to_string(),
        description: None,
        credit_history_weight: None,
        rental_history_weight: None,
        income_stability_weight: None,
        employment_stability_weight: None,
        eviction_history_weight: None,
        criminal_background_weight: None,
        identity_verification_weight: None,
        reference_quality_weight: None,
        excellent_threshold: None,
        good_threshold: None,
        fair_threshold: None,
        poor_threshold: None,
        auto_approve_threshold: None,
        auto_reject_threshold: None,
    }
}

fn sample_provider(name: &str) -> CreateScreeningProviderConfig {
    CreateScreeningProviderConfig {
        provider_name: name.to_string(),
        provider_type: ScreeningProviderType::CreditBureau,
        api_endpoint: Some("https://example.test/api".to_string()),
        api_key: None,
        api_secret: None,
        rate_limit_per_hour: None,
        priority: None,
        cost_per_check: Some(Decimal::new(199, 2)),
    }
}

// ---------------------------------------------------------------------------
// Seed the full chain required for a screening_ai_results row.
// organizations -> buildings -> units -> tenant_applications -> tenant_screenings
// ---------------------------------------------------------------------------

async fn seed_screening(pool: &PgPool, org_id: Uuid, slug: &str) -> Uuid {
    let building_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO buildings (organization_id, street, city, postal_code, country)
        VALUES ($1, $2, 'Bratislava', '81101', 'Slovakia') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(format!("{slug} Street 1"))
    .fetch_one(pool)
    .await
    .expect("seed building");

    let unit_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO units (
            building_id, designation, floor, unit_type,
            size_sqm, rooms, ownership_share, occupancy_status, status, settings
        )
        VALUES ($1, $2, 1, 'apartment', 50.0, 2, 100.00, 'unknown', 'active', '{}'::jsonb)
        RETURNING id
        "#,
    )
    .bind(building_id)
    .bind(format!("{slug}-1A"))
    .fetch_one(pool)
    .await
    .expect("seed unit");

    let application_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO tenant_applications (
            organization_id, unit_id, applicant_name, applicant_email, status
        )
        VALUES ($1, $2, 'Applicant', $3, 'draft') RETURNING id
        "#,
    )
    .bind(org_id)
    .bind(unit_id)
    .bind(format!("{slug}@applicant.test"))
    .fetch_one(pool)
    .await
    .expect("seed application");

    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO tenant_screenings (
            application_id, organization_id, screening_type, status
        )
        VALUES ($1, $2, 'credit_check', 'completed') RETURNING id
        "#,
    )
    .bind(application_id)
    .bind(org_id)
    .fetch_one(pool)
    .await
    .expect("seed screening")
}

/// Insert a screening_ai_results row for `screening_id` in `org_id`.
async fn seed_ai_result(pool: &PgPool, org_id: Uuid, screening_id: Uuid, model_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO screening_ai_results (
            screening_id, organization_id, model_id,
            ai_risk_score, risk_category, recommendation
        )
        VALUES ($1, $2, $3, 72, 'good', 'approve') RETURNING id
        "#,
    )
    .bind(screening_id)
    .bind(org_id)
    .bind(model_id)
    .fetch_one(pool)
    .await
    .expect("seed ai result")
}

// ---------------------------------------------------------------------------
// T1 — risk models are scoped to the owning org (positive + negative)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repo_risk_model_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "model-a").await;
    let org_b = seed_org(&pool, "model-b").await;
    let user_a = seed_user(&pool, "model-a@screening-idor.test").await;

    let repo = EnhancedTenantScreeningRepository::new(pool.clone());
    let model_a = repo
        .create_risk_model(&pool, org_a, user_a, sample_model("Model A"))
        .await
        .expect("create model A");

    // The owning org reads its model.
    let own = repo
        .get_risk_model(&pool, org_a, model_a.id)
        .await
        .expect("get model with right org");
    assert!(own.is_some(), "org A must read its own risk model by id");
    assert_eq!(own.unwrap().id, model_a.id);

    // A foreign org gets nothing (closed IDOR).
    let cross = repo
        .get_risk_model(&pool, org_b, model_a.id)
        .await
        .expect("get model with wrong org");
    assert!(
        cross.is_none(),
        "org B must not read org A's risk model by id"
    );
}

// ---------------------------------------------------------------------------
// T2 — provider configs are scoped to the owning org (positive + negative)
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repo_provider_config_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "prov-a").await;
    let org_b = seed_org(&pool, "prov-b").await;

    let repo = EnhancedTenantScreeningRepository::new(pool.clone());
    let config_a = repo
        .create_provider_config(&pool, org_a, sample_provider("Provider A"))
        .await
        .expect("create provider A");

    let own = repo
        .get_provider_config(&pool, org_a, config_a.id)
        .await
        .expect("get config with right org");
    assert!(own.is_some(), "org A must read its own provider config");
    assert_eq!(own.unwrap().id, config_a.id);

    let cross = repo
        .get_provider_config(&pool, org_b, config_a.id)
        .await
        .expect("get config with wrong org");
    assert!(
        cross.is_none(),
        "org B must not read org A's provider config by id"
    );
}

// ---------------------------------------------------------------------------
// T3 — AI risk results (sensitive PII) are scoped to the owning org
// ---------------------------------------------------------------------------

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn repo_ai_result_is_scoped_to_owning_org(pool: PgPool) {
    let org_a = seed_org(&pool, "ai-a").await;
    let org_b = seed_org(&pool, "ai-b").await;

    let user_a = seed_user(&pool, "ai-a@screening-idor.test").await;
    let repo = EnhancedTenantScreeningRepository::new(pool.clone());
    let model_a = repo
        .create_risk_model(&pool, org_a, user_a, sample_model("AI Model A"))
        .await
        .expect("create model A");
    let screening_a = seed_screening(&pool, org_a, "ai-a").await;
    let _result_a = seed_ai_result(&pool, org_a, screening_a, model_a.id).await;

    // The owning org reads the AI result for its screening.
    let own = repo
        .get_ai_result_by_screening(&pool, org_a, screening_a)
        .await
        .expect("get ai result with right org");
    assert!(own.is_some(), "org A must read its own screening AI result");
    assert_eq!(own.unwrap().screening_id, screening_a);

    // A foreign org gets nothing — the risk score / recommendation never leaks.
    let cross = repo
        .get_ai_result_by_screening(&pool, org_b, screening_a)
        .await
        .expect("get ai result with wrong org");
    assert!(
        cross.is_none(),
        "org B must not read org A's screening AI result"
    );

    // The complete-data aggregate (multi-statement) takes a connection.
    let mut conn = pool.acquire().await.expect("acquire conn");

    // The complete-data aggregate is likewise closed for the foreign org.
    let cross_complete = repo
        .get_complete_screening_data(&mut conn, org_b, screening_a)
        .await
        .expect("complete data with wrong org");
    assert!(
        cross_complete.ai_result.is_none(),
        "org B must not get org A's AI result via the complete-data aggregate"
    );

    // ...and open for the owner.
    let own_complete = repo
        .get_complete_screening_data(&mut conn, org_a, screening_a)
        .await
        .expect("complete data with right org");
    assert!(
        own_complete.ai_result.is_some(),
        "org A must read its own complete screening data"
    );
}
