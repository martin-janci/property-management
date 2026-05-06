use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BackendMode {
    Shared,
    Dedicated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorktreeState {
    Running,
    Paused,
    Closing,
    Closed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeUrls {
    pub ppt: Option<String>,
    pub reality: Option<String>,
    pub api: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    pub name: String,
    pub branch: String,
    pub backend_mode: BackendMode,
    pub state: WorktreeState,
    pub urls: WorktreeUrls,
    pub containers: Vec<String>,
    pub db_name: Option<String>,
    pub dump_path: Option<String>,
    pub ttl_seconds: i64,
    pub last_traffic_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub created_by: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_round_trip_json() {
        let wt = Worktree {
            name: "feature-uc14".into(),
            branch: "feature/UC-14".into(),
            backend_mode: BackendMode::Shared,
            state: WorktreeState::Running,
            urls: WorktreeUrls {
                ppt: Some("https://wt-feature-uc14.dev.ppt.rlt.sk".into()),
                reality: Some("https://wt-feature-uc14.dev.rlt.sk".into()),
                api: None,
            },
            containers: vec![
                "wt-feature-uc14-ppt".into(),
                "wt-feature-uc14-reality".into(),
            ],
            db_name: None,
            dump_path: None,
            ttl_seconds: 172_800,
            last_traffic_at: None,
            closed_at: None,
            created_at: Utc::now(),
            created_by: "oidc:martin-janci/property-management@feature/UC-14".into(),
        };

        let json = serde_json::to_string(&wt).unwrap();
        let parsed: Worktree = serde_json::from_str(&json).unwrap();
        assert_eq!(wt.name, parsed.name);
        assert_eq!(wt.backend_mode, parsed.backend_mode);
        assert_eq!(wt.state, parsed.state);
    }

    #[test]
    fn backend_mode_serializes_lowercase() {
        let json = serde_json::to_string(&BackendMode::Dedicated).unwrap();
        assert_eq!(json, "\"dedicated\"");
    }
}
