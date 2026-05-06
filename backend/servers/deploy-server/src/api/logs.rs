// backend/servers/deploy-server/src/api/logs.rs
use crate::api::worktree::WorktreeService;
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
    Path(name): Path<String>,
    Query(q): Query<LogsQuery>,
) -> Result<Sse<impl Stream<Item = std::result::Result<Event, Infallible>>>> {
    let wt = svc
        .store
        .get_worktree(&name)
        .await?
        .ok_or_else(|| DeployError::NotFound(format!("worktree {name}")))?;

    // Pick container by service hint or aggregate.
    let svc_filter = q.service.as_deref().unwrap_or("all");
    let containers: Vec<String> = wt
        .containers
        .iter()
        .filter(|c| {
            if svc_filter == "all" {
                return true;
            }
            c.ends_with(&format!("-{svc_filter}"))
        })
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
