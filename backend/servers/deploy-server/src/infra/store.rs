// backend/servers/deploy-server/src/infra/store.rs
use crate::domain::{BackendMode, Worktree, WorktreeState, WorktreeUrls};
use crate::Result;
use chrono::{TimeZone, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;

#[derive(Clone)]
pub struct Store {
    pool: Pool<Sqlite>,
}

impl Store {
    pub async fn open(db_path: &Path) -> Result<Self> {
        let opts =
            SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", db_path.display()))
                .map_err(|e| crate::DeployError::Config(format!("sqlite opts: {e}")))?
                .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| crate::DeployError::Internal(format!("migrate: {e}")))?;

        Ok(Self { pool })
    }

    #[cfg(test)]
    pub(crate) fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn upsert_worktree(&self, wt: &Worktree) -> Result<()> {
        let urls = serde_json::to_string(&wt.urls).unwrap();
        let containers = serde_json::to_string(&wt.containers).unwrap();
        let backend = match wt.backend_mode {
            BackendMode::Shared => "shared",
            BackendMode::Dedicated => "dedicated",
        };
        let state = match wt.state {
            WorktreeState::Running => "running",
            WorktreeState::Paused => "paused",
            WorktreeState::Closing => "closing",
            WorktreeState::Closed => "closed",
        };

        sqlx::query(
            r#"INSERT INTO worktree
                (name, branch, backend_mode, state, urls, containers, db_name, dump_path,
                 ttl_seconds, last_traffic_at, closed_at, created_at, created_by)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(name) DO UPDATE SET
                  branch=excluded.branch,
                  backend_mode=excluded.backend_mode,
                  state=excluded.state,
                  urls=excluded.urls,
                  containers=excluded.containers,
                  db_name=excluded.db_name,
                  dump_path=excluded.dump_path,
                  ttl_seconds=excluded.ttl_seconds,
                  last_traffic_at=excluded.last_traffic_at,
                  closed_at=excluded.closed_at"#,
        )
        .bind(&wt.name)
        .bind(&wt.branch)
        .bind(backend)
        .bind(state)
        .bind(urls)
        .bind(containers)
        .bind(wt.db_name.as_deref())
        .bind(wt.dump_path.as_deref())
        .bind(wt.ttl_seconds)
        .bind(wt.last_traffic_at.map(|t| t.timestamp()))
        .bind(wt.closed_at.map(|t| t.timestamp()))
        .bind(wt.created_at.timestamp())
        .bind(&wt.created_by)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_worktree(&self, name: &str) -> Result<Option<Worktree>> {
        let row = sqlx::query_as::<_, WorktreeRow>(
            r#"SELECT name, branch, backend_mode, state, urls, containers, db_name, dump_path,
                       ttl_seconds, last_traffic_at, closed_at, created_at, created_by
                FROM worktree WHERE name = ?"#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await?;
        row.map(WorktreeRow::into_domain).transpose()
    }

    pub async fn list_worktrees(&self) -> Result<Vec<Worktree>> {
        let rows = sqlx::query_as::<_, WorktreeRow>(
            r#"SELECT name, branch, backend_mode, state, urls, containers, db_name, dump_path,
                       ttl_seconds, last_traffic_at, closed_at, created_at, created_by
                FROM worktree ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(WorktreeRow::into_domain).collect()
    }

    pub async fn update_last_traffic(&self, name: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp();
        sqlx::query("UPDATE worktree SET last_traffic_at = ? WHERE name = ?")
            .bind(now)
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn record_audit(
        &self,
        caller_kind: &str,
        caller_id: &str,
        endpoint: &str,
        params: Option<&str>,
        result: &str,
        duration_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO audit (ts, caller_kind, caller_id, endpoint, params, result, duration_ms)
               VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(Utc::now().timestamp())
        .bind(caller_kind)
        .bind(caller_id)
        .bind(endpoint)
        .bind(params)
        .bind(result)
        .bind(duration_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_audit(&self, limit: i64) -> Result<Vec<AuditEntry>> {
        let rows = sqlx::query_as::<_, AuditRow>(
            r#"SELECT id, ts, caller_kind, caller_id, endpoint, params, result, duration_ms
                FROM audit ORDER BY ts DESC LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(AuditRow::into_domain).collect())
    }

    pub async fn upsert_release(&self, rel: &crate::domain::Release) -> Result<()> {
        let images = serde_json::to_string(&rel.images).unwrap();
        let state = match rel.state {
            crate::domain::ReleaseState::Candidate => "candidate",
            crate::domain::ReleaseState::Staging => "staging",
            crate::domain::ReleaseState::Prod => "prod",
            crate::domain::ReleaseState::Previous => "previous",
        };
        sqlx::query(
            r#"INSERT INTO release (tag, images, state, target, promoted_at, notes)
               VALUES (?, ?, ?, ?, ?, ?)
               ON CONFLICT(tag) DO UPDATE SET
                 images=excluded.images, state=excluded.state, target=excluded.target,
                 promoted_at=excluded.promoted_at, notes=excluded.notes"#,
        )
        .bind(&rel.tag)
        .bind(images)
        .bind(state)
        .bind(rel.target.as_deref())
        .bind(rel.promoted_at.map(|t| t.timestamp()))
        .bind(rel.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_release(&self, tag: &str) -> Result<Option<crate::domain::Release>> {
        let row = sqlx::query_as::<_, ReleaseRow>(
            r#"SELECT tag, images, state, target, promoted_at, notes FROM release WHERE tag = ?"#,
        )
        .bind(tag)
        .fetch_optional(&self.pool)
        .await?;
        row.map(ReleaseRow::into_domain).transpose()
    }

    pub async fn current_release_for(
        &self,
        target: &str,
        state: &str,
    ) -> Result<Option<crate::domain::Release>> {
        let row = sqlx::query_as::<_, ReleaseRow>(
            r#"SELECT tag, images, state, target, promoted_at, notes FROM release
               WHERE target = ? AND state = ? ORDER BY promoted_at DESC LIMIT 1"#,
        )
        .bind(target)
        .bind(state)
        .fetch_optional(&self.pool)
        .await?;
        row.map(ReleaseRow::into_domain).transpose()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: i64,
    pub caller_kind: String,
    pub caller_id: String,
    pub endpoint: String,
    pub params: Option<String>,
    pub result: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct AuditRow {
    id: i64,
    ts: i64,
    caller_kind: String,
    caller_id: String,
    endpoint: String,
    params: Option<String>,
    result: Option<String>,
    duration_ms: Option<i64>,
}

impl AuditRow {
    fn into_domain(self) -> AuditEntry {
        AuditEntry {
            id: self.id,
            ts: self.ts,
            caller_kind: self.caller_kind,
            caller_id: self.caller_id,
            endpoint: self.endpoint,
            params: self.params,
            result: self.result,
            duration_ms: self.duration_ms,
        }
    }
}

#[derive(sqlx::FromRow)]
struct WorktreeRow {
    name: String,
    branch: String,
    backend_mode: String,
    state: String,
    urls: String,
    containers: String,
    db_name: Option<String>,
    dump_path: Option<String>,
    ttl_seconds: i64,
    last_traffic_at: Option<i64>,
    closed_at: Option<i64>,
    created_at: i64,
    created_by: String,
}

impl WorktreeRow {
    fn into_domain(self) -> Result<Worktree> {
        let backend_mode = match self.backend_mode.as_str() {
            "shared" => BackendMode::Shared,
            "dedicated" => BackendMode::Dedicated,
            other => {
                return Err(crate::DeployError::Internal(format!(
                    "bad backend_mode {other}"
                )))
            }
        };
        let state = match self.state.as_str() {
            "running" => WorktreeState::Running,
            "paused" => WorktreeState::Paused,
            "closing" => WorktreeState::Closing,
            "closed" => WorktreeState::Closed,
            other => return Err(crate::DeployError::Internal(format!("bad state {other}"))),
        };
        let urls: WorktreeUrls = serde_json::from_str(&self.urls)
            .map_err(|e| crate::DeployError::Internal(format!("bad urls json: {e}")))?;
        let containers: Vec<String> = serde_json::from_str(&self.containers)
            .map_err(|e| crate::DeployError::Internal(format!("bad containers json: {e}")))?;
        Ok(Worktree {
            name: self.name,
            branch: self.branch,
            backend_mode,
            state,
            urls,
            containers,
            db_name: self.db_name,
            dump_path: self.dump_path,
            ttl_seconds: self.ttl_seconds,
            last_traffic_at: self
                .last_traffic_at
                .map(|t| Utc.timestamp_opt(t, 0).unwrap()),
            closed_at: self.closed_at.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
            created_at: Utc.timestamp_opt(self.created_at, 0).unwrap(),
            created_by: self.created_by,
        })
    }
}

#[derive(sqlx::FromRow)]
struct ReleaseRow {
    tag: String,
    images: String,
    state: String,
    target: Option<String>,
    promoted_at: Option<i64>,
    notes: Option<String>,
}

impl ReleaseRow {
    fn into_domain(self) -> Result<crate::domain::Release> {
        use crate::domain::ReleaseState;
        let state = match self.state.as_str() {
            "candidate" => ReleaseState::Candidate,
            "staging" => ReleaseState::Staging,
            "prod" => ReleaseState::Prod,
            "previous" => ReleaseState::Previous,
            other => {
                return Err(crate::DeployError::Internal(format!(
                    "bad release state {other}"
                )))
            }
        };
        let images: std::collections::HashMap<String, String> = serde_json::from_str(&self.images)
            .map_err(|e| crate::DeployError::Internal(format!("bad images json: {e}")))?;
        Ok(crate::domain::Release {
            tag: self.tag,
            images,
            state,
            target: self.target,
            promoted_at: self.promoted_at.map(|t| Utc.timestamp_opt(t, 0).unwrap()),
            notes: self.notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn upsert_and_get_round_trip() {
        let dir = tempdir().unwrap();
        let store = Store::open(&dir.path().join("state.db")).await.unwrap();
        let wt = Worktree {
            name: "foo".into(),
            branch: "feature/foo".into(),
            backend_mode: BackendMode::Shared,
            state: WorktreeState::Running,
            urls: WorktreeUrls {
                ppt: Some("https://x".into()),
                reality: None,
                api: None,
            },
            containers: vec!["c1".into()],
            db_name: None,
            dump_path: None,
            ttl_seconds: 7200,
            last_traffic_at: None,
            closed_at: None,
            created_at: Utc::now(),
            created_by: "test".into(),
        };
        store.upsert_worktree(&wt).await.unwrap();
        let got = store.get_worktree("foo").await.unwrap().unwrap();
        assert_eq!(got.branch, "feature/foo");
        assert_eq!(got.containers, vec!["c1".to_string()]);

        let list = store.list_worktrees().await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn release_upsert_and_get() {
        use crate::domain::{Release, ReleaseState};
        use std::collections::HashMap;
        let dir = tempdir().unwrap();
        let store = Store::open(&dir.path().join("state.db")).await.unwrap();
        let mut images = HashMap::new();
        images.insert("api-server".into(), "ghcr.io/x/api:v1".into());
        let rel = Release {
            tag: "v1.0.0".into(),
            images,
            state: ReleaseState::Candidate,
            target: Some("staging".into()),
            promoted_at: None,
            notes: None,
        };
        store.upsert_release(&rel).await.unwrap();
        let got = store.get_release("v1.0.0").await.unwrap().unwrap();
        assert_eq!(got.tag, "v1.0.0");
        assert!(matches!(got.state, ReleaseState::Candidate));
    }

    #[tokio::test]
    async fn audit_insert() {
        let dir = tempdir().unwrap();
        let store = Store::open(&dir.path().join("state.db")).await.unwrap();
        store
            .record_audit(
                "api_key",
                "claude-skill",
                "POST /api/worktree",
                Some("{}"),
                "ok",
                42,
            )
            .await
            .unwrap();
        let row: (i64,) = sqlx::query_as("SELECT count(*) FROM audit")
            .fetch_one(store.pool())
            .await
            .unwrap();
        assert_eq!(row.0, 1);
    }
}
