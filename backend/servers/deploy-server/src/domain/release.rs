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
