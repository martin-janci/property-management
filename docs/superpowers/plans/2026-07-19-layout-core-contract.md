# Layout Core Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `layout-core` Rust crate — config types, layered merge resolver, and publish/rails validation for the Layout & Content Manager — as pure, DB-free, fully unit-tested logic.

**Architecture:** New workspace crate `backend/crates/layout-core` implementing the contract from `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md` §3–§5: screen configs are flat section lists; resolution merges `base → platform override → tenant override → kill flags` against a per-platform component registry manifest; required sections degrade to placeholders, optional ones collapse; publish and tenant-save validation are hard gates. No HTTP, no SQL — those come in the control-plane plan.

**Tech Stack:** Rust 2021 (workspace `rust-version = 1.75`), serde/serde_json, thiserror. No new external dependencies.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md` — this plan implements §3 (core model), §4 rules 1–2 (resolver side), §5 (kill semantics), §6.3 (validation gate logic only).
- Crate must compile without a database (repo convention: DB-free compile; runtime sqlx only in `db` crate).
- Additive-only evolution: all config structs get `#[serde(default)]`-friendly fields and must ignore unknown JSON fields (serde default behavior — do NOT add `deny_unknown_fields`).
- `required` comes from the registry manifest, never from config (spec §3.1).
- Merge precedence, exactly: base config → platform override → tenant override → kill flags (spec §3.2).
- Commit scope: `feat(layout-core): …` / `test(layout-core): …` per repo Conventional Commits.
- Pre-push gate: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p layout-core` (run inside `backend/`).

## File Structure

```
backend/crates/layout-core/
├── Cargo.toml
└── src/
    ├── lib.rs        # module wiring + re-exports
    ├── types.rs      # config, override, registry, rails, resolved types
    ├── resolve.rs    # layered merge resolver
    └── validate.rs   # publish gate + tenant-rails validation
backend/Cargo.toml    # + "crates/layout-core" workspace member
docs/repo-map.md      # + one line for the new crate
```

---

### Task 1: Crate scaffold + config types + serde round-trip

**Files:**
- Create: `backend/crates/layout-core/Cargo.toml`
- Create: `backend/crates/layout-core/src/lib.rs`
- Create: `backend/crates/layout-core/src/types.rs`
- Modify: `backend/Cargo.toml` (workspace `members` list, after `"crates/common"`)
- Test: inline `#[cfg(test)]` in `types.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces (used by every later task): `Platform` (enum `Web | Mobile`, serde lowercase, `Ord`), `SectionType(pub String)`, `Props = BTreeMap<String, serde_json::Value>`, `SectionPatch { visible: Option<bool>, mode: Option<String>, props: Props }`, `SectionConfig { section_type, visible, mode, props, overrides: BTreeMap<Platform, SectionPatch> }`, `ScreenConfig { screen: String, version: u32, sections: Vec<SectionConfig> }`.

- [ ] **Step 1: Register the crate in the workspace**

In `backend/Cargo.toml`, add to `members` after `"crates/common"`:

```toml
    "crates/layout-core",
```

- [ ] **Step 2: Create `backend/crates/layout-core/Cargo.toml`**

```toml
[package]
name = "layout-core"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
```

- [ ] **Step 3: Create `src/lib.rs`**

```rust
//! Layout & Content Manager contract: config types, merge resolver, validation.
//!
//! Spec: docs/superpowers/specs/2026-07-19-layout-content-manager-design.md

pub mod types;

pub use types::*;
```

- [ ] **Step 4: Write the failing round-trip test**

Create `src/types.rs` containing ONLY the test module for now:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
```

- [ ] **Step 5: Run test to verify it fails**

Run: `cd backend && cargo test -p layout-core`
Expected: FAIL to compile — `ScreenConfig`, `Platform` not defined.

- [ ] **Step 6: Implement the types**

Add above the test module in `src/types.rs`:

```rust
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
```

- [ ] **Step 7: Run test to verify it passes**

Run: `cd backend && cargo test -p layout-core`
Expected: PASS (1 test).

- [ ] **Step 8: Commit**

```bash
git add backend/Cargo.toml backend/crates/layout-core
git commit -m "feat(layout-core): scaffold crate with screen config types"
```

---

### Task 2: Registry manifest, rails, tenant override + resolved-output types

**Files:**
- Modify: `backend/crates/layout-core/src/types.rs`
- Test: inline `#[cfg(test)]` in `types.rs`

