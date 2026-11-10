//! Account and lease lifecycle jobs for the background [`Scheduler`].
//!
//! - GDPR Article 17 erasure once `users.scheduled_deletion_at` has passed
//!   (gap audit R6 P1-5). `request_data_deletion` promises erasure after a
//!   30-day grace period; before this job nothing ever read the column.
//! - Lease expiry: `active` leases whose `end_date` is in the past become
//!   `expired` (gap audit R6 P2-13).
//!
//! Both queries hit partial indexes (`idx_users_scheduled_deletion`,
//! `idx_leases_expiring`), so running them on every ~60s tick is cheap.

use super::Scheduler;

/// Upper bound of accounts erased per tick, so a backlog cannot hold the tick
/// loop for long. The remainder is picked up on the next tick.
const ACCOUNT_DELETIONS_PER_TICK: i64 = 50;

impl Scheduler {
    /// Anonymize accounts whose GDPR deletion grace period has elapsed.
    pub(super) async fn execute_scheduled_account_deletions(&self) -> Result<(), sqlx::Error> {
        // Scheduler runs without a user context; `users` is not RLS-scoped.
        let erased = self
            .user_repo
            .anonymize_due_deletions(ACCOUNT_DELETIONS_PER_TICK)
            .await?;
        if erased.is_empty() {
            return Ok(());
        }

        for user_id in &erased {
            tracing::info!(user_id = %user_id, "GDPR erasure executed for account");
        }
        let mut metrics = self.metrics.lock().unwrap_or_else(|e| e.into_inner());
        metrics.accounts_anonymized += erased.len() as u64;
        Ok(())
    }

    /// Flip `active` leases past their end date to `expired`.
    pub(super) async fn expire_ended_leases(&self) -> Result<(), sqlx::Error> {
        let expired = self.lease_repo.expire_ended_leases().await?;
        if expired.is_empty() {
            return Ok(());
        }

        tracing::info!(count = expired.len(), "Expired leases past their end date");
        let mut metrics = self.metrics.lock().unwrap_or_else(|e| e.into_inner());
        metrics.leases_expired += expired.len() as u64;
        Ok(())
    }
}
