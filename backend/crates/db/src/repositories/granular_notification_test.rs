//! Tests for the inactivity email-digest query path (Story 2B.3, PM #969 gap 1).

#[cfg(test)]
mod tests {
    use crate::repositories::granular_notification::GranularNotificationRepository;
    use chrono::{Duration, Utc};
    use sqlx::Row;
    use uuid::Uuid;

    /// Insert a minimal active user and return its id.
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

    /// Record the user's last activity by inserting a refresh token whose
    /// `last_used_at` is `hours_ago` in the past.
    async fn set_last_active(pool: &sqlx::PgPool, user_id: Uuid, hours_ago: i64) {
        let when = Utc::now() - Duration::hours(hours_ago);
        sqlx::query(
            "INSERT INTO refresh_tokens (user_id, token_hash, expires_at, last_used_at) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(user_id)
        .bind(format!("hash-{user_id}-{hours_ago}"))
        .bind(Utc::now() + Duration::days(30))
        .bind(when)
        .execute(pool)
        .await
        .unwrap();
    }

    /// Enable (or disable) the digest preference for a user.
    async fn set_digest_enabled(pool: &sqlx::PgPool, user_id: Uuid, enabled: bool) {
        sqlx::query(
            "INSERT INTO notification_schedule (user_id, digest_enabled) VALUES ($1, $2)",
        )
        .bind(user_id)
        .bind(enabled)
        .execute(pool)
        .await
        .unwrap();
    }

    /// Insert one notification group for the user.
    async fn add_group(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        entity_type: &str,
        count: i32,
        is_read: bool,
    ) {
        let entity_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO notification_groups \
             (user_id, group_key, entity_type, entity_id, title, notification_count, is_read) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(user_id)
        .bind(format!("{entity_type}-{entity_id}"))
        .bind(entity_type)
        .bind(entity_id)
        .bind(format!("{entity_type} group"))
        .bind(count)
        .bind(is_read)
        .execute(pool)
        .await
        .unwrap();
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn inactive_opted_in_user_with_unread_is_a_candidate(pool: sqlx::PgPool) {
        let repo = GranularNotificationRepository::new(pool.clone());
        let user = make_user(&pool, "digest-hit@example.com").await;
        set_last_active(&pool, user, 48).await; // inactive > 24h
        set_digest_enabled(&pool, user, true).await;
        // 2 fault notifications (2 + 1) and 2 vote notifications.
        add_group(&pool, user, "fault", 2, false).await;
        add_group(&pool, user, "fault", 1, false).await;
        add_group(&pool, user, "vote", 2, false).await;
        // A read group must NOT be counted.
        add_group(&pool, user, "announcement", 9, true).await;

        let now = Utc::now();
        let candidates = repo
            .find_inactive_digest_candidates(
                now - Duration::hours(24),
                now - Duration::hours(20),
                100,
            )
            .await
            .unwrap();

        assert_eq!(candidates.len(), 1, "exactly one candidate expected");
        let c = &candidates[0];
        assert_eq!(c.user_id, user);
        assert_eq!(c.email, "digest-hit@example.com");
        assert_eq!(c.notification_count, 5, "2+1 fault + 2 vote = 5 unread");
        assert_eq!(c.category_counts["fault"].as_i64(), Some(3));
        assert_eq!(c.category_counts["vote"].as_i64(), Some(2));
        assert!(
            c.category_counts.get("announcement").is_none(),
            "read groups must be excluded from the breakdown"
        );
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn excludes_recently_active_optout_and_no_unread_users(pool: sqlx::PgPool) {
        let repo = GranularNotificationRepository::new(pool.clone());

        // Recently active (12h) — excluded despite unread + opt-in.
        let active = make_user(&pool, "active@example.com").await;
        set_last_active(&pool, active, 12).await;
        set_digest_enabled(&pool, active, true).await;
        add_group(&pool, active, "fault", 3, false).await;

        // Inactive but opted OUT — excluded.
        let optout = make_user(&pool, "optout@example.com").await;
        set_last_active(&pool, optout, 48).await;
        set_digest_enabled(&pool, optout, false).await;
        add_group(&pool, optout, "fault", 3, false).await;

        // Inactive, opted in, but no unread — excluded.
        let empty = make_user(&pool, "empty@example.com").await;
        set_last_active(&pool, empty, 48).await;
        set_digest_enabled(&pool, empty, true).await;
        add_group(&pool, empty, "fault", 3, true).await; // all read

        let now = Utc::now();
        let candidates = repo
            .find_inactive_digest_candidates(
                now - Duration::hours(24),
                now - Duration::hours(20),
                100,
            )
            .await
            .unwrap();

        assert!(candidates.is_empty(), "no user should qualify");
    }

    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn recent_digest_suppresses_then_unsent_is_retried(pool: sqlx::PgPool) {
        let repo = GranularNotificationRepository::new(pool.clone());
        let user = make_user(&pool, "dedup@example.com").await;
        set_last_active(&pool, user, 48).await;
        set_digest_enabled(&pool, user, true).await;
        add_group(&pool, user, "fault", 4, false).await;

        let now = Utc::now();

        // Persist a digest row (email not yet sent).
        let digest = repo
            .create_digest(
                user,
                "daily",
                now - Duration::hours(1),
                now,
                4,
                serde_json::json!({ "fault": 4 }),
                None,
                None,
            )
            .await
            .unwrap();

        // The fresh row is within the resend window, so the user must NOT be a
        // new candidate (no duplicate digest).
        let candidates = repo
            .find_inactive_digest_candidates(
                now - Duration::hours(24),
                now - Duration::hours(20),
                100,
            )
            .await
            .unwrap();
        assert!(
            candidates.is_empty(),
            "a recent digest row must suppress new candidacy"
        );

        // But the unsent row is surfaced for retry.
        let unsent = repo
            .get_unsent_digests(now - Duration::hours(20), 100)
            .await
            .unwrap();
        assert_eq!(unsent.len(), 1);
        assert_eq!(unsent[0].digest_id, digest.id);
        assert_eq!(unsent[0].email, "dedup@example.com");
        assert_eq!(unsent[0].notification_count, 4);

        // Once marked sent, it is no longer retried.
        repo.mark_digest_email_sent(digest.id).await.unwrap();
        let unsent_after = repo
            .get_unsent_digests(now - Duration::hours(20), 100)
            .await
            .unwrap();
        assert!(unsent_after.is_empty(), "sent digests are not retried");
    }
}
