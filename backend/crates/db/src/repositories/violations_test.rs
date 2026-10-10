//! Cross-tenant isolation regression tests for `ViolationRepository` read
//! paths — issue #2944 (Cross-tenant IDOR: violation comments / evidence /
//! payments read without org scoping, + internal-notes privilege leak).
//!
//! Before the fix, `list_evidence`, `list_comments` and `list_payments` keyed
//! only on the child id (`violation_id` / `enforcement_action_id`) with no
//! `organization_id` predicate, so a caller authenticated to Org B could read
//! Org A's rows by supplying an Org A id (IDOR). `list_comments` additionally
//! returned `is_internal` notes whenever the caller asked, with the role gate
//! living only in the route layer. These tests pin the repository-layer scoping
//! so a regression cannot silently reopen the hole.

#[cfg(test)]
mod tests {
    use crate::models::violations::{
        CreateEnforcementAction, CreateViolation, CreateViolationComment, CreateViolationEvidence,
        EnforcementActionType, RecordFinePayment, ViolationCategory,
    };
    use crate::repositories::violations::ViolationRepository;
    use chrono::Utc;
    use rust_decimal::Decimal;
    use sqlx::Row;
    use uuid::Uuid;

    async fn make_user(pool: &sqlx::PgPool, email: &str) -> Uuid {
        sqlx::query(
            "INSERT INTO users (email, password_hash, name, status) \
             VALUES ($1, 'x', 'Test User', 'active') RETURNING id",
        )
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap()
        .get("id")
    }

    async fn make_org(pool: &sqlx::PgPool, slug: &str) -> Uuid {
        sqlx::query(
            "INSERT INTO organizations (name, slug, contact_email) \
             VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(slug)
        .bind(slug)
        .bind(format!("{slug}@example.com"))
        .fetch_one(pool)
        .await
        .unwrap()
        .get("id")
    }

    fn sample_violation() -> CreateViolation {
        CreateViolation {
            building_id: None,
            unit_id: None,
            rule_id: None,
            category: ViolationCategory::Noise,
            severity: None,
            title: "Loud music".to_string(),
            description: "After hours".to_string(),
            location: None,
            violator_id: None,
            violator_name: Some("Resident".to_string()),
            violator_unit: Some("A1".to_string()),
            occurred_at: Utc::now(),
            evidence_description: None,
            witness_count: None,
        }
    }

    /// Evidence is only visible to the owning org. An Org B caller passing an
    /// Org A `violation_id` must get an empty list, never the rows.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_evidence_is_org_scoped(pool: sqlx::PgPool) {
        let repo = ViolationRepository::new(pool.clone());
        let org_a = make_org(&pool, "evi-org-a").await;
        let org_b = make_org(&pool, "evi-org-b").await;
        let reporter = make_user(&pool, "evi-reporter@example.com").await;

        let violation = repo
            .create_violation(org_a, sample_violation(), reporter)
            .await
            .unwrap();
        repo.add_evidence(
            violation.id,
            org_a,
            CreateViolationEvidence {
                file_name: "photo.jpg".to_string(),
                file_type: "image/jpeg".to_string(),
                file_size: Some(1024),
                storage_path: Some("s3://bucket/photo.jpg".to_string()),
                description: None,
                captured_at: None,
            },
            reporter,
        )
        .await
        .unwrap();

        // Owning org sees the evidence.
        let own = repo.list_evidence(violation.id, org_a).await.unwrap();
        assert_eq!(own.len(), 1, "owning org must see its own evidence");

        // Foreign org sees nothing — the IDOR is closed.
        let cross = repo.list_evidence(violation.id, org_b).await.unwrap();
        assert!(
            cross.is_empty(),
            "cross-tenant read must return no evidence rows, got {}",
            cross.len()
        );
    }

    /// Comments are org-scoped, and `include_internal=false` hides `is_internal`
    /// notes even for the owning org (route-layer role gate feeds this flag).
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_comments_is_org_scoped_and_filters_internal(pool: sqlx::PgPool) {
        let repo = ViolationRepository::new(pool.clone());
        let org_a = make_org(&pool, "cmt-org-a").await;
        let org_b = make_org(&pool, "cmt-org-b").await;
        let author = make_user(&pool, "cmt-author@example.com").await;

        let violation = repo
            .create_violation(org_a, sample_violation(), author)
            .await
            .unwrap();
        repo.add_comment(
            violation.id,
            org_a,
            CreateViolationComment {
                comment_type: "note".to_string(),
                content: "Public note".to_string(),
                is_internal: Some(false),
            },
            author,
        )
        .await
        .unwrap();
        repo.add_comment(
            violation.id,
            org_a,
            CreateViolationComment {
                comment_type: "internal".to_string(),
                content: "Staff-only note".to_string(),
                is_internal: Some(true),
            },
            author,
        )
        .await
        .unwrap();

        // Owning org, privileged view: both comments.
        let all = repo.list_comments(violation.id, org_a, true).await.unwrap();
        assert_eq!(all.len(), 2, "privileged view must include internal notes");

        // Owning org, unprivileged view: internal note hidden.
        let public = repo
            .list_comments(violation.id, org_a, false)
            .await
            .unwrap();
        assert_eq!(
            public.len(),
            1,
            "unprivileged view must hide internal notes"
        );
        assert!(
            public.iter().all(|c| !c.is_internal),
            "no internal comment may leak into the public view"
        );

        // Foreign org, even asking for internal: nothing.
        let cross = repo.list_comments(violation.id, org_b, true).await.unwrap();
        assert!(
            cross.is_empty(),
            "cross-tenant read must return no comments, got {}",
            cross.len()
        );
    }

    /// Fine payments are only visible to the owning org. An Org B caller passing
    /// an Org A `enforcement_action_id` must get an empty list.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_payments_is_org_scoped(pool: sqlx::PgPool) {
        let repo = ViolationRepository::new(pool.clone());
        let org_a = make_org(&pool, "pay-org-a").await;
        let org_b = make_org(&pool, "pay-org-b").await;
        let staff = make_user(&pool, "pay-staff@example.com").await;

        let violation = repo
            .create_violation(org_a, sample_violation(), staff)
            .await
            .unwrap();
        let action = repo
            .create_enforcement_action(
                violation.id,
                org_a,
                CreateEnforcementAction {
                    action_type: EnforcementActionType::FirstFine,
                    fine_amount: Some(Decimal::new(10000, 2)),
                    due_date: None,
                    description: None,
                    notes: None,
                    suspended_privileges: None,
                    suspension_start: None,
                    suspension_end: None,
                },
                staff,
            )
            .await
            .unwrap();
        repo.record_payment(
            action.id,
            org_a,
            RecordFinePayment {
                amount: Decimal::new(5000, 2),
                payment_method: Some("cash".to_string()),
                transaction_reference: None,
                payer_id: None,
                payer_name: Some("Resident".to_string()),
                notes: None,
            },
            staff,
        )
        .await
        .unwrap();

        // Owning org sees the payment.
        let own = repo.list_payments(action.id, org_a).await.unwrap();
        assert_eq!(own.len(), 1, "owning org must see its own payments");

        // Foreign org sees nothing.
        let cross = repo.list_payments(action.id, org_b).await.unwrap();
        assert!(
            cross.is_empty(),
            "cross-tenant read must return no payment rows, got {}",
            cross.len()
        );
    }
}
