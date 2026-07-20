use crate::types::*;
use std::collections::BTreeSet;

/// Resolve a screen for one platform + optional tenant, against the platform's
/// registry manifest. Precedence: base → platform override → tenant override →
/// kill flags (spec §3.2). Tenant and kill layers are applied in Tasks 4–5.
///
/// Note: the signature will grow an /client-capabilities parameter when
/// server-side stale-client filtering (spec §4.1) lands in the control-plane plan —
/// do not treat the current arity as frozen.
pub fn resolve(
    base: &ScreenConfig,
    platform: Platform,
    tenant: Option<&TenantOverride>,
    killed: &BTreeSet<SectionType>,
    registry: &RegistryManifest,
) -> ResolvedScreen {
    let mut sections = Vec::with_capacity(base.sections.len());
    let ordered = order_sections(&base.sections, tenant.and_then(|t| t.order.as_deref()));
    for cfg in ordered {
        let Some(def) = registry.components.get(&cfg.section_type) else {
            // Unknown type handling lands in Task 5.
            continue;
        };
        let mut visible = cfg.visible;
        let mut mode = cfg.mode.clone();
        let mut props = cfg.props.clone();
        if let Some(patch) = cfg.overrides.get(&platform) {
            apply_patch(&mut visible, &mut mode, &mut props, patch);
        }
        if let Some(t) = tenant {
            if let Some(patch) = t.sections.get(&cfg.section_type) {
                apply_patch(&mut visible, &mut mode, &mut props, patch);
            }
        }
        // Defensive mode clamp: never emit a mode the component doesn't support.
        // If the fallback default_mode is itself not in supported_modes (inconsistent
        // manifest), clamp to None rather than emitting a mode the component never declared.
        if let Some(m) = &mode {
            if !def.supported_modes.iter().any(|s| s == m) {
                let fallback = def.default_mode.clone();
                mode = fallback.filter(|dm| def.supported_modes.iter().any(|s| s == dm));
            }
        }
        let alive = visible && !killed.contains(&cfg.section_type);
        if alive {
            sections.push(ResolvedSection {
                section_type: cfg.section_type.clone(),
                mode,
                props,
                presentation: Presentation::Visible,
            });
        } else if def.required {
            // Spec §4.2 / §5: required sections never disappear — placeholder,
            // with no mode/props (the section may be killed over bad data).
            sections.push(ResolvedSection {
                section_type: cfg.section_type.clone(),
                mode: None,
                props: Props::new(),
                presentation: Presentation::Placeholder,
            });
        }
        // optional + not alive → omitted entirely (containers own spacing, §4.3)
    }
    ResolvedScreen {
        screen: base.screen.clone(),
        version: base.version,
        sections,
    }
}

fn apply_patch(
    visible: &mut bool,
    mode: &mut Option<String>,
    props: &mut Props,
    patch: &SectionPatch,
) {
    if let Some(v) = patch.visible {
        *visible = v;
    }
    if patch.mode.is_some() {
        *mode = patch.mode.clone();
    }
    for (k, v) in &patch.props {
        props.insert(k.clone(), v.clone());
    }
}

