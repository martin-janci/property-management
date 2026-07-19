use crate::types::*;
use std::collections::BTreeSet;

/// Resolve a screen for one platform + optional tenant, against the platform's
/// registry manifest. Precedence: base → platform override → tenant override →
/// kill flags (spec §3.2). Tenant and kill layers are applied in Tasks 4–5.
pub fn resolve(
    base: &ScreenConfig,
    platform: Platform,
    tenant: Option<&TenantOverride>,
    killed: &BTreeSet<SectionType>,
    registry: &RegistryManifest,
) -> ResolvedScreen {
    let _ = (tenant, killed); // applied in later layers of this function
    let mut sections = Vec::with_capacity(base.sections.len());
    for cfg in &base.sections {
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
        // Defensive mode clamp: never emit a mode the component doesn't support.
        if let Some(m) = &mode {
            if !def.supported_modes.iter().any(|s| s == m) {
                mode = def.default_mode.clone();
            }
        }
        let _ = visible;
        sections.push(ResolvedSection {
            section_type: cfg.section_type.clone(),
            mode,
            props,
            presentation: Presentation::Visible,
        });
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
        assert_eq!(mobile.sections[0].section_type, SectionType::from("gallery.v1"));
        assert_eq!(mobile.sections[0].presentation, Presentation::Visible);
        assert_eq!(mobile.screen, "reality/listing-detail");
        assert_eq!(mobile.version, 1);
    }

    #[test]
    fn unsupported_mode_falls_back_to_default_mode() {
        let mut s = section("listing-grid.v1");
        s.mode = Some("hologram".into()); // not supported by the component
        let base = ScreenConfig { screen: "s".into(), version: 1, sections: vec![s] };
        let reg = registry(&[("listing-grid.v1", false, &["list", "grid"])]);
        let out = resolve(&base, Platform::Web, None, &BTreeSet::new(), &reg);
        // defensive rendering: never emit a mode the client can't render
        assert_eq!(out.sections[0].mode.as_deref(), Some("list"));
    }
}
