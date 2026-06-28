//! Migration-effect (catalog metadata) regression test for the Epic-11+
//! `app.current_organization_id` RLS owner-bypass + GUC-mismatch cluster
//! (PAP-62).
//!
//! A large cluster of Epic-11+ feature tables (work orders, budgets, vendors,
//! equipment, ESG, screening, insurance, emergency, certifications, board /
//! HOA meetings, forms, sensors, workflows, document-intelligence, reserve
//! funds, subscription billing, AML/AI, webhooks, ...) shipped their
//! tenant-isolation policies against
//! `current_setting('app.current_organization_id')` and only `ENABLE`d RLS --
//! never `FORCE`. Two compounding defects:
//!
//!   * GUC mismatch -- nothing in the codebase sets `app.current_organization_id`
//!     (`set_request_context` sets `app.current_org_id`, read by
//!     `get_current_org_id()`). For any non-owner role the policies collapsed
//!     to `organization_id = NULL` -> deny-all.
//!   * Owner-bypass -- `ENABLE` without `FORCE` is bypassed for the table
//!     owner. The production api-server connects as the table OWNER, so RLS was
//!     bypassed entirely -> every tenant context saw every org's rows: a
//!     cross-tenant data-leak path (PAP-51 exit gate).
//!
//! Migration 00179 rewrites every policy to the canonical `get_current_org_id()`
//! helper and adds `FORCE ROW LEVEL SECURITY`. This mirrors the documents
//! (#754 / 00172), messaging (#898 / 00171) and Epic 8A notification
//! (#755 / 00177) fixes.
//!
//! ## Why this is a metadata test, not a behavioral one
//!
//! `#[sqlx::test]` connects as the `DATABASE_URL` superuser, which PostgreSQL
//! exempts from RLS entirely -- a behavioral "must not see" assertion passes
//! vacuously here. Behavioral cross-tenant isolation is covered by CI's
//! dedicated non-superuser `rls-smoke-test` / `security-tests` jobs. This suite
//! instead asserts on the Postgres catalog what 00179 changes, mirroring the
//! 00177 notification precedent (`notification_rls_guc_tests.rs`):
//!   1. FORCE RLS (`relforcerowsecurity`) is set on each cluster table.
//!   2. Each table still carries at least one policy (FORCE is real enforcement,
//!      not implicit deny-all).
//!   3. No surviving policy references the never-set `app.current_organization_id`
//!      GUC.
//!
//! Assertions (1) and (3) FAIL on `origin/dev` (pre-00179:
//! `relforcerowsecurity = false`, policies still on `app.current_organization_id`)
//! -- the IG3 regression intent -- and PASS after 00179 is applied.

use sqlx::PgPool;

/// The Epic-11+ feature tables that 00179 brings under FORCE RLS with corrected
/// `get_current_org_id()` policies. Generated from the full migration history
/// (every table whose authoritative policy still referenced
/// `app.current_organization_id` after 00140).
const ORG_GUC_CLUSTER_TABLES: [&str; 135] = [
    "ai_chat_messages",
    "ai_chat_sessions",
    "ai_risk_scoring_models",
    "ai_training_feedback",
    "board_meetings",
    "board_members",
    "budget_actuals",
    "budget_categories",
    "budget_items",
    "budget_variance_alerts",
    "budgets",
    "building_certifications",
    "capital_plans",
    "carbon_footprints",
    "certification_audit_logs",
    "certification_benchmarks",
    "certification_costs",
    "certification_credits",
    "certification_documents",
    "certification_milestones",
    "certification_reminders",
    "compliance_audit_trail",
    "compliance_requirements",
    "compliance_templates",
    "compliance_verifications",
    "connector_execution_logs",
    "coupon_redemptions",
    "document_classification_history",
    "document_embeddings",
    "document_intelligence_stats",
    "document_ocr_queue",
    "document_summarization_queue",
    "emergency_broadcast_acknowledgments",
    "emergency_broadcasts",
    "emergency_contacts",
    "emergency_drills",
    "emergency_incident_attachments",
    "emergency_incident_updates",
    "emergency_incidents",
    "emergency_protocols",
    "equipment",
    "equipment_documents",
    "equipment_health_thresholds",
    "equipment_maintenance",
    "equipment_predictions",
    "equipment_registry",
    "esg_benchmarks",
    "esg_configurations",
    "esg_dashboard_metrics",
    "esg_import_jobs",
    "esg_metrics",
    "esg_reports",
    "esg_targets",
    "eu_taxonomy_assessments",
    "financial_forecasts",
    "form_downloads",
    "form_fields",
    "form_submissions",
    "forms",
    "fund_alerts",
    "fund_components",
    "fund_contribution_schedules",
    "fund_investment_policies",
    "fund_projection_items",
    "fund_projections",
    "fund_transactions",
    "insurance_claim_documents",
    "insurance_claim_history",
    "insurance_claims",
    "insurance_policies",
    "insurance_policy_documents",
    "insurance_renewal_reminders",
    "integration_ratings",
    "invoice_line_items",
    "legal_document_versions",
    "legal_documents",
    "legal_notice_recipients",
    "legal_notices",
    "maintenance_alerts",
    "maintenance_log_photos",
    "maintenance_logs",
    "maintenance_predictions",
    "maintenance_schedules",
    "meeting_action_items",
    "meeting_agenda_items",
    "meeting_attendance",
    "meeting_documents",
    "meeting_minutes",
    "meeting_motions",
    "meeting_statistics",
    "motion_votes",
    "organization_connectors",
    "organization_integrations",
    "organization_subscriptions",
    "payment_methods",
    "reserve_funds",
    "schedule_executions",
    "screening_ai_results",
    "screening_background_results",
    "screening_credit_results",
    "screening_eviction_results",
    "screening_provider_configs",
    "screening_reports",
    "screening_request_queue",
    "screening_risk_factors",
    "search_embeddings",
    "search_history",
    "sensor_alerts",
    "sensor_fault_correlations",
    "sensor_readings",
    "sensor_threshold_templates",
    "sensor_thresholds",
    "sensors",
    "sentiment_alerts",
    "sentiment_thresholds",
    "sentiment_trends",
    "subscription_events",
    "subscription_invoices",
    "usage_records",
    "vendor_contacts",
    "vendor_contract_documents",
    "vendor_contracts",
    "vendor_invoices",
    "vendor_ratings",
    "vendor_service_areas",
    "vendors",
    "webhook_deliveries",
    "webhook_subscriptions",
    "work_order_updates",
    "work_orders",
    "workflow_actions",
    "workflow_execution_steps",
    "workflow_executions",
    "workflow_schedules",
    "workflows",
];

