use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
}
