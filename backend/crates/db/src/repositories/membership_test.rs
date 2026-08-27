//! Regression tests for `MembershipRepository::list_for_user` deterministic
//! ordering (issue #2861).
//!
//! `list_for_user` historically had no `ORDER BY`, so Postgres returned the
//! rows in arbitrary physical order. `.first()` callers (e.g.
//! `check_principal_kind_change`'s liveness policy load) therefore behaved
//! non-deterministically. The query now sorts by
//! `granted_at ASC, organization_id ASC, role ASC` — a total order given the
//! `(user_id, organization_id, role)` primary key.

#[cfg(test)]
mod tests {
    use crate::repositories::membership::MembershipRepository;
    use chrono::{DateTime, Duration, Utc};
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
        sqlx::query("INSERT INTO organizations (name, slug) VALUES ($1, $2) RETURNING id")
            .bind(slug)
            .bind(slug)
            .fetch_one(pool)
            .await
            .unwrap()
            .get("id")
    }

    async fn add_membership(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        org_id: Uuid,
        role: &str,
        granted_at: DateTime<Utc>,
    ) {
        sqlx::query(
            "INSERT INTO user_memberships (user_id, organization_id, role, granted_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(org_id)
        .bind(role)
        .bind(granted_at)
        .execute(pool)
        .await
        .unwrap();
    }

    /// Rows must come back oldest-`granted_at` first, regardless of insertion
    /// order — the crux of the #2861 non-determinism.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_for_user_orders_by_granted_at(pool: sqlx::PgPool) {
        let repo = MembershipRepository::new(pool.clone());
        let user = make_user(&pool, "order@example.com").await;
        let org_late = make_org(&pool, "late-org").await;
        let org_early = make_org(&pool, "early-org").await;

        let now = Utc::now();
        // Insert the LATER membership first so a missing ORDER BY would likely
        // surface it first.
        add_membership(&pool, user, org_late, "member", now).await;
        add_membership(&pool, user, org_early, "member", now - Duration::days(2)).await;

        let rows = repo.list_for_user(user).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].organization_id, org_early,
            "earliest granted_at must sort first"
        );
        assert_eq!(rows[1].organization_id, org_late);
    }

    /// Ties on `granted_at` break deterministically by `organization_id` then
    /// `role`, giving `.first()` callers a stable pick.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn list_for_user_tiebreaks_deterministically(pool: sqlx::PgPool) {
        let repo = MembershipRepository::new(pool.clone());
        let user = make_user(&pool, "tiebreak@example.com").await;
        let org = make_org(&pool, "tiebreak-org").await;

        // Same org, same granted_at, two roles → role ASC ("admin" < "manager").
        let ts = Utc::now();
        add_membership(&pool, user, org, "manager", ts).await;
        add_membership(&pool, user, org, "admin", ts).await;

        let rows = repo.list_for_user(user).await.unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].role, "admin", "role must tiebreak ascending");
        assert_eq!(rows[1].role, "manager");

        // The same query twice returns the same order (deterministic).
        let again = repo.list_for_user(user).await.unwrap();
        assert_eq!(
            again.iter().map(|m| m.role.clone()).collect::<Vec<_>>(),
            rows.iter().map(|m| m.role.clone()).collect::<Vec<_>>(),
        );
    }
}
