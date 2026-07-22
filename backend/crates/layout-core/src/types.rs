use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Free-form props bag.
///
/// Only prop *keys* are gated: tenant overrides are checked against
/// `rails.prop_whitelist[type]` (see [`crate::validate::validate_tenant_override`]).
/// Prop *values* are NOT yet validated — any type, range, or length passes
/// through unchecked at both publish and tenant-override time. Per-prop schema
/// enforcement (type/min/max/maxLen) is not implemented; see issue #2449.
pub type Props = BTreeMap<String, serde_json::Value>;

/// Semantic, versioned component type name, e.g. "price-box.v1" (spec §3.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SectionType(pub String);

impl From<&str> for SectionType {
    fn from(s: &str) -> Self {
        SectionType(s.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Web,
    Mobile,
}

/// Sparse patch applied on top of a section's base config (platform or tenant layer).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SectionPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: Props,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionConfig {
    #[serde(rename = "type")]
    pub section_type: SectionType,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: Props,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<Platform, SectionPatch>,
}