**Interfaces:**
- Consumes: Task 1 types.
- Produces: `ComponentDef { required: bool, supported_modes: Vec<String>, default_mode: Option<String> }`, `RegistryManifest { platform: Platform, components: BTreeMap<SectionType, ComponentDef> }`, `TenantOverride { order: Option<Vec<SectionType>>, sections: BTreeMap<SectionType, SectionPatch> }`, `Rails { hideable, mode_editable: BTreeSet<SectionType>, reorderable: bool, prop_whitelist: BTreeMap<SectionType, BTreeSet<String>> }`, `Presentation` (enum `Visible | Placeholder`), `ResolvedSection { section_type, mode, props, presentation }`, `ResolvedScreen { screen, version, sections }`.

- [ ] **Step 1: Write the failing test**

Append inside the existing `mod tests`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd backend && cargo test -p layout-core`
Expected: FAIL to compile — `RegistryManifest`, `TenantOverride`, `Rails` not defined.

- [ ] **Step 3: Implement the types**

Append to `src/types.rs` (above the test module). Also add `BTreeSet` to the existing `use std::collections::BTreeMap;` import, making it `use std::collections::{BTreeMap, BTreeSet};`:

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd backend && cargo test -p layout-core`
Expected: PASS (2 tests).

- [ ] **Step 5: Commit**

```bash
git add backend/crates/layout-core/src/types.rs
git commit -m "feat(layout-core): add registry, rails, tenant override and resolved types"
```

---

### Task 3: Resolver — platform merge precedence

**Files:**
- Create: `backend/crates/layout-core/src/resolve.rs`
- Modify: `backend/crates/layout-core/src/lib.rs`
- Test: inline `#[cfg(test)]` in `resolve.rs`

**Interfaces:**
- Consumes: all Task 1–2 types.
- Produces: `pub fn resolve(base: &ScreenConfig, platform: Platform, tenant: Option<&TenantOverride>, killed: &BTreeSet<SectionType>, registry: &RegistryManifest) -> ResolvedScreen` and (crate-private) `fn apply_patch(visible: &mut bool, mode: &mut Option<String>, props: &mut Props, patch: &SectionPatch)`. Tasks 4–5 extend `resolve` internals; the signature is FINAL as written here.

- [ ] **Step 1: Wire the module**

In `src/lib.rs` add:

```rust
pub mod resolve;

pub use resolve::resolve;
```

- [ ] **Step 2: Write the failing test**

Create `src/resolve.rs`:

```rust
use crate::types::*;
use std::collections::BTreeSet;

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
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd backend && cargo test -p layout-core`
Expected: FAIL to compile — `resolve` not defined.

- [ ] **Step 4: Implement the resolver core**

Add above the test module in `src/resolve.rs`:

```rust
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
```

Note: `visible` is computed but not yet consumed — visibility/placeholder handling lands in Task 5. Silence the unused-variable lint until then by adding `let _ = visible;` on the line immediately before `sections.push(...)`. Task 5 deletes that line.

- [ ] **Step 5: Run test to verify it passes**

Run: `cd backend && cargo test -p layout-core`
Expected: PASS (4 tests).

- [ ] **Step 6: Commit**

```bash
git add backend/crates/layout-core/src
git commit -m "feat(layout-core): resolver with platform merge precedence and mode clamp"
```

---

### Task 4: Resolver — tenant override layer

**Files:**
- Modify: `backend/crates/layout-core/src/resolve.rs`
- Test: inline `#[cfg(test)]` in `resolve.rs`

