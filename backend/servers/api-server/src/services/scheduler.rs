//! Background scheduler service for periodic tasks (Epic 106).
//!
//! Handles scheduled announcements publishing, vote management, reminders,
//! and session cleanup.

use db::repositories::{
    AnnouncementRepository, MeterRepository, SessionRepository, SignatureRequestRepository,
    UnitResidentRepository, VoteRepository,
};
use db::DbPool;
use integrations::LightweightProvider;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::Instrument;

use super::notification::{NotificationService, NotificationServiceConfig};
use super::EmailService;

/// Scheduler service configuration.
#[derive(Clone)]
pub struct SchedulerConfig {
    /// Interval between scheduler runs (in seconds).
    pub interval_secs: u64,
    /// Whether the scheduler is enabled.
    pub enabled: bool,
    /// Days before vote end to send reminder (default: 1).
    pub vote_reminder_days_before: i64,
    /// Days before meter reading due to send reminder (default: 3).
    pub meter_reminder_days_before: i64,
    /// Days before payment due to send reminder (default: 7).
    pub payment_reminder_days_before: i64,
    /// Days before signature request expiry to send reminder (default: 3).
    pub signature_reminder_days_before: i64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval_secs: 60, // Check every minute
            enabled: true,
            vote_reminder_days_before: 1,
            meter_reminder_days_before: 3,
            payment_reminder_days_before: 7,
            signature_reminder_days_before: 3,
        }
    }
}

/// Metrics for scheduler operations.
#[derive(Debug, Default)]
pub struct SchedulerMetrics {
    pub announcements_published: u64,
    pub votes_activated: u64,
    pub votes_closed: u64,
    pub vote_reminders_sent: u64,
    pub meter_reminders_sent: u64,
    pub payment_reminders_sent: u64,
    pub signature_reminders_sent: u64,
    pub signature_requests_expired: u64,
    pub sessions_cleaned: u64,
    pub login_attempts_cleaned: u64,
    pub errors: u64,
}

/// Background scheduler for periodic tasks.
pub struct Scheduler {
    pool: DbPool,
    announcement_repo: AnnouncementRepository,
    vote_repo: VoteRepository,
    session_repo: SessionRepository,
    meter_repo: MeterRepository,
    unit_resident_repo: UnitResidentRepository,
    signature_request_repo: SignatureRequestRepository,
    notification_service: Arc<NotificationService>,
    email_service: EmailService,
    config: SchedulerConfig,
    metrics: std::sync::Mutex<SchedulerMetrics>,
}

impl Scheduler {
    /// Create a new scheduler.
    pub fn new(
        pool: DbPool,
        announcement_repo: AnnouncementRepository,
        config: SchedulerConfig,
    ) -> Self {
        let email_service = EmailService::development();
        let notification_service = Arc::new(NotificationService::new(
            pool.clone(),
            email_service.clone(),
            NotificationServiceConfig::default(),
        ));

        Self {
            vote_repo: VoteRepository::new(pool.clone()),
            session_repo: SessionRepository::new(pool.clone()),
            meter_repo: MeterRepository::new(pool.clone()),
            unit_resident_repo: UnitResidentRepository::new(pool.clone()),
            signature_request_repo: SignatureRequestRepository::new(pool.clone()),
            pool,
            announcement_repo,
            notification_service,
            email_service,
            config,
            metrics: std::sync::Mutex::new(SchedulerMetrics::default()),
        }
    }

    /// Create a scheduler with a custom notification service and email service.
    pub fn with_notification_service(
        pool: DbPool,
        announcement_repo: AnnouncementRepository,
        notification_service: Arc<NotificationService>,
        config: SchedulerConfig,
    ) -> Self {
        let email_service = EmailService::development();
        Self {
            vote_repo: VoteRepository::new(pool.clone()),
            session_repo: SessionRepository::new(pool.clone()),
            meter_repo: MeterRepository::new(pool.clone()),
            unit_resident_repo: UnitResidentRepository::new(pool.clone()),
            signature_request_repo: SignatureRequestRepository::new(pool.clone()),
            pool,
            announcement_repo,
            notification_service,
            email_service,
            config,
            metrics: std::sync::Mutex::new(SchedulerMetrics::default()),
        }
    }

    /// Create a scheduler with a custom email service (for production use).
    pub fn with_email_service(mut self, email_service: EmailService) -> Self {
        self.email_service = email_service;
        self
    }

