//! Background scheduler service for periodic tasks (Epic 106).
//!
//! Handles scheduled announcements publishing, vote management, reminders,
//! and session cleanup.

use db::repositories::{
    AnnouncementRepository, ESignatureNonceRepository, FinancialRepository, MeterRepository,
    ReportScheduleRepository, SessionRepository, SignatureRequestRepository,
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

/// Per-signer minimum interval between reminder emails sent by the
/// background scheduler. Prevents the every-60s scheduler tick from
/// spamming the same signer for the entire reminder window.
const SIGNATURE_REMINDER_MIN_INTERVAL_HOURS: i64 = 12;

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
    /// Maximum age (in days) a pinned announcement is kept pinned before the
    /// scheduler auto-unpins it (default: 30, issue #972.7).
    pub pin_max_age_days: i64,
    /// Grace period (in days) after `due_date` before an invoice is
    /// transitioned to `overdue` (default: 0 — transition on the due date).
    pub overdue_grace_period_days: i64,
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
            pin_max_age_days: 30,
            overdue_grace_period_days: 0,
        }
    }
}

/// Metrics for scheduler operations.
#[derive(Debug, Default)]
pub struct SchedulerMetrics {
    pub announcements_published: u64,
    pub announcements_unpinned: u64,
    pub votes_activated: u64,
    pub votes_closed: u64,
    pub vote_reminders_sent: u64,
    pub meter_reminders_sent: u64,
    pub payment_reminders_sent: u64,
    pub invoices_transitioned_to_overdue: u64,
    pub signature_reminders_sent: u64,
    pub signature_requests_expired: u64,
    pub sessions_cleaned: u64,
    pub login_attempts_cleaned: u64,
    /// Report schedules fired by the due-work consumer (issue #2303).
    pub report_schedules_fired: u64,
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
    e_signature_nonce_repo: ESignatureNonceRepository,
    financial_repo: FinancialRepository,
    report_schedule_repo: ReportScheduleRepository,
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
            e_signature_nonce_repo: ESignatureNonceRepository::new(pool.clone()),
            financial_repo: FinancialRepository::new(pool.clone()),
            report_schedule_repo: ReportScheduleRepository::new(pool.clone()),
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
            e_signature_nonce_repo: ESignatureNonceRepository::new(pool.clone()),
            financial_repo: FinancialRepository::new(pool.clone()),
            report_schedule_repo: ReportScheduleRepository::new(pool.clone()),
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
            announcements_unpinned: guard.announcements_unpinned,
            votes_activated: guard.votes_activated,
            votes_closed: guard.votes_closed,
            vote_reminders_sent: guard.vote_reminders_sent,
            meter_reminders_sent: guard.meter_reminders_sent,
            payment_reminders_sent: guard.payment_reminders_sent,
            invoices_transitioned_to_overdue: guard.invoices_transitioned_to_overdue,
            signature_reminders_sent: guard.signature_reminders_sent,
            signature_requests_expired: guard.signature_requests_expired,
            sessions_cleaned: guard.sessions_cleaned,
            login_attempts_cleaned: guard.login_attempts_cleaned,
            report_schedules_fired: guard.report_schedules_fired,
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