**Interfaces:**
- Consumes: `resolve` signature from Task 3 (unchanged).
- Produces: tenant `order` reordering + per-section tenant patches applied after the platform layer. Rails enforcement is NOT here — it happens at save-time validation (Task 7); the resolver trusts stored overrides.

- [ ] **Step 1: Write the failing test**

Append inside `mod tests` in `src/resolve.rs`:

```rust
    #[test]
    fn tenant_override_reorders_and_patches_after_platform_layer() {
        let mut a = section("a.v1");
        a.overrides.insert(
            Platform::Web,
            SectionPatch { mode: Some("grid".into()), ..Default::default() },
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
        let types: Vec<&str> =
            out.sections.iter().map(|s| s.section_type.0.as_str()).collect();
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
        let reg = registry(&[("a.v1", false, &[]), ("b.v1", false, &[]), ("c.v1", false, &[])]);
        let tenant = TenantOverride {
            order: Some(vec![SectionType::from("b.v1")]),
            sections: BTreeMap::new(),
        };
        let out = resolve(&base, Platform::Web, Some(&tenant), &BTreeSet::new(), &reg);
        let types: Vec<&str> =
            out.sections.iter().map(|s| s.section_type.0.as_str()).collect();
        // listed first, unlisted keep base relative order
        assert_eq!(types, vec!["b.v1", "a.v1", "c.v1"]);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd backend && cargo test -p layout-core`
Expected: FAIL — tenant layer not applied (`mode` is `"grid"`, order unchanged).

- [ ] **Step 3: Implement the tenant layer**

In `resolve()`: delete the `let _ = (tenant, killed);` line and replace it with `let _ = killed;` (kills land in Task 5). Then:

(a) Apply the tenant patch after the platform patch. Between the platform `apply_patch` call and the mode clamp, insert:

```rust
        if let Some(t) = tenant {
            if let Some(patch) = t.sections.get(&cfg.section_type) {
                apply_patch(&mut visible, &mut mode, &mut props, patch);
            }
        }
```

(b) Reorder before iterating. Replace `for cfg in &base.sections {` with:

```rust
    let ordered = order_sections(&base.sections, tenant.and_then(|t| t.order.as_deref()));
    for cfg in ordered {
```

and add below `apply_patch` at the bottom of the file:

```rust
/// Listed types first (in override order), unlisted types after, keeping base
/// relative order. Types in `order` that don't exist in base are ignored.
fn order_sections<'a>(
    base: &'a [SectionConfig],
    order: Option<&[SectionType]>,
) -> Vec<&'a SectionConfig> {
    let Some(order) = order else {
        return base.iter().collect();
    };
    let mut out: Vec<&SectionConfig> = Vec::with_capacity(base.len());
    for t in order {
        if let Some(cfg) = base.iter().find(|c| &c.section_type == t) {
            out.push(cfg);
        }
    }
    for cfg in base {
        if !order.contains(&cfg.section_type) {
            out.push(cfg);
        }
    }
    out
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd backend && cargo test -p layout-core`
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add backend/crates/layout-core/src/resolve.rs
git commit -m "feat(layout-core): apply tenant override layer in resolver"
```

---

### Task 5: Resolver — visibility, kill flags, placeholders, unknown types

**Files:**
- Modify: `backend/crates/layout-core/src/resolve.rs`
- Test: inline `#[cfg(test)]` in `resolve.rs`

**Interfaces:**
- Consumes: `resolve` signature from Task 3 (unchanged).
- Produces: final resolver semantics per spec §4 rules 1–3 and §5 — hidden/killed optional → omitted from output; hidden/killed required → `Presentation::Placeholder` with empty props; unknown type → omitted (optional-equivalent, since `required` is unknowable without a registry entry). The control-plane plan adds render-log metrics on top; nothing else changes here.

- [ ] **Step 1: Write the failing test**

Append inside `mod tests`:

