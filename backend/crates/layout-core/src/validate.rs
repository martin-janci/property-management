use crate::types::*;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("section {section:?} appears more than once")]
    DuplicateSection { section: SectionType },
    #[error("section {section:?} is not in the {platform:?} registry")]
    UnknownType { section: SectionType, platform: Platform },
    #[error("required section {section:?} is hidden in the base config")]
    RequiredHidden { section: SectionType },
    #[error("required component {section:?} is missing from the config")]
    RequiredMissing { section: SectionType },
    #[error("section {section:?} uses mode {mode:?}, unsupported on {platform:?}")]
    UnsupportedMode { section: SectionType, mode: String, platform: Platform },
}

/// Publish gate (spec §6.3): empty result = publishable. Errors block publish.
pub fn validate_publish(
    config: &ScreenConfig,
    manifests: &[RegistryManifest],
) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    // duplicates
    let mut seen = std::collections::BTreeSet::new();
    for s in &config.sections {
        if !seen.insert(&s.section_type) {
            errs.push(ValidationError::DuplicateSection { section: s.section_type.clone() });
        }
    }
    for m in manifests {
        for s in &config.sections {
            let Some(def) = m.components.get(&s.section_type) else {
                errs.push(ValidationError::UnknownType {
                    section: s.section_type.clone(),
                    platform: m.platform,
                });
                continue;
            };
            // effective visibility/mode on this platform (base + platform patch)
            let patch = s.overrides.get(&m.platform);
            let visible = patch.and_then(|p| p.visible).unwrap_or(s.visible);
            let mode = patch
                .and_then(|p| p.mode.clone())
                .or_else(|| s.mode.clone());
            if def.required && !visible {
                let err = ValidationError::RequiredHidden { section: s.section_type.clone() };
                if !errs.contains(&err) {
                    errs.push(err);
                }
            }
            if let Some(mode) = mode {
                if !def.supported_modes.contains(&mode) {
                    errs.push(ValidationError::UnsupportedMode {
                        section: s.section_type.clone(),
                        mode,
                        platform: m.platform,
                    });
                }
            }
        }
        // every required component in the manifest must be present in the config
        for (t, def) in &m.components {
            if def.required && !config.sections.iter().any(|s| &s.section_type == t) {
                let err = ValidationError::RequiredMissing { section: t.clone() };
                if !errs.contains(&err) {
                    errs.push(err);
                }
            }
        }
    }
    errs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn manifest(platform: Platform, entries: &[(&str, bool, &[&str])]) -> RegistryManifest {
        RegistryManifest {
            platform,
            components: entries
                .iter()
                .map(|(t, req, modes)| {
                    (
                        SectionType::from(*t),
                        ComponentDef {
                            required: *req,
                            supported_modes: modes.iter().map(|m| m.to_string()).collect(),
                            default_mode: None,
                        },
                    )
                })
                .collect(),
        }
    }

    fn section(t: &str) -> SectionConfig {
        SectionConfig {
            section_type: SectionType::from(t),
            visible: true,
            mode: None,
            props: BTreeMap::new(),
            overrides: BTreeMap::new(),
        }
    }

    #[test]
    fn valid_config_passes() {
        let cfg = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![section("gallery.v1"), section("faq.v1")],
        };
        let manifests = vec![
            manifest(Platform::Web, &[("gallery.v1", true, &[]), ("faq.v1", false, &[])]),
            manifest(Platform::Mobile, &[("gallery.v1", true, &[]), ("faq.v1", false, &[])]),
        ];
        assert!(validate_publish(&cfg, &manifests).is_empty());
    }

    #[test]
    fn publish_gate_catches_all_error_classes() {
        let mut hidden_required = section("gallery.v1");
        hidden_required.visible = false;
        let mut bad_mode = section("faq.v1");
        bad_mode.mode = Some("carousel".into());
        let cfg = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![
                hidden_required,          // required hidden in base → error
                bad_mode,                 // mode unsupported on web → error
                section("faq.v1"),        // duplicate type → error
                section("only-web.v1"),   // missing from mobile manifest → error
            ],
        };
        let manifests = vec![
            manifest(
                Platform::Web,
                &[("gallery.v1", true, &[]), ("faq.v1", false, &["accordion"]),
                  ("only-web.v1", false, &[])],
            ),
            manifest(
                Platform::Mobile,
                &[("gallery.v1", true, &[]), ("faq.v1", false, &["accordion"])],
            ),
        ];
        let errs = validate_publish(&cfg, &manifests);
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::RequiredHidden { section } if section.0 == "gallery.v1")));
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::UnsupportedMode { section, .. } if section.0 == "faq.v1")));
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::DuplicateSection { section } if section.0 == "faq.v1")));
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::UnknownType { section, platform }
                if section.0 == "only-web.v1" && *platform == Platform::Mobile)));
    }

    #[test]
    fn missing_required_component_is_an_error() {
        // manifest declares a required component the config omits entirely
        let cfg = ScreenConfig { screen: "s".into(), version: 1, sections: vec![] };
        let manifests =
            vec![manifest(Platform::Web, &[("gallery.v1", true, &[])])];
        let errs = validate_publish(&cfg, &manifests);
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::RequiredMissing { section } if section.0 == "gallery.v1")));
    }
}
