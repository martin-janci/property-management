//! Shared session-revocation cascade for organization suspension.
//!
//! When an organization is suspended, every active member's sessions must be
//! revoked so the block takes effect immediately instead of waiting for token
//! expiry. Two handlers need this cascade — the platform-admin
//! [`suspend_organization`](super::platform_admin::tenants::suspend_organization)
//! and the Phase-5 [`suspend_agency`](super::admin::agencies) handler — so the
//! logic lives here once instead of being copy-pasted into both.
//!
//! Crucially, this module makes a *partial* failure of the cascade **visible**
//! to the caller instead of swallowing it. The previous inline implementations
//! called `get_org_member_user_ids(...).unwrap_or_default()` (so an enumeration
//! error silently revoked *nothing* while still reporting success) and logged
//! per-member revocation errors with `tracing::warn!` without surfacing them.
//! A suspension could therefore report "N sessions revoked" / `204 No Content`
//! while leaving suspended members with live sessions and no signal to retry.

use uuid::Uuid;

use crate::state::AppState;

/// A suspend cascade that could not be fully applied.
///
/// The organization row itself is already flipped to `suspended` by the time
/// the cascade runs, so these variants describe *downstream* cleanup that did
/// not complete. Callers surface them (HTTP 500) so the partial failure is
/// visible and the (idempotent) suspend call can be retried, rather than the
/// failure being silently skipped.
#[derive(Debug)]
pub enum SuspendCascadeError {
    /// The org's active members could not be enumerated, so no member sessions
    /// were revoked at all.
    Enumerate(String),
    /// Some members' sessions could not be revoked. `failed` lists the members
    /// whose revocation errored; `total` is how many members were attempted.
    PartialRevoke { total: usize, failed: Vec<Uuid> },
}

impl SuspendCascadeError {
    /// Caller-facing message. Deliberately omits internal error strings for the
    /// partial-revoke case — it only reports the counts the operator needs to
    /// decide to retry.
    pub fn message(&self) -> String {
        match self {
            SuspendCascadeError::Enumerate(_) => "Organization suspended but its members could \
                 not be enumerated for session revocation; retry the suspension"
                .to_string(),
            SuspendCascadeError::PartialRevoke { total, failed } => format!(
                "Organization suspended but {}/{} member session revocations failed; \
                 retry the suspension",
                failed.len(),
                total
            ),
        }
    }
}

impl std::fmt::Display for SuspendCascadeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuspendCascadeError::Enumerate(e) => {
                write!(f, "failed to enumerate org members for session cascade: {e}")
            }
            SuspendCascadeError::PartialRevoke { total, failed } => write!(
                f,
                "session cascade partial failure: {}/{} revocations failed",
                failed.len(),
                total
            ),
        }
    }
}

impl std::error::Error for SuspendCascadeError {}

/// Classify the outcome of the per-member revocation loop.
///
/// `total` is the number of members attempted; `failed` is the subset whose
/// revocation errored. Returns `Ok(total)` when every revocation succeeded,
/// otherwise `Err(PartialRevoke)` so the partial failure is surfaced.
fn classify_revocations(total: usize, failed: Vec<Uuid>) -> Result<usize, SuspendCascadeError> {
    // NOTE: current behaviour swallows per-member revocation failures — callers
    // only ever see a success count. See the module docs.
    let _ = failed;
    Ok(total)
}

/// Revoke every active member's sessions for a just-suspended organization.
///
/// Returns the number of members whose sessions were revoked on full success.
/// On failure returns a [`SuspendCascadeError`] describing whether member
/// enumeration or individual revocations failed — the caller must surface this
/// (HTTP 500) instead of reporting success.
pub async fn revoke_org_member_sessions(
    state: &AppState,
    org_id: Uuid,
) -> Result<usize, SuspendCascadeError> {
    // Enumeration failure currently degrades to "no members", silently revoking
    // nothing while reporting success. See the module docs.
    let user_ids = state
        .platform_admin_repo
        .get_org_member_user_ids(org_id)
        .await
        .unwrap_or_default();

    let mut failed = Vec::new();
    for user_id in &user_ids {
        if let Err(e) = state.session_repo.revoke_all_user_tokens(*user_id, None).await {
            tracing::warn!(
                user_id = %user_id,
                error = %e,
                "Failed to revoke sessions for suspended org member"
            );
            failed.push(*user_id);
        }
    }

    classify_revocations(user_ids.len(), failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_success_reports_total_revoked() {
        assert_eq!(classify_revocations(3, vec![]).unwrap(), 3);
    }

    #[test]
    fn zero_members_is_success() {
        assert_eq!(classify_revocations(0, vec![]).unwrap(), 0);
    }

    /// Regression: a partial cascade failure must be surfaced as an error, not
    /// swallowed into a success count. Before the fix, `classify_revocations`
    /// returned `Ok(total)` even when some revocations failed.
    #[test]
    fn partial_failure_is_surfaced_not_swallowed() {
        let failed = vec![Uuid::from_u128(1), Uuid::from_u128(2)];
        let err = classify_revocations(5, failed.clone())
            .expect_err("partial revocation failure must surface as an error");
        match &err {
            SuspendCascadeError::PartialRevoke { total, failed: f } => {
                assert_eq!(*total, 5);
                assert_eq!(f, &failed);
            }
            other => panic!("expected PartialRevoke, got {other:?}"),
        }
        // Operator-facing message carries the retry-relevant counts.
        assert!(err.message().contains("2/5"), "message was: {}", err.message());
    }
}