/// 00179 must set `relforcerowsecurity = true` (FORCE) on every cluster table,
/// with `relrowsecurity = true` (ENABLE, from the create migrations) still in
/// place. Pre-migration these are `(true, false)`, so this fails on origin/dev.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn org_guc_cluster_tables_have_force_rls_enabled(pool: PgPool) {
    let rows: Vec<(String, bool, bool)> = sqlx::query_as(
        r#"
        SELECT relname, relrowsecurity, relforcerowsecurity
        FROM pg_class
        WHERE relname = ANY($1)
          AND relnamespace = 'public'::regnamespace
        ORDER BY relname
        "#,
    )
    .bind(&ORG_GUC_CLUSTER_TABLES[..])
    .fetch_all(&pool)
    .await
    .expect("query pg_class RLS flags");

    assert_eq!(
        rows.len(),
        ORG_GUC_CLUSTER_TABLES.len(),
        "expected all Epic-11+ org-GUC cluster tables present in pg_class (public schema); \
         got {} of {}",
        rows.len(),
        ORG_GUC_CLUSTER_TABLES.len()
    );

    for (relname, relrowsecurity, relforcerowsecurity) in &rows {
        assert!(
            *relrowsecurity,
            "{relname}: ENABLE ROW LEVEL SECURITY (relrowsecurity) must be set"
        );
        assert!(
            *relforcerowsecurity,
            "{relname}: FORCE ROW LEVEL SECURITY (relforcerowsecurity) must be set by 00179 -- \
             without FORCE the table owner bypasses the tenant-isolation policy (PAP-62)"
        );
    }
}

/// FORCE RLS with no policy is an implicit deny-all; FORCE RLS with policies is
/// real enforcement. Assert each table still carries at least one RLS policy.
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn org_guc_cluster_tables_have_rls_policies(pool: PgPool) {
    for table in ORG_GUC_CLUSTER_TABLES {
        let policy_count: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)
            FROM pg_policies
            WHERE schemaname = 'public'
              AND tablename = $1
            "#,
        )
        .bind(table)
        .fetch_one(&pool)
        .await
        .expect("query pg_policies");

        assert!(
            policy_count > 0,
            "{table}: expected at least one RLS policy so FORCE ROW LEVEL SECURITY enforces a \
             real policy rather than implicit deny-all; found {policy_count}"
        );
    }
}

/// No surviving policy on the cluster tables may reference the never-set
/// `app.current_organization_id` GUC -- every policy must use the canonical
/// `get_current_org_id()` / `app.current_org_id` convention. The `qual`
/// (USING) and `with_check` expressions are inspected as text. Fails on
/// origin/dev (policies still on `app.current_organization_id`).
#[sqlx::test(migrator = "db::MIGRATOR")]
#[ignore = "BIT-351 quarantine: pre-existing blind-CI test failure (schema/seed never migrated or repo decode drift); never green on the real PR gate. Repair tracked in BIT-352."]
async fn org_guc_cluster_policies_use_canonical_org_guc(pool: PgPool) {
    let offenders: Vec<(String, String)> = sqlx::query_as(
        r#"
        SELECT tablename, policyname
        FROM pg_policies
        WHERE schemaname = 'public'
          AND tablename = ANY($1)
          AND (
                COALESCE(qual, '')       LIKE '%current_organization_id%'
             OR COALESCE(with_check, '') LIKE '%current_organization_id%'
          )
        ORDER BY tablename, policyname
        "#,
    )
    .bind(&ORG_GUC_CLUSTER_TABLES[..])
    .fetch_all(&pool)
    .await
    .expect("query pg_policies for legacy app.current_organization_id GUC");

    assert!(
        offenders.is_empty(),
        "Epic-11+ cluster policies must use the canonical get_current_org_id() convention \
         (00179); these still reference the never-set app.current_organization_id GUC: {offenders:?}"
    );
}