/// Listed types first (in override order), unlisted types after, keeping base
/// relative order. Types in `order` that don't exist in base are ignored.
/// Defensive: duplicate `order` entries are emitted only once (first
/// occurrence wins), so stored-bad data can never duplicate sections in the
/// resolved output — `validate_tenant_override` rejects duplicates at save
/// time, this guards data that bypassed it.
fn order_sections<'a>(
    base: &'a [SectionConfig],
    order: Option<&[SectionType]>,
) -> Vec<&'a SectionConfig> {
    let Some(order) = order else {
        return base.iter().collect();
    };
    let mut out: Vec<&SectionConfig> = Vec::with_capacity(base.len());
    let mut emitted = BTreeSet::new();
    for t in order {
        if !emitted.insert(t) {
            continue; // duplicate order entry — already emitted
        }
        if let Some(cfg) = base.iter().find(|c| &c.section_type == t) {
            out.push(cfg);
        }
    }
    for cfg in base {
        if !emitted.contains(&cfg.section_type) {
            out.push(cfg);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn registry(entries: &[(&str, bool, &[&str])]) -> RegistryManifest {
        RegistryManifest {
            platform: Platform::Web,
            components: entries
                .iter()
                .map(|(t, req, modes)| {
                    (
                        SectionType::from(*t),
                        ComponentDef {
                            required: *req,
                            supported_modes: modes.iter().map(|m| m.to_string()).collect(),
                            default_mode: modes.first().map(|m| m.to_string()),
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
    fn platform_override_beats_base() {
        let mut agent = section("agent-contact.v1");
        agent.mode = Some("sticky-sidebar".into());
        agent.overrides.insert(
            Platform::Mobile,
            SectionPatch {
                mode: Some("bottom-bar".into()),
                ..Default::default()
            },
        );
        let base = ScreenConfig {
            screen: "reality/listing-detail".into(),
            version: 1,
            sections: vec![section("gallery.v1"), agent],
        };
        let reg = registry(&[
            ("gallery.v1", true, &[]),
            ("agent-contact.v1", false, &["sticky-sidebar", "bottom-bar"]),
        ]);

        let web = resolve(&base, Platform::Web, None, &BTreeSet::new(), &reg);
        assert_eq!(web.sections[1].mode.as_deref(), Some("sticky-sidebar"));

        let mobile = resolve(&base, Platform::Mobile, None, &BTreeSet::new(), &reg);
        assert_eq!(mobile.sections[1].mode.as_deref(), Some("bottom-bar"));
        // untouched fields carry through; order preserved
        assert_eq!(
            mobile.sections[0].section_type,
            SectionType::from("gallery.v1")
        );
        assert_eq!(mobile.sections[0].presentation, Presentation::Visible);
        assert_eq!(mobile.screen, "reality/listing-detail");
        assert_eq!(mobile.version, 1);
    }

    #[test]
    fn unsupported_mode_falls_back_to_default_mode() {
        let mut s = section("listing-grid.v1");
        s.mode = Some("hologram".into()); // not supported by the component
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![s],
        };
        let reg = registry(&[("listing-grid.v1", false, &["list", "grid"])]);
        let out = resolve(&base, Platform::Web, None, &BTreeSet::new(), &reg);
        // defensive rendering: never emit a mode the client can't render
        assert_eq!(out.sections[0].mode.as_deref(), Some("list"));
    }

    #[test]
    fn tenant_override_reorders_and_patches_after_platform_layer() {
        let mut a = section("a.v1");
        a.overrides.insert(
            Platform::Web,
            SectionPatch {
                mode: Some("grid".into()),
                ..Default::default()
            },
        );
        let base = ScreenConfig {
            screen: "ppt/dashboard".into(),
            version: 3,
            sections: vec![a, section("b.v1"), section("c.v1")],
        };
        let reg = registry(&[
            ("a.v1", false, &["list", "grid", "map"]),
            ("b.v1", false, &[]),
            ("c.v1", false, &[]),
        ]);
        let tenant = TenantOverride {
            order: Some(vec![
                SectionType::from("c.v1"),
                SectionType::from("a.v1"),
                SectionType::from("b.v1"),
            ]),
            sections: BTreeMap::from([(
                SectionType::from("a.v1"),
                SectionPatch {
                    mode: Some("map".into()),
                    props: BTreeMap::from([("limit".to_string(), serde_json::json!(6))]),
                    ..Default::default()
                },
            )]),
        };
        let out = resolve(&base, Platform::Web, Some(&tenant), &BTreeSet::new(), &reg);
        let types: Vec<&str> = out
            .sections
            .iter()
            .map(|s| s.section_type.0.as_str())
            .collect();
        assert_eq!(types, vec!["c.v1", "a.v1", "b.v1"]);
        // tenant mode beats platform mode (precedence §3.2)
        assert_eq!(out.sections[1].mode.as_deref(), Some("map"));
        assert_eq!(out.sections[1].props["limit"], serde_json::json!(6));
    }

    #[test]
    fn tenant_order_omitting_a_type_keeps_it_after_listed_ones() {
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![section("a.v1"), section("b.v1"), section("c.v1")],
        };
        let reg = registry(&[
            ("a.v1", false, &[]),
            ("b.v1", false, &[]),
            ("c.v1", false, &[]),
        ]);
        let tenant = TenantOverride {
            order: Some(vec![SectionType::from("b.v1")]),
            sections: BTreeMap::new(),
        };
        let out = resolve(&base, Platform::Web, Some(&tenant), &BTreeSet::new(), &reg);
        let types: Vec<&str> = out
            .sections
            .iter()
            .map(|s| s.section_type.0.as_str())
            .collect();
        // listed first, unlisted keep base relative order
        assert_eq!(types, vec!["b.v1", "a.v1", "c.v1"]);
    }

    #[test]
    fn duplicate_order_entries_emit_each_section_once() {
        // Stored-bad data: order repeats "a.v1" many times. Resolve must emit
        // each section exactly once (first occurrence wins).
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![section("a.v1"), section("b.v1")],
        };
        let reg = registry(&[("a.v1", false, &[]), ("b.v1", false, &[])]);
        let tenant = TenantOverride {
            order: Some(vec![SectionType::from("a.v1"); 10]),
            sections: BTreeMap::new(),
        };
        let out = resolve(&base, Platform::Web, Some(&tenant), &BTreeSet::new(), &reg);
        let types: Vec<&str> = out
            .sections
            .iter()
            .map(|s| s.section_type.0.as_str())
            .collect();
        assert_eq!(types, vec!["a.v1", "b.v1"]);
    }

    /// Finding 1a: if default_mode itself is not in supported_modes, clamp to None.
    #[test]
    fn inconsistent_default_mode_clamps_to_none() {
        // Component declares supported_modes: ["list"] but default_mode: Some("grid").
        // Section requests mode "hologram" -> not in supported_modes -> falls back to
        // default_mode "grid" -> but "grid" is also not in supported_modes -> must be None.
        let mut s = section("listing-grid.v1");
        s.mode = Some("hologram".into());
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![s],
        };
        let reg = RegistryManifest {
            platform: Platform::Web,
            components: std::collections::BTreeMap::from([(
                SectionType::from("listing-grid.v1"),
                ComponentDef {
                    required: false,
                    supported_modes: vec!["list".to_string()],
                    default_mode: Some("grid".to_string()), // inconsistent: not in supported_modes
                },
            )]),
        };
        let out = resolve(&base, Platform::Web, None, &BTreeSet::new(), &reg);
        assert_eq!(
            out.sections[0].mode, None,
            "inconsistent default_mode must clamp to None, not emit an unsupported mode"
        );
    }

    #[test]
    fn hidden_and_killed_sections_collapse_or_placeholder() {
        let mut hidden_opt = section("similar-listings.v1");
        hidden_opt.visible = false;
        let mut hidden_req = section("price-box.v1");
        hidden_req.visible = false;
        hidden_req.props = BTreeMap::from([("currency".to_string(), serde_json::json!("EUR"))]);
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![
                section("gallery.v1"),       // required, visible, killed below
                hidden_req,                  // required, hidden → placeholder
                hidden_opt,                  // optional, hidden → gone
                section("mortgage-calc.v1"), // optional, killed below → gone
                section("unknown.v9"),       // not in registry → gone
            ],
        };
        let reg = registry(&[
            ("gallery.v1", true, &[]),
            ("price-box.v1", true, &[]),
            ("similar-listings.v1", false, &[]),
            ("mortgage-calc.v1", false, &[]),
        ]);
        let killed = BTreeSet::from([
            SectionType::from("gallery.v1"),
            SectionType::from("mortgage-calc.v1"),
        ]);
        let out = resolve(&base, Platform::Web, None, &killed, &reg);
        let rendered: Vec<(&str, Presentation)> = out
            .sections
            .iter()
            .map(|s| (s.section_type.0.as_str(), s.presentation))
            .collect();
        assert_eq!(
            rendered,
            vec![
                ("gallery.v1", Presentation::Placeholder), // killed required
                ("price-box.v1", Presentation::Placeholder), // hidden required
            ]
        );
        // placeholders leak no props (section may be killed for a data bug)
        assert!(out.sections.iter().all(|s| s.props.is_empty()));
    }
}