        // Story 6.4 (issue #972.7): Auto-unpin announcements pinned too long
        if let Err(e) = self.auto_unpin_expired_announcements().await {
            tracing::error!("Failed to auto-unpin expired announcements: {}", e);
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

        // Story 12.2: Send meter reading reminders to residents before window closes
        if let Err(e) = self.send_meter_reminders().await {
            tracing::error!("Failed to send meter reading reminders: {}", e);
            self.increment_errors();
        }

        // Story 11.6: Send payment due reminders
        if let Err(e) = self.send_payment_reminders().await {
            tracing::error!("Failed to send payment reminders: {}", e);
            self.increment_errors();
        }

        // Story 11.6: Transition overdue invoices and fire escalation
        if let Err(e) = self.transition_overdue_invoices().await {
            tracing::error!("Failed to transition overdue invoices: {}", e);
            self.increment_errors();
        }

        // Issue #2303: fire due report schedules and advance next_run_at.
        if let Err(e) = self.fire_due_report_schedules().await {
            tracing::error!("Failed to fire due report schedules: {}", e);
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

    /// Auto-unpin announcements that have been pinned longer than the
    /// configured maximum age (Story 6.4 / issue #972.7).
    async fn auto_unpin_expired_announcements(&self) -> Result<(), sqlx::Error> {
        // Note: Scheduler runs in background without user context.
        // These operations are privileged/admin-level and don't need RLS enforcement.
        let max_age = chrono::Duration::days(self.config.pin_max_age_days);
        let unpinned = self.announcement_repo.auto_unpin_expired(max_age).await?;

        if !unpinned.is_empty() {
            tracing::info!(
                "Auto-unpinned {} announcement(s) pinned longer than {} day(s): {:?}",
                unpinned.len(),
                self.config.pin_max_age_days,
                unpinned
            );

            let mut metrics = self.metrics.lock().unwrap();
            metrics.announcements_unpinned += unpinned.len() as u64;
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
                    WHERE u.building_id = ANY($1) AND ur.end_date IS NULL
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
                    WHERE unit_id = ANY($1) AND end_date IS NULL
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
            tracing::info!(
                expired_count = expired_count,
                "Expired overdue signature requests"
            );
            let mut metrics = self.metrics.lock().unwrap();
            metrics.signature_requests_expired += expired_count as u64;
        }
        Ok(())
    }

    /// Send reminder emails for pending signature requests approaching expiry.
    async fn send_signature_reminders(&self) -> Result<(), sqlx::Error> {
        let cutoff =
            chrono::Utc::now() + chrono::Duration::days(self.config.signature_reminder_days_before);

        let expiring_requests = self
            .signature_request_repo
            .find_expiring_in_window(cutoff)
            .await?;

        if expiring_requests.is_empty() {
            return Ok(());
        }

        tracing::info!(
            count = expiring_requests.len(),
            reminder_window_days = self.config.signature_reminder_days_before,
            "Processing signature requests approaching expiry for reminders"
        );

        let provider = match LightweightProvider::from_env() {
            Ok(p) => p,
            Err(e) => {
                // Refuse to mint reminder URLs when the HMAC secret is
                // missing — startup validation ensures this should never
                // fire in production.
                tracing::error!(error = %e, "Skipping signature-reminder tick — e-signature provider misconfigured");
                return Ok(());
            }
        };
        let _base_url =
            std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

        // Per-signer minimum interval between reminders. The scheduler ticks
        // every 60s by default; without this gate every pending signer
        // would get spammed on every tick for the entire reminder window.
        let now = chrono::Utc::now();
        let min_interval = chrono::Duration::hours(SIGNATURE_REMINDER_MIN_INTERVAL_HOURS);

        let mut total_reminders = 0u64;

        for sig_req in &expiring_requests {
            let pending_signers: Vec<_> = sig_req
                .signers
                .iter()
                .filter(|s| !s.is_complete())
                .filter(|s| {
                    // Skip signers reminded within the dedup window.
                    s.last_reminder_at
                        .map(|t| now.signed_duration_since(t) >= min_interval)
                        .unwrap_or(true)
                })
                .collect();

            if pending_signers.is_empty() {
                continue;
            }

            let expires_str = sig_req.expires_at.map(|e| e.format("%Y-%m-%d").to_string());
            let doc_label = sig_req
                .subject
                .clone()
                .unwrap_or_else(|| "Document".to_string());

            let org_id_str = sig_req.organization_id.to_string();
            for signer in pending_signers {
                let signer_status = signer.status.to_string();
                let sign_url = match provider.build_signing_url(
                    &signer.email,
                    &sig_req.id.to_string(),
                    &org_id_str,
                    &signer_status,
                ) {
                    Ok(s) => {
                        // Persist the freshly-issued nonce so the future
                        // /sign consumer can reject a replay (issue #673).
                        if let Err(e) = self
                            .e_signature_nonce_repo
                            .record_nonce(sig_req.id, s.nonce)
                            .await
                        {
                            tracing::error!(
                                error = %e,
                                signature_request_id = %sig_req.id,
                                signer_email = %signer.email,
                                "Failed to persist e-signature nonce — skipping reminder for this signer"
                            );
                            continue;
                        }
                        s.url
                    }
                    Err(e) => {
                        tracing::error!(
                            error = %e,
                            signature_request_id = %sig_req.id,
                            signer_email = %signer.email,
                            "Failed to build signing URL — skipping reminder for this signer"
                        );
                        continue;
                    }
                };

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
                        // Stamp the signer so subsequent ticks within the
                        // dedup window skip them. Failure here is logged
                        // but not fatal — the next tick will re-evaluate.
                        if let Err(e) = self
                            .signature_request_repo
                            .touch_signer_reminder(sig_req.id, &signer.email, now)
                            .await
                        {
                            tracing::warn!(
                                error = %e,
                                signature_request_id = %sig_req.id,
                                signer_email = %signer.email,
                                "Failed to stamp last_reminder_at — next tick may resend"
                            );
                        }
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

    // ========================================================================
    // Story 12.2: Meter Reading Reminders
    // ========================================================================

    /// Notify residents of unit-linked meters whose building has a submission
    /// window closing soon (within `meter_reminder_days_before`).
    ///
    /// Each resident receives at most ONE reminder per window per tick — the
    /// per-window `notified` set dedupes a resident who sits on several meters in
    /// the same building (#1777).
    ///
    /// Known limitation (#1772 finding-2): this does NOT yet skip residents who
    /// have *already submitted* a reading for the window, nor dedupe across
    /// scheduler ticks — a resident is re-reminded on each run until the window
    /// closes. (An earlier version of this doc claimed "who have not yet
    /// submitted a reading"; that filtering was never implemented, so the doc is
    /// corrected here to match behavior.) Durable cross-tick dedup is tracked
    /// alongside the sibling payment-reminder work (#1769 / #1790).
    async fn send_meter_reminders(&self) -> Result<(), sqlx::Error> {
        let windows = self
            .meter_repo
            .find_windows_closing_soon(self.config.meter_reminder_days_before)
            .await?;

        if windows.is_empty() {
            return Ok(());
        }

        tracing::info!(
            count = windows.len(),
            reminder_window_days = self.config.meter_reminder_days_before,
            "Processing meter submission windows approaching close for reminders"
        );

        let mut total_reminders = 0u64;

        for window in &windows {
            // Fetch active unit meters for the building.
            let meters_page = match self
                .meter_repo
                .list_meters_for_building(window.building_id, 500, 0)
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(
                        building_id = %window.building_id,
                        error = %e,
                        "Failed to list meters for building — skipping window"
                    );
                    continue;
                }
            };

            let due_date = window
                .submission_end
                .and_hms_opt(23, 59, 59)
                .map(|ndt| {
                    chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(ndt, chrono::Utc)
                })
                .unwrap_or_else(chrono::Utc::now);

            // Dedup per window (#1777): a resident on multiple unit-linked meters
            // in the same building window must receive ONE reading reminder, not
            // one per meter.
            let mut notified = std::collections::HashSet::new();

            for meter in meters_page.meters.iter().filter(|m| m.unit_id.is_some()) {
                let unit_id = match meter.unit_id {
                    Some(id) => id,
                    None => continue,
                };

                let residents = match self.unit_resident_repo.find_by_unit(unit_id).await {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!(
                            unit_id = %unit_id,
                            error = %e,
                            "Failed to fetch residents for unit — skipping"
                        );
                        continue;
                    }
                };

                for resident in &residents {
                    // Skip residents already reminded for an earlier meter in
                    // this window (#1777).
                    if !notified.insert(resident.user_id) {
                        continue;
                    }
                    match self
                        .notification_service
                        .notify_meter_reading_due(
                            resident.user_id,
                            meter.id,
                            &meter.meter_number,
                            due_date,
                        )
                        .await
                    {
                        Ok(()) => {
                            total_reminders += 1;
                            tracing::info!(
                                meter_id = %meter.id,
                                user_id = %resident.user_id,
                                "Sent meter reading due notification"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                meter_id = %meter.id,
                                user_id = %resident.user_id,
                                error = %e,
                                "Failed to send meter reading due notification"
                            );
                        }
                    }
                }
            }
        }

        if total_reminders > 0 {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.meter_reminders_sent += total_reminders;
        }

        Ok(())
    }

    // ========================================================================
    // Story 11.6: Payment Reminder & Overdue Transition Tasks
    // ========================================================================

    /// Send payment-due reminder notifications for all `sent` invoices whose
    /// `due_date` falls within the next `payment_reminder_days_before` days.
    /// Notifications are dispatched per-resident of the invoice's unit via
    /// the existing `notify_payment_due` path (is_overdue = false).
    async fn send_payment_reminders(&self) -> Result<(), sqlx::Error> {
        let invoices = self
            .financial_repo
            .find_invoices_due_for_reminder(self.config.payment_reminder_days_before)
            .await?;

        if invoices.is_empty() {
            return Ok(());
        }

        tracing::info!(
            count = invoices.len(),
            reminder_window_days = self.config.payment_reminder_days_before,
            "Processing invoices approaching due date for payment reminders"
        );

        let mut total_sent = 0u64;

        for invoice in &invoices {
            let user_ids = self.get_unit_resident_user_ids(invoice.unit_id).await?;
            if user_ids.is_empty() {
                continue;
            }

            let amount_str = invoice.balance_due.to_string();
            let due_dt = invoice
                .due_date
                .and_hms_opt(0, 0, 0)
                .and_then(|dt| dt.and_local_timezone(chrono::Utc).single())
                .unwrap_or_else(chrono::Utc::now);

            let mut sent_for_invoice = 0u64;
            for user_id in &user_ids {
                match self
                    .notification_service
                    .notify_payment_due(*user_id, invoice.id, &amount_str, due_dt, false)
                    .await
                {
                    Ok(()) => {
                        total_sent += 1;
                        sent_for_invoice += 1;
                        tracing::info!(
                            invoice_id = %invoice.id,
                            user_id = %user_id,
                            "Sent payment reminder notification"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            invoice_id = %invoice.id,
                            user_id = %user_id,
                            error = %e,
                            "Failed to send payment reminder notification"
                        );
                    }
                }
            }

            // Persistent dedup (#1790): once at least one reminder for this
            // invoice has gone out, stamp it so the next ~60s tick skips it for
            // 24h instead of re-spamming the unit. A fully-failed invoice is
            // left unstamped so it is retried on the next tick.
            if sent_for_invoice > 0 {
                if let Err(e) = self
                    .financial_repo
                    .mark_payment_reminder_sent(invoice.id)
                    .await
                {
                    tracing::warn!(
                        invoice_id = %invoice.id,
                        error = %e,
                        "Failed to stamp last_payment_reminder_at — next tick may resend"
                    );
                }
            }
        }

        if total_sent > 0 {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.payment_reminders_sent += total_sent;
        }

        Ok(())
    }

    /// Transition `sent`/`partial` invoices past the grace period to `overdue`
    /// and fire escalation notifications for each affected invoice.
    async fn transition_overdue_invoices(&self) -> Result<(), sqlx::Error> {
        let transitioned = self
            .financial_repo
            .transition_invoices_to_overdue(self.config.overdue_grace_period_days)
            .await?;

        if transitioned.is_empty() {
            return Ok(());
        }

        tracing::info!(
            count = transitioned.len(),
            grace_period_days = self.config.overdue_grace_period_days,
            "Transitioned invoices to overdue status"
        );

        let mut escalations_sent = 0u64;

        for invoice in &transitioned {
            let user_ids = self.get_unit_resident_user_ids(invoice.unit_id).await?;
            if user_ids.is_empty() {
                continue;
            }

            let amount_str = invoice.balance_due.to_string();
            let due_dt = invoice
                .due_date
                .and_hms_opt(0, 0, 0)
                .and_then(|dt| dt.and_local_timezone(chrono::Utc).single())
                .unwrap_or_else(chrono::Utc::now);

            for user_id in &user_ids {
                match self
                    .notification_service
                    .notify_payment_due(*user_id, invoice.id, &amount_str, due_dt, true)
                    .await
                {
                    Ok(()) => {
                        escalations_sent += 1;
                        tracing::info!(
                            invoice_id = %invoice.id,
                            user_id = %user_id,
                            "Sent overdue escalation notification"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            invoice_id = %invoice.id,
                            user_id = %user_id,
                            error = %e,
                            "Failed to send overdue escalation notification"
                        );
                    }
                }
            }
        }

        {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.invoices_transitioned_to_overdue += transitioned.len() as u64;
        }

        tracing::info!(
            escalations_sent = escalations_sent,
            "Overdue invoice escalation notifications dispatched"
        );

        Ok(())
    }

    /// Fetch the active resident `user_id`s for a given unit. Used by the
    /// payment-reminder and overdue-transition tasks to resolve notification
    /// targets without crossing tenant boundaries (invoices are scoped to a
    /// single unit within a single organization).
    async fn get_unit_resident_user_ids(
        &self,
        unit_id: uuid::Uuid,
    ) -> Result<Vec<uuid::Uuid>, sqlx::Error> {
        let rows: Vec<(uuid::Uuid,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT user_id
            FROM unit_residents
            WHERE unit_id = $1
              AND end_date IS NULL
            "#,
        )
        .bind(unit_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    // ========================================================================
    // Issue #2303: Report Schedule Due-Work Consumer
    // ========================================================================

    /// Fire report schedules that are due and advance their `next_run_at`
    /// (issue #2303, finding 1 — the previously-missing due-work consumer).
    ///
    /// Before this, `report_schedules.next_run_at` was write-only: `create` and
    /// `update_schedule` set it but nothing ever selected schedules by it, so
    /// scheduled reports never fired. This mirrors the workflow
    /// (`list_due_schedules` + `update_schedule_after_run`) and automation
    /// (`get_due_rules` + `update_next_run`) loops:
    ///
    /// 1. select active schedules whose `next_run_at` has elapsed,
    /// 2. record a `pending` execution for each so the fire is observable in the
    ///    Story 81.2 execution-history endpoints, then
    /// 3. advance `last_run_at`/`next_run_at` using the schedule's canonical
    ///    cadence (`compute_next_run_for_schedule` — cron_expression when set,
    ///    else the legacy frequency/time columns; finding 2).
    ///
    /// Advancing is what stops a schedule whose `next_run_at` is in the past from
    /// re-firing on every 60s tick. A cadence that yields no future run parks the
    /// schedule (NULL `next_run_at`) rather than spinning.
    ///
    /// NOTE: actual report *generation* and email delivery are a downstream
    /// concern — a future generator worker consumes the `pending` executions
    /// recorded here and transitions them to completed/failed. This method owns
    /// only the select-fire-advance loop.
    ///
    /// RLS (issue #2318): both tables carry FORCE RLS, and the production
    /// api-server connects as the table owner (bound by FORCE, no BYPASSRLS —
    /// see migration 00179). Running on the bare pool therefore made this loop
    /// a silent no-op: no GUC set → `get_current_org_id()` NULL → the due-work
    /// SELECT returned 0 rows. The loop now checks out a dedicated connection,
    /// reads due work under the global-read context (SELECT-only policy leg,
    /// migration 00218), and performs each schedule's writes under that
    /// schedule's own tenant context. The context is cleared before the
    /// connection returns to the pool.
    async fn fire_due_report_schedules(&self) -> Result<(), common::errors::AppError> {
        // Dedicated connection: RLS context GUCs are session-local, so the
        // whole select-fire-advance loop must run on one connection.
        let mut conn = self
            .pool
            .acquire()
            .await
            .map_err(|e| common::errors::AppError::Database(e.to_string()))?;

        // Global-read context for the cross-org due-work SELECT.
        db::tenant_context::set_global_read_context(&mut *conn, true)
            .await
            .map_err(|e| common::errors::AppError::Database(e.to_string()))?;

        let due = self
            .report_schedule_repo
            .get_due_schedules(&mut *conn)
            .await;

        // Drop the global-read flag before any writes; the per-schedule writes
        // below run under each schedule's own tenant context.
        db::tenant_context::set_global_read_context(&mut *conn, false)
            .await
            .map_err(|e| common::errors::AppError::Database(e.to_string()))?;

        let due = due?;
        if due.is_empty() {
            db::tenant_context::clear_request_context(&mut *conn)
                .await
                .map_err(|e| common::errors::AppError::Database(e.to_string()))?;
            return Ok(());
        }

        tracing::info!(count = due.len(), "Firing due report schedules");

        let now = chrono::Utc::now();
        let mut fired = 0u64;

        for schedule in &due {
            // Each due row carries its organization_id: scope the connection to
            // that tenant so the org-scoped write policies pass.
            if let Err(e) =
                db::tenant_context::set_tenant_context(&mut *conn, schedule.organization_id).await
            {
                tracing::error!(
                    schedule_id = %schedule.id,
                    organization_id = %schedule.organization_id,
                    error = %e,
                    "Failed to set tenant context — leaving schedule due for retry"
                );
                continue;
            }

            // Record the fire in execution history (Story 81.2). If this fails,
            // skip the advance so the schedule stays due and retries next tick
            // instead of silently losing the run.
            if let Err(e) = self
                .report_schedule_repo
                .record_execution(&mut *conn, schedule.id)
                .await
            {
                tracing::error!(
                    schedule_id = %schedule.id,
                    error = %e,
                    "Failed to record report execution — leaving schedule due for retry"
                );
                continue;
            }

            // Recompute the next fire from the schedule's canonical cadence.
            let next = crate::routes::reports::compute_next_run_for_schedule(schedule, now);
            if next.is_none() {
                tracing::warn!(
                    schedule_id = %schedule.id,
                    "Could not compute next_run_at — parking schedule (next_run_at set NULL)"
                );
            }

            if let Err(e) = self
                .report_schedule_repo
                .advance_after_run(&mut *conn, schedule.id, next)
                .await
            {
                tracing::error!(
                    schedule_id = %schedule.id,
                    error = %e,
                    "Failed to advance report schedule after fire"
                );
                continue;
            }

            fired += 1;
            tracing::info!(
                schedule_id = %schedule.id,
                organization_id = %schedule.organization_id,
                next_run_at = ?next,
                "Fired report schedule and advanced next_run_at"
            );
        }

        // Defensive: reset every RLS-relevant GUC before the connection goes
        // back to the pool so tenant context cannot bleed onto the next user.
        db::tenant_context::clear_request_context(&mut *conn)
            .await
            .map_err(|e| common::errors::AppError::Database(e.to_string()))?;

        if fired > 0 {
            let mut metrics = self.metrics.lock().unwrap();
            metrics.report_schedules_fired += fired;
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
    use std::collections::{HashMap, HashSet};
    use uuid::Uuid;

    /// Pure model of the per-window dedup performed inside
    /// [`Scheduler::send_meter_reminders`] (#1777).
    ///
    /// The live fan-out iterates a building window's unit-linked meters in
    /// order and, for each meter's unit, its residents — sending one reminder
    /// per resident per window even if that resident is on multiple meters.
    /// This helper mirrors that algorithm: walk `meters` in order, look up the
    /// residents for each meter's `unit_id`, and collect every `user_id` exactly
    /// once, preserving first-seen order. It lets the dedup decision be tested
    /// deterministically without Postgres or a live `NotificationService`.
    fn dedup_meter_reminder_targets(
        meters: &[(Uuid, Uuid)],
        residents_by_unit: &HashMap<Uuid, Vec<Uuid>>,
    ) -> Vec<Uuid> {
        let mut notified = HashSet::new();
        let mut targets = Vec::new();
        for (_meter_id, unit_id) in meters {
            let Some(residents) = residents_by_unit.get(unit_id) else {
                continue;
            };
            for &user_id in residents {
                if notified.insert(user_id) {
                    targets.push(user_id);
                }
            }
        }
        targets
    }

    #[test]
    fn test_meter_reminder_dedup_same_unit_one_resident_one_reminder() {
        // #1777: a resident on two unit-linked meters in the same window must
        // receive exactly ONE reminder, not one per meter.
        let unit_x = Uuid::new_v4();
        let meter_a = Uuid::new_v4();
        let meter_b = Uuid::new_v4();
        let user_u = Uuid::new_v4();

        let meters = vec![(meter_a, unit_x), (meter_b, unit_x)];
        let mut residents_by_unit = HashMap::new();
        residents_by_unit.insert(unit_x, vec![user_u]);

        let targets = dedup_meter_reminder_targets(&meters, &residents_by_unit);
        assert_eq!(targets.len(), 1, "resident reminded once across two meters");
        assert_eq!(targets, vec![user_u]);
    }

    #[test]
    fn test_meter_reminder_dedup_distinct_units_not_collapsed() {
        // Cross-unit residents must NOT be collapsed: per-window scope is
        // per-resident, not global, so two units with distinct residents both
        // get reminded.
        let unit_x = Uuid::new_v4();
        let unit_y = Uuid::new_v4();
        let meter_a = Uuid::new_v4();
        let meter_b = Uuid::new_v4();
        let user_1 = Uuid::new_v4();
        let user_2 = Uuid::new_v4();

        let meters = vec![(meter_a, unit_x), (meter_b, unit_y)];
        let mut residents_by_unit = HashMap::new();
        residents_by_unit.insert(unit_x, vec![user_1]);
        residents_by_unit.insert(unit_y, vec![user_2]);

        let targets = dedup_meter_reminder_targets(&meters, &residents_by_unit);
        assert_eq!(targets.len(), 2);
        assert!(targets.contains(&user_1));
        assert!(targets.contains(&user_2));
    }

    #[test]
    fn test_meter_reminder_dedup_shared_resident_across_units() {
        // A resident occupying two units that each have a meter in the same
        // window is still reminded only once.
        let unit_x = Uuid::new_v4();
        let unit_y = Uuid::new_v4();
        let meter_a = Uuid::new_v4();
        let meter_b = Uuid::new_v4();
        let shared_user = Uuid::new_v4();

        let meters = vec![(meter_a, unit_x), (meter_b, unit_y)];
        let mut residents_by_unit = HashMap::new();
        residents_by_unit.insert(unit_x, vec![shared_user]);
        residents_by_unit.insert(unit_y, vec![shared_user]);

        let targets = dedup_meter_reminder_targets(&meters, &residents_by_unit);
        assert_eq!(
            targets.len(),
            1,
            "same resident across two units reminded once"
        );
        assert_eq!(targets, vec![shared_user]);
    }

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.interval_secs, 60);
        assert!(config.enabled);
        assert_eq!(config.vote_reminder_days_before, 1);
    }

    #[test]
    fn test_scheduler_config_pin_max_age_default() {
        // Issue #972.7: pinned announcements auto-unpin after 30 days by default.
        let config = SchedulerConfig::default();
        assert_eq!(config.pin_max_age_days, 30);
    }

    #[test]
    fn test_scheduler_config_payment_reminder_defaults() {
        let config = SchedulerConfig::default();
        assert_eq!(config.payment_reminder_days_before, 7);
        // Grace period defaults to 0: transition to overdue on the due date itself.
        assert_eq!(config.overdue_grace_period_days, 0);
    }

    #[test]
    fn test_scheduler_metrics_default() {
        let metrics = SchedulerMetrics::default();
        assert_eq!(metrics.announcements_published, 0);
        assert_eq!(metrics.announcements_unpinned, 0);
        assert_eq!(metrics.votes_closed, 0);
        assert_eq!(metrics.payment_reminders_sent, 0);
        assert_eq!(metrics.invoices_transitioned_to_overdue, 0);
        assert_eq!(metrics.errors, 0);
    }
}
