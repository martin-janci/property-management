use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseState {
    Candidate,
    Staging,
    Prod,
    Previous,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Staging,
    Prod,
}

impl TargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TargetKind::Staging => "staging",
            TargetKind::Prod => "prod",
        }
    }

    /// String form of the live ReleaseState for this target.
    /// Released to this target = its own name (per existing schema invariant).
    pub fn live_release_state_str(self) -> &'static str {
        self.as_str()
    }

    pub fn live_release_state(self) -> ReleaseState {
        match self {
            TargetKind::Staging => ReleaseState::Staging,
            TargetKind::Prod => ReleaseState::Prod,
        }
    }
}

impl std::fmt::Display for TargetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for TargetKind {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s {
            "staging" => Ok(TargetKind::Staging),
            "prod" => Ok(TargetKind::Prod),
            other => Err(format!("unknown target: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub tag: String,
    /// Map of service name (e.g. "api-server") -> image ref.
    pub images: HashMap<String, String>,
    pub state: ReleaseState,
    pub target: Option<String>,
    pub promoted_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
}