    /// Get current metrics.
    pub fn get_metrics(&self) -> SchedulerMetrics {
        let guard = self.metrics.lock().unwrap();
        SchedulerMetrics {
            announcements_published: guard.announcements_published,
            votes_activated: guard.votes_activated,
            votes_closed: guard.votes_closed,
            vote_reminders_sent: guard.vote_reminders_sent,
            meter_reminders_sent: guard.meter_reminders_sent,
            payment_reminders_sent: guard.payment_reminders_sent,
            signature_reminders_sent: guard.signature_reminders_sent,
            signature_requests_expired: guard.signature_requests_expired,
            sessions_cleaned: guard.sessions_cleaned,
            login_attempts_cleaned: guard.login_attempts_cleaned,
            errors: guard.errors,
        }
    }

    /// Start the scheduler background loop.
    ///
    /// This spawns a tokio task that runs indefinitely,
    /// checking for scheduled tasks at the configured interval.
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        let interval_secs = self.config.interval_secs;
        tokio::spawn(
            async move {
                if !self.config.enabled {
                    tracing::info!("Scheduler disabled, not starting background tasks");
                    return;
                }

                tracing::info!(
                    "Starting background scheduler with {}s interval",
                    self.config.interval_secs
                );

                let mut ticker = interval(Duration::from_secs(self.config.interval_secs));

                loop {
                    ticker.tick().await;
                    self.run_scheduled_tasks().await;
                }
            }
            .instrument(tracing::info_span!(
                "bg.scheduler_tick",
                interval_secs = interval_secs,
            )),
        )
    }

    /// Run all scheduled tasks.
    async fn run_scheduled_tasks(&self) {
        // Story 106.1: Publish scheduled announcements and send notifications
        if let Err(e) = self.publish_scheduled_announcements().await {
            tracing::error!("Failed to publish scheduled announcements: {}", e);
            self.increment_errors();
        }

        // Story 106.2: Activate scheduled votes
        if let Err(e) = self.activate_scheduled_votes().await {
            tracing::error!("Failed to activate scheduled votes: {}", e);
            self.increment_errors();
        }

        // Story 106.2: Close expired votes and notify results
        if let Err(e) = self.close_expired_votes().await {
            tracing::error!("Failed to close expired votes: {}", e);
            self.increment_errors();
        }

        // Story 106.3: Send vote reminders
        if let Err(e) = self.send_vote_reminders().await {
            tracing::error!("Failed to send vote reminders: {}", e);
            self.increment_errors();
        }

        // Story 106.4: Clean up expired sessions
        if let Err(e) = self.cleanup_sessions().await {
            tracing::error!("Failed to cleanup sessions: {}", e);
            self.increment_errors();
        }

        // Story 84.2: Expire overdue signature requests
        if let Err(e) = self.expire_signature_requests().await {
            tracing::error!("Failed to expire signature requests: {}", e);
            self.increment_errors();
        }

        // Story 84.2: Send signature reminder emails
        if let Err(e) = self.send_signature_reminders().await {
            tracing::error!("Failed to send signature reminders: {}", e);
            self.increment_errors();
        }
    }

    // ========================================================================
    // Story 106.1: Announcement Notification Triggers
    // ========================================================================

    /// Publish all scheduled announcements that are due and send notifications.
    async fn publish_scheduled_announcements(&self) -> Result<(), sqlx::Error> {
        // Note: Scheduler runs in background without user context.
        // These operations are privileged/admin-level and don't need RLS enforcement.
        #[allow(deprecated)]
        let published = self.announcement_repo.publish_scheduled().await?;

        if !published.is_empty() {
            tracing::info!(
                "Published {} scheduled announcement(s): {:?}",
                published.len(),
                published.iter().map(|a| a.id).collect::<Vec<_>>()
            );

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.announcements_published += published.len() as u64;
            }

            // Send notifications for each published announcement
            for announcement in &published {
                // Get target users based on target_type and target_ids
                let target_user_ids = self
                    .get_announcement_target_users(announcement)
                    .await
                    .unwrap_or_default();

                if !target_user_ids.is_empty() {
                    match self
                        .notification_service
                        .notify_announcement_published(announcement, &target_user_ids)
                        .await
                    {
                        Ok(sent) => {
                            tracing::info!(
                                announcement_id = %announcement.id,
                                sent_count = sent,
                                "Sent announcement notifications"
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                announcement_id = %announcement.id,
                                error = %e,
                                "Failed to send announcement notifications"
                            );
                        }
                    }
                }

                tracing::info!(
                    announcement_id = %announcement.id,
                    title = %announcement.title,
                    target_type = %announcement.target_type,
                    "Scheduled announcement published"
                );
            }
        }

        Ok(())
    }

    /// Get target users for an announcement based on target_type and target_ids.
    async fn get_announcement_target_users(
        &self,
        announcement: &db::models::Announcement,
    ) -> Result<Vec<uuid::Uuid>, sqlx::Error> {
        // Parse target_ids from JSON
        let target_ids: Vec<uuid::Uuid> =
            serde_json::from_value(announcement.target_ids.clone()).unwrap_or_default();

        match announcement.target_type.as_str() {
            "all" => {
                // Get all users in the organization. `users` has no
                // `organization_id` column — membership lives in
                // `user_memberships` (migration 00128, the canonical authz
                // spine). Join through it and filter to active grants.
                let users: Vec<(uuid::Uuid,)> = sqlx::query_as(
                    r#"
                    SELECT DISTINCT u.id
                    FROM users u
                    JOIN user_memberships m ON m.user_id = u.id
                    WHERE m.organization_id = $1
                      AND m.revoked_at IS NULL
                      AND (m.expires_at IS NULL OR m.expires_at > NOW())
                      AND u.status = 'active'
                    "#,
                )
                .bind(announcement.organization_id)
                .fetch_all(&self.pool)
                .await?;

                Ok(users.into_iter().map(|(id,)| id).collect())
            }
            "building" => {
                // Get all users associated with the specified buildings.
                // Single query across all target buildings (was N+1: one
                // SELECT per building). DISTINCT collapses duplicates that
                // existed across the old per-building results.
                if target_ids.is_empty() {
                    return Ok(Vec::new());
                }
                let users: Vec<(uuid::Uuid,)> = sqlx::query_as(
                    r#"
                    SELECT DISTINCT ur.user_id
                    FROM unit_residents ur
                    JOIN units u ON ur.unit_id = u.id
                    WHERE u.building_id = ANY($1) AND ur.move_out_date IS NULL
                    "#,
                )
                .bind(&target_ids)
                .fetch_all(&self.pool)
                .await?;
                Ok(users.into_iter().map(|(id,)| id).collect())
            }
            "units" => {
                // Get all users associated with the specified units
                let users: Vec<(uuid::Uuid,)> = sqlx::query_as(
                    r#"
                    SELECT DISTINCT user_id FROM unit_residents
                    WHERE unit_id = ANY($1) AND move_out_date IS NULL
                    "#,
                )
                .bind(&target_ids)
                .fetch_all(&self.pool)
                .await?;

                Ok(users.into_iter().map(|(id,)| id).collect())
            }
            "roles" => {
                // Get all users with the specified roles in the organization
                // Role IDs would be stored in target_ids
                let users: Vec<(uuid::Uuid,)> = sqlx::query_as(
                    r#"
                    SELECT DISTINCT om.user_id
                    FROM organization_members om
                    WHERE om.organization_id = $1
                      AND om.role_id = ANY($2)
                      AND om.status = 'active'
                    "#,
                )
                .bind(announcement.organization_id)
                .bind(&target_ids)
                .fetch_all(&self.pool)
                .await?;

                Ok(users.into_iter().map(|(id,)| id).collect())
            }
            _ => {
                tracing::warn!(
                    target_type = %announcement.target_type,
                    "Unknown announcement target type"
                );
                Ok(Vec::new())
            }
        }
    }

    // ========================================================================
    // Story 106.2: Vote Expiry Handler
    // ========================================================================

    /// Activate scheduled votes that have reached their start time.
    async fn activate_scheduled_votes(&self) -> Result<(), sqlx::Error> {
        let activated = self.vote_repo.activate_scheduled_votes().await?;

        if !activated.is_empty() {
            tracing::info!(
                "Activated {} scheduled vote(s): {:?}",
                activated.len(),
                activated.iter().map(|v| v.id).collect::<Vec<_>>()
            );

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap();
                metrics.votes_activated += activated.len() as u64;
            }

            // Send notifications for each activated vote
            for vote in &activated {
                let eligible_user_ids = self
                    .get_vote_eligible_users(vote.building_id)
                    .await
                    .unwrap_or_default();

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
    async fn close_expired_votes(&self) -> Result<(), sqlx::Error> {
        let closed_ids = self.vote_repo.close_expired_votes().await?;

        if !closed_ids.is_empty() {
            tracing::info!(
                "Closed {} expired vote(s): {:?}",
                closed_ids.len(),
                closed_ids
            );

            // Update metrics
            {
                let mut metrics = self.metrics.lock().unwrap();
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
              AND ur.move_out_date IS NULL
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
              AND ur.move_out_date IS NULL
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
              AND ur.move_out_date IS NULL
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
    async fn send_vote_reminders(&self) -> Result<(), sqlx::Error> {
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
            let mut metrics = self.metrics.lock().unwrap();
            metrics.vote_reminders_sent += reminders_sent;
        }

        Ok(())
    }

    // ========================================================================
    // Story 106.4: Session Cleanup Task
    // ========================================================================

    /// Cleanup expired sessions and old login attempts.
    async fn cleanup_sessions(&self) -> Result<(), sqlx::Error> {
        // Cleanup expired refresh tokens
        let tokens_cleaned = self.session_repo.cleanup_expired_tokens().await?;

        // Cleanup old login attempts
        let attempts_cleaned = self.session_repo.cleanup_old_attempts().await?;

        if tokens_cleaned > 0 || attempts_cleaned > 0 {
            tracing::info!(
                tokens_cleaned = tokens_cleaned,
                attempts_cleaned = attempts_cleaned,
                "Session cleanup completed"
            );

            let mut metrics = self.metrics.lock().unwrap();
            metrics.sessions_cleaned += tokens_cleaned;
            metrics.login_attempts_cleaned += attempts_cleaned;
        }

        Ok(())
    }

    // ========================================================================
    // Story 84.2: E-Signature Reminder & Expiry Tasks
    // ========================================================================

    /// Expire overdue signature requests (status pending/in_progress, expires_at < now).
    async fn expire_signature_requests(&self) -> Result<(), sqlx::Error> {
        let expired_count = self.signature_request_repo.expire_old_requests().await?;
        if expired_count > 0 {
            tracing::info!(expired_count = expired_count, "Expired overdue signature requests");
            let mut metrics = self.metrics.lock().unwrap();
            metrics.signature_requests_expired += expired_count as u64;
        }
        Ok(())
    }

    /// Send reminder emails for pending signature requests approaching expiry.
    async fn send_signature_reminders(&self) -> Result<(), sqlx::Error> {
        let cutoff = chrono::Utc::now()
            + chrono::Duration::days(self.config.signature_reminder_days_before);

        let expiring_requests: Vec<db::models::SignatureRequest> = sqlx::query_as(
            r#"
            SELECT * FROM signature_requests
            WHERE status IN ('pending', 'in_progress')
              AND expires_at IS NOT NULL
              AND expires_at > NOW()
              AND expires_at <= $1
            "#,
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await?;

        if expiring_requests.is_empty() {
            return Ok(());
        }

        tracing::info!(
            count = expiring_requests.len(),
            reminder_window_days = self.config.signature_reminder_days_before,
            "Processing signature requests approaching expiry for reminders"
        );

        let provider = LightweightProvider::from_env();
        let base_url = std::env::var("BASE_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());

        let mut total_reminders = 0u64;

        for sig_req in &expiring_requests {
            let pending_signers: Vec<_> = sig_req
                .signers
                .iter()
                .filter(|s| !s.is_complete())
                .collect();

            if pending_signers.is_empty() {
                continue;
            }

            let expires_str = sig_req.expires_at.map(|e| e.format("%Y-%m-%d").to_string());
            let doc_label = sig_req
                .subject
                .clone()
                .unwrap_or_else(|| "Document".to_string());

            for signer in pending_signers {
                let sign_url = provider
                    .build_signing_url(sig_req.id, &signer.email)
                    .unwrap_or_else(|_| {
                        format!(
                            "{}/sign?request_id={}&email={}",
                            base_url.trim_end_matches('/'),
                            sig_req.id,
                            signer.email
                        )
                    });

                match self
                    .email_service
                    .send_signature_reminder_email(
                        &signer.email,
                        &signer.name,
                        &doc_label,
                        &sign_url,
                        expires_str.as_deref(),
                    )
                    .await
                {
                    Ok(()) => {
                        tracing::info!(
                            signature_request_id = %sig_req.id,
                            signer_email = %signer.email,
                            "Sent scheduled signature reminder email"
                        );
                        total_reminders += 1;
                    }
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            signature_request_id = %sig_req.id,
                            signer_email = %signer.email,
                            "Failed to send scheduled signature reminder email"
                        );
                    }
                }
            }
        }

        if total_reminders > 0 {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.signature_reminders_sent += total_reminders;
        }

        Ok(())
    }

    /// Helper to increment error count in metrics.
    fn increment_errors(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.errors += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.interval_secs, 60);
        assert!(config.enabled);
        assert_eq!(config.vote_reminder_days_before, 1);
    }

    #[test]
    fn test_scheduler_metrics_default() {
        let metrics = SchedulerMetrics::default();
        assert_eq!(metrics.announcements_published, 0);
        assert_eq!(metrics.votes_closed, 0);
        assert_eq!(metrics.errors, 0);
    }
}
