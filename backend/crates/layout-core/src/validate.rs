use crate::types::*;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum ValidationError {
    #[error("section {section:?} appears more than once")]
    DuplicateSection { section: SectionType },
    #[error("section {section:?} is not in the {platform:?} registry")]
    UnknownType {
        section: SectionType,
        platform: Platform,
    },
    #[error("required section {section:?} is hidden in the base config")]
    RequiredHidden { section: SectionType },
    #[error("required component {section:?} is missing from the config")]
    RequiredMissing { section: SectionType },
    #[error("section {section:?} uses mode {mode:?}, unsupported on {platform:?}")]
    UnsupportedMode {
        section: SectionType,
        mode: String,
        platform: Platform,
    },
    #[error("tenant override references section {section:?} not present in the base config")]
    NotInBase { section: SectionType },
    #[error("section {section:?} is not tenant-hideable")]
    NotHideable { section: SectionType },
    #[error("section {section:?} is not tenant-mode-editable")]
    NotModeEditable { section: SectionType },
    #[error("prop {prop:?} on section {section:?} is not whitelisted for tenants")]
    PropNotWhitelisted { section: SectionType, prop: String },
    #[error("this screen is not tenant-reorderable")]
    NotReorderable,
    #[error("tenant override order lists section {section:?} more than once")]
    DuplicateOrderEntry { section: SectionType },
    #[error("component {section:?} declares default_mode {mode:?} not in its supported_modes on {platform:?}")]
    InvalidDefaultMode {
        section: SectionType,
        mode: String,
        platform: Platform,
    },
}

/// Publish gate (spec §6.3): empty result = publishable. Errors block publish.
///
/// Error ordering follows section iteration order (the order sections appear in the config).
pub fn validate_publish(
    config: &ScreenConfig,
    manifests: &[RegistryManifest],
) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    // duplicates
    let mut seen = std::collections::BTreeSet::new();
    for s in &config.sections {
        if !seen.insert(&s.section_type) {
            errs.push(ValidationError::DuplicateSection {
                section: s.section_type.clone(),
            });
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
                let err = ValidationError::RequiredHidden {
                    section: s.section_type.clone(),
                };
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
            // gate inconsistent default_mode: if declared, it must be in supported_modes
            if let Some(dm) = &def.default_mode {
                if !def.supported_modes.contains(dm) {
                    errs.push(ValidationError::InvalidDefaultMode {
                        section: t.clone(),
                        mode: dm.clone(),
                        platform: m.platform,
                    });
                }
            }
        }
    }
    errs
}

