//! Vote-lifecycle scheduled jobs for the background [`Scheduler`] (Epic 106).
//!
//! Extracted verbatim from `scheduler/mod.rs` to keep the core tick loop
//! readable (churn-reduction refactor — no behaviour change). Holds the vote
//! expiry/reminder jobs:
//!
//! - `activate_scheduled_votes` — flip scheduled votes to active at start time
//!   and notify eligible owners (Story 106.2).
//! - `close_expired_votes` — close votes past their deadline and notify
//!   participants of results (Story 106.2).
//! - `send_vote_reminders` — remind eligible owners who have not yet voted on
//!   votes ending within the reminder window (Story 106.3).
//!
//! The three jobs above are the public entry points invoked from
//! `Scheduler::run_scheduled_tasks` in the parent module; the remaining
//! methods are private target-resolution helpers. The jobs are attached to
//! `Scheduler` via the `impl` block below.

use super::Scheduler;

impl Scheduler {
    // ========================================================================
    // Story 106.2: Vote Expiry Handler
    // ========================================================================

    /// Activate scheduled votes that have reached their start time.
    pub(super) async fn activate_scheduled_votes(&self) -> Result<(), sqlx::Error> {
        let activated = self.vote_repo.activate_scheduled_votes().await?;

        if !activated.is_empty() {
            tracing::info!(
                "Activated {} scheduled vote(s): {:?}",
                activated.len(),
                activated.iter().map(|v| v.id).collect::<Vec<_>>()
            );

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap_or_else(|e| e.into_inner());
                metrics.votes_activated += activated.len() as u64;
            }

            // Send notifications for each activated vote
            for vote in &activated {
                // A DB error resolving the eligible voters must be surfaced
                // (not coerced into an empty target set) so a failed dispatch
                // is observable — mirrors `send_vote_reminders`.
                let eligible_user_ids = match self.get_vote_eligible_users(vote.building_id).await {
                    Ok(ids) => ids,
                    Err(e) => {
                        tracing::error!(
                            vote_id = %vote.id,
                            building_id = %vote.building_id,
                            error = %e,
                            "Failed to resolve vote-started notification targets; \
                             skipping dispatch for this vote"
                        );
                        Vec::new()
                    }
                };

                if !eligible_user_ids.is_empty() {
                    match self
                        .notification_service
                        .notify_vote_started(vote, &eligible_user_ids)
                        .await
                    {
                        Ok(sent) => {
                            tracing::info!(
                                vote_id = %vote.id,
                                sent_count = sent,
                                "Sent vote started notifications"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                vote_id = %vote.id,
                                error = %e,
                                "Failed to send vote started notifications"
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Close expired votes and send result notifications.
    pub(super) async fn close_expired_votes(&self) -> Result<(), sqlx::Error> {
        let closed_ids = self.vote_repo.close_expired_votes().await?;

        if !closed_ids.is_empty() {
            tracing::info!(
                "Closed {} expired vote(s): {:?}",
                closed_ids.len(),
                closed_ids
            );

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap_or_else(|e| e.into_inner());
                metrics.votes_closed += closed_ids.len() as u64;
            }

            // Batch-fetch participants for all closed votes in a single query
            // (was N+1: one SELECT per closed vote). Group by vote_id so each
            // iteration below has its participant list ready in memory.
            let participants_by_vote: std::collections::HashMap<uuid::Uuid, Vec<uuid::Uuid>> =
                match self.get_vote_participants_batch(&closed_ids).await {
                    Ok(map) => map,
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            "Failed to batch-fetch vote participants; falling back to empty map"
                        );
                        std::collections::HashMap::new()
                    }
                };

            // Send notifications for each closed vote
            for vote_id in &closed_ids {
                // Get the vote with results
                // Note: Scheduler runs in background without user context - RLS not applicable
                #[allow(deprecated)]
                let vote_result = self.vote_repo.find_by_id(*vote_id).await;
                #[allow(deprecated)]
                let results_result = self.vote_repo.get_results(*vote_id).await;
                // Surface DB errors on the vote/result lookups instead of
                // letting `if let Ok(Some(..))` silently drop the `Err` arm —
                // otherwise a failed closed-vote dispatch is invisible.
                // `Ok(None)` (vote genuinely gone) stays a silent skip.
                if let Err(e) = &vote_result {
                    tracing::error!(
                        vote_id = %vote_id,
                        error = %e,
                        "Failed to load closed vote for result notification; \
                         skipping dispatch for this vote"
                    );
                }
                if let Err(e) = &results_result {
                    tracing::error!(
                        vote_id = %vote_id,
                        error = %e,
                        "Failed to load vote results for result notification; \
                         skipping dispatch for this vote"
                    );
                }
                if let Ok(Some(vote)) = vote_result {
                    if let Ok(Some(results)) = results_result {
                        // Participants pre-fetched by batch query above.
                        let participant_ids = participants_by_vote
                            .get(vote_id)
                            .cloned()
                            .unwrap_or_default();

                        if !participant_ids.is_empty() {
                            match self
                                .notification_service
                                .notify_vote_closed(&vote, &results, &participant_ids)
                                .await
                            {
                                Ok(sent) => {
                                    tracing::info!(
                                        vote_id = %vote_id,
                                        sent_count = sent,
                                        "Sent vote closed notifications"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        vote_id = %vote_id,
                                        error = %e,
                                        "Failed to send vote closed notifications"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Get eligible users for a vote (owners in the building).
    async fn get_vote_eligible_users(
        &self,
        building_id: uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>, sqlx::Error> {
        let users: Vec<(uuid::Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT ur.user_id
            FROM unit_residents ur
            JOIN units u ON ur.unit_id = u.id
            WHERE u.building_id = $1
              AND ur.resident_type = 'owner'
              AND ur.end_date IS NULL
            "#,
        )
        .bind(building_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(users.into_iter().map(|(id,)| id).collect())
    }

    /// Get users who participated in a vote.
    ///
    /// Retained for ad-hoc single-vote use; the closed-vote notification
    /// loop now prefers `get_vote_participants_batch` to avoid N+1.
    #[allow(dead_code)]
    async fn get_vote_participants(
        &self,
        vote_id: uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>, sqlx::Error> {
        let users: Vec<(uuid::Uuid,)> =
            sqlx::query_as("SELECT DISTINCT user_id FROM vote_responses WHERE vote_id = $1")
                .bind(vote_id)
                .fetch_all(&self.pool)
                .await?;

        Ok(users.into_iter().map(|(id,)| id).collect())
    }

    /// Batched variant of `get_vote_participants` — fetches participants for
    /// many votes in a single query and groups by `vote_id`. Used by the
    /// closed-vote notification loop to avoid N+1 round-trips.
    async fn get_vote_participants_batch(
        &self,
        vote_ids: &[uuid::Uuid],
    ) -> Result<std::collections::HashMap<uuid::Uuid, Vec<uuid::Uuid>>, sqlx::Error> {
        if vote_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let rows: Vec<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
            r#"
            SELECT DISTINCT vote_id, user_id
            FROM vote_responses
            WHERE vote_id = ANY($1)
            "#,
        )
        .bind(vote_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut map: std::collections::HashMap<uuid::Uuid, Vec<uuid::Uuid>> =
            std::collections::HashMap::new();
        for (vote_id, user_id) in rows {
            map.entry(vote_id).or_default().push(user_id);
        }
        Ok(map)
    }

    /// Get eligible users who have NOT voted yet.
    ///
    /// Retained for ad-hoc single-vote use; the reminder loop now prefers
    /// `get_users_not_voted_batch` to avoid N+1.
    #[allow(dead_code)]
    async fn get_users_not_voted(
        &self,
        vote_id: uuid::Uuid,
        building_id: uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>, sqlx::Error> {
        let users: Vec<(uuid::Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT ur.user_id
            FROM unit_residents ur
            JOIN units u ON ur.unit_id = u.id
            WHERE u.building_id = $1
              AND ur.resident_type = 'owner'
              AND ur.end_date IS NULL
              AND NOT EXISTS (
                  SELECT 1 FROM vote_responses vr
                  WHERE vr.vote_id = $2 AND vr.unit_id = ur.unit_id
              )
            "#,
        )
        .bind(building_id)
        .bind(vote_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(users.into_iter().map(|(id,)| id).collect())
    }

    /// Batched variant of `get_users_not_voted` — for each vote in `votes`,
    /// resolves the eligible owners in its building who have not yet voted,
    /// in a single query. Avoids N+1 round-trips when the scheduler is
    /// reminding many concurrently-expiring votes.
    async fn get_users_not_voted_batch(
        &self,
        votes: &[(uuid::Uuid, uuid::Uuid)],
    ) -> Result<std::collections::HashMap<uuid::Uuid, Vec<uuid::Uuid>>, sqlx::Error> {
        if votes.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let vote_ids: Vec<uuid::Uuid> = votes.iter().map(|(v, _)| *v).collect();

        // Join votes -> unit_residents via the vote's building_id; filter out
        // residents who have already voted via NOT EXISTS. Returns one row per
        // (vote_id, user_id) pair, which we group below.
        let rows: Vec<(uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
            r#"
            SELECT DISTINCT v.id as vote_id, ur.user_id
            FROM votes v
            JOIN units u ON u.building_id = v.building_id
            JOIN unit_residents ur ON ur.unit_id = u.id
            WHERE v.id = ANY($1)
              AND ur.resident_type = 'owner'
              AND ur.end_date IS NULL
              AND NOT EXISTS (
                  SELECT 1 FROM vote_responses vr
                  WHERE vr.vote_id = v.id AND vr.unit_id = ur.unit_id
              )
            "#,
        )
        .bind(&vote_ids)
        .fetch_all(&self.pool)
        .await?;

        let mut map: std::collections::HashMap<uuid::Uuid, Vec<uuid::Uuid>> =
            std::collections::HashMap::new();
        for (vote_id, user_id) in rows {
            map.entry(vote_id).or_default().push(user_id);
        }
        Ok(map)
    }

    // ========================================================================
    // Story 106.3: Reminder Notifications
    // ========================================================================

    /// Send reminders for votes ending soon.
    pub(super) async fn send_vote_reminders(&self) -> Result<(), sqlx::Error> {
        // Find active votes ending within the reminder window
        let reminder_cutoff =
            chrono::Utc::now() + chrono::Duration::days(self.config.vote_reminder_days_before);

        let votes_ending_soon: Vec<db::models::Vote> = sqlx::query_as(
            r#"
            SELECT * FROM votes
            WHERE status = 'active'
              AND end_at <= $1
              AND end_at > NOW()
            "#,
        )
        .bind(reminder_cutoff)
        .fetch_all(&self.pool)
        .await?;

        let mut reminders_sent = 0u64;

        // Batch-fetch "users not yet voted" for all expiring votes in one
        // query (was N+1: one SELECT per vote). The helper returns a map
        // keyed by vote_id so the per-vote loop below stays O(1) lookup.
        let vote_pairs: Vec<(uuid::Uuid, uuid::Uuid)> = votes_ending_soon
            .iter()
            .map(|v| (v.id, v.building_id))
            .collect();
        let not_voted_by_vote = match self.get_users_not_voted_batch(&vote_pairs).await {
            Ok(map) => map,
            Err(e) => {
                tracing::error!(
                    error = %e,
                    "Failed to batch-fetch users-not-voted; falling back to empty map"
                );
                std::collections::HashMap::new()
            }
        };

        for vote in votes_ending_soon {
            // Pre-fetched via batch query above.
            let users_not_voted = not_voted_by_vote.get(&vote.id).cloned().unwrap_or_default();

            if !users_not_voted.is_empty() {
                match self
                    .notification_service
                    .notify_vote_reminder(&vote, &users_not_voted)
                    .await
                {
                    Ok(sent) => {
                        reminders_sent += sent as u64;
                        tracing::info!(
                            vote_id = %vote.id,
                            sent_count = sent,
                            "Sent vote reminder notifications"
                        );
                    }
                    Err(e) => {
                        tracing::error!(
                            vote_id = %vote.id,
                            error = %e,
                            "Failed to send vote reminder notifications"
                        );
                    }
                }
            }
        }

        if reminders_sent > 0 {
            let mut metrics = self.metrics.lock().unwrap_or_else(|e| e.into_inner());
            metrics.vote_reminders_sent += reminders_sent;
        }

        Ok(())
    }
}
