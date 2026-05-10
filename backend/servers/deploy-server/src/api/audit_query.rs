// backend/servers/deploy-server/src/api/audit_query.rs
use crate::api::worktree::WorktreeService;
use crate::infra::{AuditEntry, CallerIdentity};
use crate::Result;
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    100
}

pub async fn list_handler(
    State(svc): State<Arc<WorktreeService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<AuditEntry>>> {
    caller.require_scope("audit:read")?;
    let entries = svc.store.list_audit(q.limit.clamp(1, 500)).await?;
    Ok(Json(entries))
}
