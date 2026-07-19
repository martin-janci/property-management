use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Free-form props bag; values validated against component prop schemas at publish time.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScreenConfig {
    pub screen: String,
    pub version: u32,
    pub sections: Vec<SectionConfig>,
}

fn default_true() -> bool {
    true
}

/// Per-component contract published by each frontend (spec §3.3).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ComponentDef {
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryManifest {
    pub platform: Platform,
    pub components: BTreeMap<SectionType, ComponentDef>,
}

/// Sparse per-org delta over a published base config (spec §3.2).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TenantOverride {
    /// Full desired order by type. None = keep base order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub order: Option<Vec<SectionType>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sections: BTreeMap<SectionType, SectionPatch>,
}

/// What tenant admins may change on a screen (spec §3.4). Authored by superadmin.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Rails {
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub hideable: BTreeSet<SectionType>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub mode_editable: BTreeSet<SectionType>,
    #[serde(default)]
    pub reorderable: bool,
    /// Per-section set of prop names tenant admins may set.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub prop_whitelist: BTreeMap<SectionType, BTreeSet<String>>,
}

/// How a resolved section renders (spec §4 rules 2–3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Presentation {
    Visible,
    /// Required section that is hidden, killed, or failed validation.
    Placeholder,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedSection {
    #[serde(rename = "type")]
    pub section_type: SectionType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub props: Props,
    pub presentation: Presentation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedScreen {
    pub screen: String,
    pub version: u32,
    pub sections: Vec<ResolvedSection>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The illustrative config from spec §3.1 must deserialize and round-trip.
    #[test]
    fn screen_config_round_trips_spec_example() {
        let json = r#"{
          "screen": "reality/listing-detail",
          "version": 42,
          "sections": [
            { "type": "gallery.v1" },
            { "type": "agent-contact.v1", "mode": "sticky-sidebar",
              "overrides": { "mobile": { "mode": "bottom-bar" } } },
            { "type": "similar-listings.v1", "visible": false },
            { "type": "mortgage-calc.v1", "props": { "maxYears": 30 } }
          ]
        }"#;
        let cfg: ScreenConfig = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.screen, "reality/listing-detail");
        assert_eq!(cfg.version, 42);
        assert_eq!(cfg.sections.len(), 4);
        // "visible" defaults to true when omitted
        assert!(cfg.sections[0].visible);
        assert!(!cfg.sections[2].visible);
        // platform override parsed under enum key
        let patch = &cfg.sections[1].overrides[&Platform::Mobile];
        assert_eq!(patch.mode.as_deref(), Some("bottom-bar"));
        // props carried as JSON values
        assert_eq!(cfg.sections[3].props["maxYears"], serde_json::json!(30));
        // unknown fields are ignored (additive evolution)
        let with_unknown = r#"{"screen":"s","version":1,"sections":[
            {"type":"x.v1","futureField":true}]}"#;
        assert!(serde_json::from_str::<ScreenConfig>(with_unknown).is_ok());
        // round-trip
        let back: ScreenConfig =
            serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
        assert_eq!(back.sections[1].overrides[&Platform::Mobile].mode.as_deref(),
                   Some("bottom-bar"));
    }

    #[test]
    fn manifest_override_and_rails_deserialize_with_defaults() {
        let manifest: RegistryManifest = serde_json::from_str(
            r#"{"platform":"web","components":{
                "gallery.v1":{"required":true},
                "listing-grid.v1":{"supported_modes":["list","grid","map"],
                                    "default_mode":"list"}}}"#,
        )
        .unwrap();
        let gallery = &manifest.components[&SectionType::from("gallery.v1")];
        assert!(gallery.required);
        assert!(gallery.supported_modes.is_empty());
        let grid = &manifest.components[&SectionType::from("listing-grid.v1")];
        assert!(!grid.required); // required defaults to false
        assert_eq!(grid.default_mode.as_deref(), Some("list"));

        let ov: TenantOverride = serde_json::from_str(
            r#"{"order":["b.v1","a.v1"],
                "sections":{"a.v1":{"visible":false}}}"#,
        )
        .unwrap();
        assert_eq!(ov.order.as_ref().unwrap().len(), 2);
        assert_eq!(ov.sections[&SectionType::from("a.v1")].visible, Some(false));
        // empty override is valid (all fields default)
        assert!(serde_json::from_str::<TenantOverride>("{}").is_ok());

        let rails: Rails = serde_json::from_str(
            r#"{"hideable":["a.v1"],"reorderable":true,
                "prop_whitelist":{"a.v1":["title","limit"]}}"#,
        )
        .unwrap();
        assert!(rails.reorderable);
        assert!(rails.hideable.contains(&SectionType::from("a.v1")));
        assert!(rails.mode_editable.is_empty()); // defaults empty
    }
}
