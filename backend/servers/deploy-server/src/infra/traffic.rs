// backend/servers/deploy-server/src/infra/traffic.rs
use crate::infra::Store;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(serde::Deserialize)]
struct CaddyLogLine {
    request: CaddyRequest,
}

#[derive(serde::Deserialize)]
struct CaddyRequest {
    host: String,
}

pub async fn tail_caddy_log(path: String, store: Arc<Store>) {
    loop {
        if let Err(e) = tail_once(&path, &store).await {
            tracing::warn!(error = %e, "caddy log tail failed; retrying in 5s");
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

async fn tail_once(path: &str, store: &Arc<Store>) -> std::io::Result<()> {
    let f = File::open(path).await?;
    let mut reader = BufReader::new(f).lines();
    while let Some(line) = reader.next_line().await? {
        if let Ok(entry) = serde_json::from_str::<CaddyLogLine>(&line) {
            if let Some(name) = parse_worktree_from_host(&entry.request.host) {
                let _ = store.update_last_traffic(&name).await;
            }
        }
    }
    Ok(())
}

pub fn parse_worktree_from_host(host: &str) -> Option<String> {
    host.strip_prefix("wt-")
        .and_then(|s| s.split('.').next())
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_host() {
        assert_eq!(
            parse_worktree_from_host("wt-feature-x.dev.ppt.rlt.sk"),
            Some("feature-x".into())
        );
        assert_eq!(parse_worktree_from_host("staging.rlt.sk"), None);
        assert_eq!(
            parse_worktree_from_host("wt-uc14.dev.rlt.sk"),
            Some("uc14".into())
        );
    }
}