```rust
    #[test]
    fn hidden_and_killed_sections_collapse_or_placeholder() {
        let mut hidden_opt = section("similar-listings.v1");
        hidden_opt.visible = false;
        let mut hidden_req = section("price-box.v1");
        hidden_req.visible = false;
        hidden_req.props =
            BTreeMap::from([("currency".to_string(), serde_json::json!("EUR"))]);
        let base = ScreenConfig {
            screen: "s".into(),
            version: 1,
            sections: vec![
                section("gallery.v1"),      // required, visible, killed below
                hidden_req,                 // required, hidden → placeholder
                hidden_opt,                 // optional, hidden → gone
                section("mortgage-calc.v1"),// optional, killed below → gone
                section("unknown.v9"),      // not in registry → gone
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
                ("gallery.v1", Presentation::Placeholder),   // killed required
                ("price-box.v1", Presentation::Placeholder), // hidden required
            ]
        );
        // placeholders leak no props (section may be killed for a data bug)
        assert!(out.sections.iter().all(|s| s.props.is_empty()));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd backend && cargo test -p layout-core`
Expected: FAIL — hidden/killed sections still emitted as `Visible`.

- [ ] **Step 3: Implement final presentation logic**

In `resolve()`: delete the `let _ = killed;` line and the `let _ = visible;` line from Task 3. Replace the `sections.push(...)` block with:

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cd backend && cargo test -p layout-core`
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add backend/crates/layout-core/src/resolve.rs
git commit -m "feat(layout-core): kill flags, required placeholders, unknown-type filtering"
```

---

### Task 6: Publish validation gate

**Files:**
- Create: `backend/crates/layout-core/src/validate.rs`
- Modify: `backend/crates/layout-core/src/lib.rs`
- Test: inline `#[cfg(test)]` in `validate.rs`

**Interfaces:**
- Consumes: Task 1–2 types.
- Produces: `pub enum ValidationError` (thiserror, variants below — FINAL, Task 7 adds four more variants) and `pub fn validate_publish(config: &ScreenConfig, manifests: &[RegistryManifest]) -> Vec<ValidationError>` (empty vec = publishable). The control-plane plan maps non-empty results to HTTP 422.

- [ ] **Step 1: Wire the module**

In `src/lib.rs` add:

```rust
pub mod validate;

pub use validate::{validate_publish, ValidationError};
```

- [ ] **Step 2: Write the failing test**

Create `src/validate.rs`:

```rust
use crate::types::*;
use thiserror::Error;

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
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd backend && cargo test -p layout-core`
Expected: FAIL to compile — `validate_publish`, `ValidationError` not defined.

- [ ] **Step 4: Implement the gate**

Add above the test module in `src/validate.rs`:

```rust
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
                if !def.supported_modes.iter().any(|sm| *sm == mode) {
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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd backend && cargo test -p layout-core`
Expected: PASS (10 tests).

- [ ] **Step 6: Commit**

```bash
git add backend/crates/layout-core/src
git commit -m "feat(layout-core): publish validation gate"
```

---

### Task 7: Tenant-override rails validation

**Files:**
- Modify: `backend/crates/layout-core/src/validate.rs`
- Modify: `backend/crates/layout-core/src/lib.rs`
- Test: inline `#[cfg(test)]` in `validate.rs`

**Interfaces:**
- Consumes: Task 6 `ValidationError` (extended here), Task 2 `Rails`/`TenantOverride`.
- Produces: `pub fn validate_tenant_override(ov: &TenantOverride, base: &ScreenConfig, rails: &Rails) -> Vec<ValidationError>` — the server-side rails enforcement for tenant saves (spec §3.4/§6.3). New `ValidationError` variants: `NotInBase`, `NotHideable`, `NotModeEditable`, `PropNotWhitelisted`, `NotReorderable`.

- [ ] **Step 1: Export the new function**

In `src/lib.rs`, change the validate re-export line to:

```rust
pub use validate::{validate_publish, validate_tenant_override, ValidationError};
```

- [ ] **Step 2: Write the failing test**

Append inside `mod tests` in `src/validate.rs`:

