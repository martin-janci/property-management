// backend/servers/deploy-server/src/infra/traffic.rs
use crate::infra::Store;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader, SeekFrom};
use tokio::time::sleep;

#[derive(serde::Deserialize)]
struct CaddyLogLine {
    request: CaddyRequest,
}

#[derive(serde::Deserialize)]
struct CaddyRequest {
    host: String,
}

pub async fn tail_caddy_log(path: String, store: Arc<Store>) {
    // Track current file offset + inode (to detect log rotation).
    let mut offset: u64 = 0;
    let mut last_inode: Option<u64> = None;
    let mut first_open = true;

    loop {
        match tail_iteration(&path, &store, &mut offset, &mut last_inode, &mut first_open).await {
            Ok(_) => {
                sleep(Duration::from_millis(500)).await;
            }
            Err(e) => {
                tracing::warn!(error = %e, "caddy log tail iteration failed; retrying in 5s");
                sleep(Duration::from_secs(5)).await;
                // After error, do not skip-to-end — start tracking from current offset on next open.
            }
        }
    }
}

#[cfg(unix)]
async fn tail_iteration(
    path: &str,
    store: &Arc<Store>,
    offset: &mut u64,
    last_inode: &mut Option<u64>,
    first_open: &mut bool,
) -> std::io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    let metadata = tokio::fs::metadata(path).await?;
    let current_inode = metadata.ino();
    let current_size = metadata.len();

    // Detect rotation: inode changed (logrotate moved old file aside) or size shrunk (truncate).
    if *last_inode != Some(current_inode) || current_size < *offset {
        if last_inode.is_some() {
            tracing::info!(path = %path, "caddy log rotated; resuming from start of new file");
        }
        *offset = 0;
    }
    *last_inode = Some(current_inode);

    // First open: skip to end of existing file (don't re-process historical entries that were
    // already counted before the deploy server started).
    if *first_open {
        *offset = current_size;
        *first_open = false;
        return Ok(());
    }

    if current_size == *offset {
        return Ok(()); // no new lines
    }

    let mut f = File::open(path).await?;
    f.seek(SeekFrom::Start(*offset)).await?;
    let mut reader = BufReader::new(f);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break; // EOF
        }
        // If the line doesn't end with `\n`, the writer may still be flushing — bail and retry.
        if !line.ends_with('\n') {
            // Don't advance offset over a partial line.
            break;
        }
        *offset += n as u64;

        if let Ok(entry) = serde_json::from_str::<CaddyLogLine>(line.trim_end()) {
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
