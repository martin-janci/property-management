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
        sqlx::query("INSERT INTO notification_schedule (user_id, digest_enabled) VALUES ($1, $2)")
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

    /// Insert a digest row and backdate its `created_at` to `hours_ago`,
    /// leaving `email_sent_at` NULL — i.e. an unsent digest that has aged.
    async fn aged_unsent_digest(pool: &sqlx::PgPool, user_id: Uuid, hours_ago: i64) -> Uuid {
        let created = Utc::now() - Duration::hours(hours_ago);
        sqlx::query(
            "INSERT INTO notification_digests \
             (user_id, digest_type, period_start, period_end, notification_count, created_at) \
             VALUES ($1, 'daily', $2, $3, 3, $4) RETURNING id",
        )
        .bind(user_id)
        .bind(created - Duration::hours(1))
        .bind(created)
        .bind(created)
        .fetch_one(pool)
        .await
        .unwrap()
        .get("id")
    }

    /// Read the `abandoned_at` timestamp of a digest row (NULL if not abandoned).
    async fn abandoned_at(pool: &sqlx::PgPool, digest_id: Uuid) -> Option<chrono::DateTime<Utc>> {
        sqlx::query("SELECT abandoned_at FROM notification_digests WHERE id = $1")
            .bind(digest_id)
            .fetch_one(pool)
            .await
            .unwrap()
            .get("abandoned_at")
    }

    /// A digest whose email send keeps failing drops out of `get_unsent_digests`
    /// once it ages past the resend window. Before the fix it was then silently
    /// given up on — never retried and recorded only in a log line.
    /// `mark_newly_abandoned_digests` stamps a durable `abandoned_at` on each
    /// aged-out row (a queryable terminal state) and returns its id exactly once,
    /// so the worker can log it once and operators can query it afterwards.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn aged_out_digest_is_marked_abandoned_once_and_is_queryable(pool: sqlx::PgPool) {
        let repo = GranularNotificationRepository::new(pool.clone());
        let user = make_user(&pool, "abandoned@example.com").await;

        let now = Utc::now();
        // The resend cutoff the worker passes each tick. A full hour of margin
        // on each side keeps the clock drift between capturing `now` here and
        // the inserts below from flipping a boundary row across the edge.
        let resend_cutoff = now - Duration::hours(19);

        // (a) Aged out of the retry window → marked abandoned.
        let abandoned = aged_unsent_digest(&pool, user, 20).await;
        // (b) Still retryable (inside the window) → NOT marked.
        let fresh = aged_unsent_digest(&pool, user, 1).await;
        // (c) Aged out but already delivered → NOT marked.
        let sent = aged_unsent_digest(&pool, user, 40).await;
        repo.mark_digest_email_sent(sent).await.unwrap();

        let marked = repo
            .mark_newly_abandoned_digests(resend_cutoff, 100)
            .await
            .unwrap();

        assert_eq!(marked, vec![abandoned], "exactly the aged-out, unsent row");

        // The terminal state is now durable and queryable, not log-only.
        assert!(
            abandoned_at(&pool, abandoned).await.is_some(),
            "the abandoned digest must carry a persisted abandoned_at timestamp"
        );
        assert!(
            abandoned_at(&pool, fresh).await.is_none(),
            "a still-retryable digest must not be marked abandoned"
        );
        assert!(
            abandoned_at(&pool, sent).await.is_none(),
            "a delivered digest must never be marked abandoned"
        );

        // Idempotent: a second sweep neither re-returns nor re-stamps the row.
        let marked_again = repo
            .mark_newly_abandoned_digests(resend_cutoff, 100)
            .await
            .unwrap();
        assert!(
            marked_again.is_empty(),
            "an already-abandoned digest must not be reported again"
        );

        // An abandoned row is terminal: it is not resurrected for retry even if
        // the retry window is later widened to cover it.
        let retryable = repo
            .get_unsent_digests(now - Duration::hours(100), 100)
            .await
            .unwrap();
        assert!(
            !retryable.iter().any(|d| d.digest_id == abandoned),
            "an abandoned digest must never be resurrected for retry"
        );
    }

    /// Backdate a user's `created_at` to `hours_ago` in the past, simulating an
    /// account that registered long ago.
    async fn backdate_created_at(pool: &sqlx::PgPool, user_id: Uuid, hours_ago: i64) {
        let when = Utc::now() - Duration::hours(hours_ago);
        sqlx::query("UPDATE users SET created_at = $2 WHERE id = $1")
            .bind(user_id)
            .bind(when)
            .execute(pool)
            .await
            .unwrap();
    }

    /// Regression (PM #969 gap 1): a user who registered long ago but has never
    /// logged in has NO `refresh_tokens` row, so the old INNER JOIN on
    /// `last_activity` silently dropped them from the candidate set — they would
    /// never receive an inactivity digest. The LEFT JOIN + `COALESCE(la.last_active,
    /// u.created_at)` falls back to the registration time, so an opted-in account
    /// that has been inactive since sign-up (and has unread notifications) still
    /// qualifies.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn never_logged_in_user_falls_back_to_created_at(pool: sqlx::PgPool) {
        let repo = GranularNotificationRepository::new(pool.clone());
        let user = make_user(&pool, "never-logged-in@example.com").await;
        // No `set_last_active` call: this account has zero refresh_tokens rows.
        // It registered 48h ago (> the 24h inactivity cutoff) and never signed in.
        backdate_created_at(&pool, user, 48).await;
        set_digest_enabled(&pool, user, true).await;
        add_group(&pool, user, "fault", 3, false).await;

        let now = Utc::now();
        let candidates = repo
            .find_inactive_digest_candidates(
                now - Duration::hours(24),
                now - Duration::hours(20),
                100,
            )
            .await
            .unwrap();

        assert_eq!(
            candidates.len(),
            1,
            "a never-logged-in but long-registered opted-in user with unread \
             notifications must be a digest candidate"
        );
        assert_eq!(candidates[0].user_id, user);
        assert_eq!(candidates[0].email, "never-logged-in@example.com");
        assert_eq!(candidates[0].notification_count, 3);
    }

    /// Insert a nil-id event-type group with an explicit creation / last-touch
    /// instant, returning its id. The `(user, event_type, Uuid::nil())` shape
    /// mirrors what `in_app_group_key` writes for entity-less notifications.
    async fn add_nil_id_group(
        pool: &sqlx::PgPool,
        user_id: Uuid,
        event_type: &str,
        count: i32,
        touched: chrono::DateTime<Utc>,
    ) -> Uuid {
        let entity_id = Uuid::nil();
        let group_key = format!("{event_type}_{entity_id}");
        sqlx::query(
            "INSERT INTO notification_groups \
             (user_id, group_key, entity_type, entity_id, title, notification_count, \
              latest_notification_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $7) RETURNING id",
        )
        .bind(user_id)
        .bind(&group_key)
        .bind(event_type)
        .bind(entity_id)
        .bind(format!("{event_type} group"))
        .bind(count)
        .bind(touched)
        .fetch_one(pool)
        .await
        .unwrap()
        .get("id")
    }

    /// Issue #3909 enforcement: a stale nil-id event-type group can be fetched
    /// by its `(user, entity)` key and reset — its running count is zeroed and
    /// its retention window restarts at `NOW()` — so the lifetime-stable group
    /// introduced by #3863 stops accumulating without bound.
    #[sqlx::test(migrator = "crate::MIGRATOR")]
    async fn reset_nil_id_group_window_zeroes_count_and_restarts_window(pool: sqlx::PgPool) {
        let repo = GranularNotificationRepository::new(pool.clone());
        let user = make_user(&pool, "nil-id-reset@example.com").await;

        // A long-accumulated nil-id group last touched 60 days ago.
        let stale_at = Utc::now() - Duration::days(60);
        let group_id = add_nil_id_group(&pool, user, "session_expiring_soon", 500, stale_at).await;

        // The group can be read back by its grouping entity.
        let fetched = repo
            .get_group_for_user_entity(user, "session_expiring_soon", Uuid::nil())
            .await
            .unwrap()
            .expect("the nil-id group must be fetchable by its entity key");
        assert_eq!(fetched.id, group_id);
        assert_eq!(fetched.notification_count, 500);

        // Reset the stale group.
        let did_reset = repo
            .reset_nil_id_group_window(user, group_id)
            .await
            .unwrap();
        assert!(
            did_reset,
            "an existing group must report a successful reset"
        );

        let after = repo
            .get_group_for_user_entity(user, "session_expiring_soon", Uuid::nil())
            .await
            .unwrap()
            .expect("the group still exists after reset");
        assert_eq!(
            after.notification_count, 0,
            "a reset group's running count must be zeroed"
        );
        assert!(
            after.latest_notification_at > stale_at,
            "a reset restarts the retention window at NOW()"
        );
        assert!(
            after.created_at > stale_at,
            "a reset starts a fresh creation instant"
        );

        // Resetting a non-existent group reports no rows affected.
        let missing = repo
            .reset_nil_id_group_window(user, Uuid::new_v4())
            .await
            .unwrap();
        assert!(!missing, "resetting an unknown group id must return false");
    }
}