/// Server-side rails enforcement for tenant saves (spec §3.4). The UI hides
/// out-of-rails controls, but this is the actual gate.
///
/// Error ordering follows alphabetical BTreeMap order (the order sections appear in the
/// tenant override's BTreeMap).
pub fn validate_tenant_override(
    ov: &TenantOverride,
    base: &ScreenConfig,
    rails: &Rails,
) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    if ov.order.is_some() && !rails.reorderable {
        errs.push(ValidationError::NotReorderable);
    }
    // Order contents gate: every entry must exist in the base config and appear
    // at most once. Without this, resolve's order pass would emit one section
    // per entry, so `order: ["a.v1" × N]` could duplicate sections N times.
    if let Some(order) = &ov.order {
        let mut seen = std::collections::BTreeSet::new();
        for t in order {
            if !seen.insert(t) {
                errs.push(ValidationError::DuplicateOrderEntry { section: t.clone() });
                continue;
            }
            if !base.sections.iter().any(|s| &s.section_type == t) {
                errs.push(ValidationError::NotInBase { section: t.clone() });
            }
        }
    }
    static EMPTY: std::sync::OnceLock<std::collections::BTreeSet<String>> =
        std::sync::OnceLock::new();
    for (t, patch) in &ov.sections {
        if !base.sections.iter().any(|s| &s.section_type == t) {
            errs.push(ValidationError::NotInBase { section: t.clone() });
            continue;
        }
        if patch.visible.is_some() && !rails.hideable.contains(t) {
            errs.push(ValidationError::NotHideable { section: t.clone() });
        }
        if patch.mode.is_some() && !rails.mode_editable.contains(t) {
            errs.push(ValidationError::NotModeEditable { section: t.clone() });
        }
        let whitelist = rails
            .prop_whitelist
            .get(t)
            .unwrap_or_else(|| EMPTY.get_or_init(Default::default));
        for prop in patch.props.keys() {
            if !whitelist.contains(prop) {
                errs.push(ValidationError::PropNotWhitelisted {
                    section: t.clone(),
                    prop: prop.clone(),
                });
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
            manifest(
                Platform::Web,
                &[("gallery.v1", true, &[]), ("faq.v1", false, &[])],
            ),
            manifest(
                Platform::Mobile,
                &[("gallery.v1", true, &[]), ("faq.v1", false, &[])],
            ),
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
                hidden_required,        // required hidden in base → error
                bad_mode,               // mode unsupported on web → error
                section("faq.v1"),      // duplicate type → error
                section("only-web.v1"), // missing from mobile manifest → error
            ],
        };
        let manifests = vec![
            manifest(
                Platform::Web,
                &[
                    ("gallery.v1", true, &[]),
                    ("faq.v1", false, &["accordion"]),
                    ("only-web.v1", false, &[]),
                ],
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
        let cfg = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![],
        };
        let manifests = vec![manifest(Platform::Web, &[("gallery.v1", true, &[])])];
        let errs = validate_publish(&cfg, &manifests);
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::RequiredMissing { section } if section.0 == "gallery.v1")));
    }

    #[test]
    fn platform_override_changes_effective_visibility() {
        // required section hidden in base but re-shown on web via override:
        // no error on web; still an error on mobile (no override there).
        let mut s = section("gallery.v1");
        s.visible = false;
        s.overrides.insert(
            Platform::Web,
            SectionPatch {
                visible: Some(true),
                ..Default::default()
            },
        );
        let cfg = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![s],
        };

        let web_only = vec![manifest(Platform::Web, &[("gallery.v1", true, &[])])];
        assert!(validate_publish(&cfg, &web_only).is_empty());

        let both = vec![
            manifest(Platform::Web, &[("gallery.v1", true, &[])]),
            manifest(Platform::Mobile, &[("gallery.v1", true, &[])]),
        ];
        let errs = validate_publish(&cfg, &both);
        assert_eq!(
            errs,
            vec![ValidationError::RequiredHidden {
                section: SectionType::from("gallery.v1")
            }]
        );
    }

    #[test]
    fn rails_enforcement_rejects_out_of_rails_edits() {
        use std::collections::BTreeSet;
        let base = ScreenConfig {
            screen: "ppt/dashboard".into(),
            version: 1,
            sections: vec![section("news.v1"), section("faults.v1"), section("kpi.v1")],
        };
        let rails = Rails {
            hideable: BTreeSet::from([SectionType::from("news.v1")]),
            mode_editable: BTreeSet::from([SectionType::from("faults.v1")]),
            reorderable: false,
            prop_whitelist: BTreeMap::from([(
                SectionType::from("news.v1"),
                BTreeSet::from(["limit".to_string()]),
            )]),
        };
        let ov = TenantOverride {
            order: Some(vec![SectionType::from("kpi.v1")]), // reorder forbidden
            sections: BTreeMap::from([
                (
                    SectionType::from("news.v1"),
                    SectionPatch {
                        visible: Some(false), // ok: hideable
                        props: BTreeMap::from([
                            ("limit".to_string(), serde_json::json!(5)), // ok: whitelisted
                            ("theme".to_string(), serde_json::json!("dark")), // not whitelisted
                        ]),
                        ..Default::default()
                    },
                ),
                (
                    SectionType::from("kpi.v1"),
                    SectionPatch {
                        visible: Some(false),
                        ..Default::default()
                    }, // not hideable
                ),
                (
                    SectionType::from("news.v1-typo"),
                    SectionPatch::default(), // not in base
                ),
                (
                    SectionType::from("faults.v1"),
                    SectionPatch {
                        mode: Some("compact".into()),
                        ..Default::default()
                    }, // ok
                ),
            ]),
        };
        let errs = validate_tenant_override(&ov, &base, &rails);
        assert!(errs
            .iter()
            .any(|e| matches!(e, ValidationError::NotReorderable)));
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::PropNotWhitelisted { section, prop }
                if section.0 == "news.v1" && prop == "theme")));
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::NotHideable { section } if section.0 == "kpi.v1")));
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::NotInBase { section } if section.0 == "news.v1-typo")));
        // exactly the four violations above — the allowed edits pass
        assert_eq!(errs.len(), 4);
    }

    #[test]
    fn mode_edit_on_non_editable_section_is_rejected() {
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![section("news.v1")],
        };
        let rails = Rails::default();
        let ov = TenantOverride {
            order: None,
            sections: BTreeMap::from([(
                SectionType::from("news.v1"),
                SectionPatch {
                    mode: Some("grid".into()),
                    ..Default::default()
                },
            )]),
        };
        let errs = validate_tenant_override(&ov, &base, &rails);
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::NotModeEditable { section } if section.0 == "news.v1")));
    }
    #[test]
    fn duplicate_order_entries_are_rejected() {
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![section("a.v1"), section("b.v1")],
        };
        let rails = Rails {
            reorderable: true,
            ..Default::default()
        };
        let ov = TenantOverride {
            order: Some(vec![
                SectionType::from("a.v1"),
                SectionType::from("a.v1"),
                SectionType::from("a.v1"),
            ]),
            sections: BTreeMap::new(),
        };
        let errs = validate_tenant_override(&ov, &base, &rails);
        assert_eq!(
            errs,
            vec![
                ValidationError::DuplicateOrderEntry {
                    section: SectionType::from("a.v1")
                },
                ValidationError::DuplicateOrderEntry {
                    section: SectionType::from("a.v1")
                },
            ]
        );
    }

    #[test]
    fn order_entry_not_in_base_is_rejected() {
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![section("a.v1")],
        };
        let rails = Rails {
            reorderable: true,
            ..Default::default()
        };
        let ov = TenantOverride {
            order: Some(vec![
                SectionType::from("a.v1"),
                SectionType::from("ghost.v1"),
            ]),
            sections: BTreeMap::new(),
        };
        let errs = validate_tenant_override(&ov, &base, &rails);
        assert_eq!(
            errs,
            vec![ValidationError::NotInBase {
                section: SectionType::from("ghost.v1")
            }]
        );
    }

    #[test]
    fn valid_reorder_still_passes() {
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![section("a.v1"), section("b.v1"), section("c.v1")],
        };
        let rails = Rails {
            reorderable: true,
            ..Default::default()
        };
        let ov = TenantOverride {
            order: Some(vec![
                SectionType::from("c.v1"),
                SectionType::from("a.v1"),
                SectionType::from("b.v1"),
            ]),
            sections: BTreeMap::new(),
        };
        assert!(validate_tenant_override(&ov, &base, &rails).is_empty());
    }

    #[test]
    fn invalid_default_mode_blocked_at_publish() {
        // Component declares default_mode 'grid' which is NOT in supported_modes ['list'].
        // validate_publish must reject this with InvalidDefaultMode.
        let cfg = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![section("listing-grid.v1")],
        };
        let manifests = vec![RegistryManifest {
            platform: Platform::Web,
            components: std::collections::BTreeMap::from([(
                SectionType::from("listing-grid.v1"),
                ComponentDef {
                    required: false,
                    supported_modes: vec!["list".to_string()],
                    default_mode: Some("grid".to_string()), // inconsistent: not in supported_modes
                },
            )]),
        }];
        let errs = validate_publish(&cfg, &manifests);
        assert!(
            errs.iter().any(|e| matches!(e,
                ValidationError::InvalidDefaultMode { section, mode, platform }
                    if section.0 == "listing-grid.v1"
                    && mode == "grid"
                    && *platform == Platform::Web
            )),
            "expected InvalidDefaultMode error, got: {:?}",
            errs
        );
    }
}
