//! Round-trip tests for `get_agency_for_user` (Issue #2359).
//!
//! These guard the `/api/v1/agencies/me` resolution against the class of bug
//! spotted in the post-merge review of PR #2355: because membership is
//! soft-deactivated (`is_active = false`, `left_at` set) rather than
//! row-deleted, a query that joins `reality_agency_members` without the
//! `is_active = TRUE` filter resolves a *departed* member back to a former
//! agency — and, via `ORDER BY joined_at ASC`, can even hand a current member
//! the wrong (earlier, since-left) agency.
//!
//! Uses `#[sqlx::test]`, which provisions an ephemeral migrated database per
//! test (needs `DATABASE_URL`), so these run automatically in CI rather than
//! behind the `#[ignore]`/live-DB gate used by `tests.rs`.

#[cfg(test)]
mod tests {
    use crate::repositories::reality_portal::RealityPortalRepository;
    use chrono::{DateTime, Utc};
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn seed_user(pool: &PgPool, tag: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO users (email, password_hash, name, status, email_verified_at)
            VALUES ($1, 'hash', $2, 'active', NOW())
            RETURNING id
            "#,
        )
        .bind(format!("{tag}@agencies-rt.test"))
        .bind(format!("AgencyRT {tag}"))
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("seed_user({tag}): {e}"))
    }

    async fn seed_agency(pool: &PgPool, slug: &str) -> Uuid {
        sqlx::query_scalar::<_, Uuid>(
            r#"
            INSERT INTO reality_agencies (name, slug, email)
            VALUES ($1, $2, $3)
            RETURNING id
            "#,
        )
        .bind(format!("Agency {slug}"))
        .bind(format!("agency-{slug}"))
        .bind(format!("{slug}@agencies-rt.test"))
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("seed_agency({slug}): {e}"))
    }

    /// Seed a membership row with explicit `is_active` / `joined_at` so the
    /// tests can pin the soft-delete and multi-membership tiebreak behaviour.
    async fn seed_membership(
        pool: &PgPool,
        agency_id: Uuid,
        user_id: Uuid,
        is_active: bool,
        joined_at: DateTime<Utc>,
    ) {
        let left_at: Option<DateTime<Utc>> = if is_active { None } else { Some(Utc::now()) };
        sqlx::query(
            r#"
            INSERT INTO reality_agency_members (agency_id, user_id, role, is_active, joined_at, left_at)
            VALUES ($1, $2, 'realtor', $3, $4, $5)
            "#,
        )
        .bind(agency_id)
        .bind(user_id)
        .bind(is_active)
        .bind(joined_at)
        .bind(left_at)
        .execute(pool)
        .await
        .expect("seed_membership failed");
    }

    /// Active member → resolves to their agency.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn active_member_resolves_to_agency(pool: PgPool) {
        let user = seed_user(&pool, "active").await;
        let agency = seed_agency(&pool, "active").await;
        seed_membership(&pool, agency, user, true, Utc::now()).await;

        let repo = RealityPortalRepository::new(pool);
        let resolved = repo.get_agency_for_user(user).await.expect("query failed");

        assert_eq!(
            resolved.map(|a| a.id),
            Some(agency),
            "an active member must resolve to their agency"
        );
    }

    /// Left / inactive member with no other membership → None (not the former
    /// agency). This is the core authz regression guard for #2359.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn left_member_resolves_to_none(pool: PgPool) {
        let user = seed_user(&pool, "left").await;
        let agency = seed_agency(&pool, "left").await;
        seed_membership(&pool, agency, user, false, Utc::now()).await;

        let repo = RealityPortalRepository::new(pool);
        let resolved = repo.get_agency_for_user(user).await.expect("query failed");

        assert!(
            resolved.is_none(),
            "a departed member (is_active = false) must NOT resolve back to the former agency"
        );
    }

    /// Left agency A earlier, now active in agency B → resolves to B, not the
    /// earlier-joined-but-since-left A. Guards the `ORDER BY joined_at ASC`
    /// wrong-agency variant of the bug.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn left_earlier_agency_resolves_to_current_active(pool: PgPool) {
        let user = seed_user(&pool, "switch").await;
        let agency_a = seed_agency(&pool, "switch-a").await;
        let agency_b = seed_agency(&pool, "switch-b").await;

        let earlier = "2024-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let later = "2025-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap();

        // Joined A first (earliest) then left it; now an active member of B.
        seed_membership(&pool, agency_a, user, false, earlier).await;
        seed_membership(&pool, agency_b, user, true, later).await;

        let repo = RealityPortalRepository::new(pool);
        let resolved = repo.get_agency_for_user(user).await.expect("query failed");

        assert_eq!(
            resolved.map(|a| a.id),
            Some(agency_b),
            "a current active member must get their active agency (B), not the earlier since-left one (A)"
        );
    }

    /// Non-member → None.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn non_member_resolves_to_none(pool: PgPool) {
        let user = seed_user(&pool, "nomember").await;

        let repo = RealityPortalRepository::new(pool);
        let resolved = repo.get_agency_for_user(user).await.expect("query failed");

        assert!(resolved.is_none(), "a non-member must resolve to None");
    }
}