```rust
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
                        visible: Some(false),                    // ok: hideable
                        props: BTreeMap::from([
                            ("limit".to_string(), serde_json::json!(5)), // ok: whitelisted
                            ("theme".to_string(), serde_json::json!("dark")), // not whitelisted
                        ]),
                        ..Default::default()
                    },
                ),
                (
                    SectionType::from("kpi.v1"),
                    SectionPatch { visible: Some(false), ..Default::default() }, // not hideable
                ),
                (
                    SectionType::from("news.v1-typo"),
                    SectionPatch::default(), // not in base
                ),
                (
                    SectionType::from("faults.v1"),
                    SectionPatch { mode: Some("compact".into()), ..Default::default() }, // ok
                ),
            ]),
        };
        let errs = validate_tenant_override(&ov, &base, &rails);
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::NotReorderable)));
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
                SectionPatch { mode: Some("grid".into()), ..Default::default() },
            )]),
        };
        let errs = validate_tenant_override(&ov, &base, &rails);
        assert!(errs.iter().any(|e| matches!(e,
            ValidationError::NotModeEditable { section } if section.0 == "news.v1")));
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cd backend && cargo test -p layout-core`
Expected: FAIL to compile — `validate_tenant_override` and new variants not defined.

- [ ] **Step 4: Implement rails enforcement**

Add the new variants to `ValidationError`:

```rust
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
```

Add the function above the test module:

```rust
/// Server-side rails enforcement for tenant saves (spec §3.4). The UI hides
/// out-of-rails controls, but this is the actual gate.
pub fn validate_tenant_override(
    ov: &TenantOverride,
    base: &ScreenConfig,
    rails: &Rails,
) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    if ov.order.is_some() && !rails.reorderable {
        errs.push(ValidationError::NotReorderable);
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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cd backend && cargo test -p layout-core`
Expected: PASS (12 tests).

- [ ] **Step 6: Commit**

```bash
git add backend/crates/layout-core/src
git commit -m "feat(layout-core): tenant-override rails validation"
```

---

### Task 8: Workspace gates + repo-map entry

**Files:**
- Modify: `docs/repo-map.md:54` (crates list under `backend/ (Rust workspace)`)
- Test: full workspace static gates

**Interfaces:**
- Consumes: everything above.
- Produces: green `fmt`/`clippy`/`test` across the workspace; repo-map documents the crate for future agents (repo rule: repo-map fixes ship in the same PR).

- [ ] **Step 1: Add the repo-map line**

In `docs/repo-map.md`, in the **Crates** list after the `common` bullet, add:

```markdown
- `layout-core` — Layout & Content Manager contract: screen configs, merge resolver
  (base → platform → tenant → kill), publish/rails validation. Pure logic, no DB.
  Spec: `docs/superpowers/specs/2026-07-19-layout-content-manager-design.md`.
```

- [ ] **Step 2: Run the full backend gate**

Run: `cd backend && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test -p layout-core`
Expected: fmt clean, clippy clean, 12 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add docs/repo-map.md backend
git commit -m "docs(repo-map): add layout-core crate entry"
```

---

## Out of scope (subsequent plans)

Per the spec's rollout (§9), each of these is its own plan, in order:

1. **Control plane** — migrations (`layout_configs`, `layout_config_versions`, `layout_tenant_overrides`, `layout_kill_flags`), repositories, superadmin + tenant routes, resolved `GET /layout/{screen}` endpoints in api-server + reality-server, TypeSpec.
2. **Defensive rendering** — pilot screens (`ppt/dashboard`, `reality/listing-detail`): section registries in ppt-web/reality-web, gap-owned spacing, error boundaries, placeholder component, ISR revalidation hook.
3. **Superadmin editor MVP** — `@ppt/layout-editor` package + admin-web mount, kill-switch UI, publish/rollback.
4. **Tenant editor + rails authoring** — ppt-web scoped mount, rails UI in admin-web.
5. **Live preview bridge**; then mobile manifests + RN/KMP renderers.
