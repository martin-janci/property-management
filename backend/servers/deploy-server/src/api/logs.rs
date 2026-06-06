// backend/servers/deploy-server/src/api/logs.rs
use crate::api::worktree::WorktreeService;
use crate::infra::CallerIdentity;
use crate::{DeployError, Result};
use axum::extract::{Path, Query, State};
use axum::response::sse::{Event, Sse};
use bollard::container::LogsOptions;
use futures_util::stream::Stream;
use futures_util::StreamExt;
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(default)]
    pub follow: bool,
    /// "ppt", "reality", "api", "reality-api", or "all" (default).
    pub service: Option<String>,
}

pub async fn handler(
    State(svc): State<Arc<WorktreeService>>,
    axum::Extension(caller): axum::Extension<CallerIdentity>,
    Path(name): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    caller.require_scope("worktree:read")?;

    // Validate the path param at the boundary (#769 finding 6) — the name is
    // interpolated into the expected container name (`wt-{name}-{service}`) below.
    crate::infra::git::validate_alias_strict(&name)?;

    let wt = svc
        .store
        .get_worktree(&name)
        .await?
        .ok_or_else(|| DeployError::NotFound(format!("worktree {name}")))?;

    // Pick container by exact service hint or aggregate. Container names follow
    // `wt-{worktree_name}-{service}` so an exact match prevents `api` from matching
    // both `wt-x-api` and `wt-x-reality-api`.
    let svc_filter = q.service.as_deref().unwrap_or("all");
    let expected = format!("wt-{name}-{svc_filter}");
    let containers: Vec<String> = wt
        .containers
        .iter()
        .filter(|c| svc_filter == "all" || c.as_str() == expected.as_str())
        .cloned()
        .collect();

    if containers.is_empty() {
        return Err(DeployError::NotFound(format!(
            "no containers match service={svc_filter} for worktree {name}"
        )));
    }

    // For MVP, only stream the first matching container. Multi-container aggregation
    // requires merging streams which is more code; defer to Phase 6.
    let container = containers.first().cloned().unwrap();
    let docker = svc.docker.bollard().clone();

    let opts = LogsOptions::<String> {
        follow: q.follow,
        stdout: true,
        stderr: true,
        tail: "100".to_string(),
        timestamps: false,
        ..Default::default()
    };

    let stream = docker.logs(&container, Some(opts)).map(|item| {
        let line = match item {
            Ok(log) => log.to_string(),
            Err(e) => format!("[stream error: {e}]\n"),
        };
        Ok::<_, Infallible>(Event::default().data(line.trim_end()))
    });

    Ok(Sse::new(stream))
}
