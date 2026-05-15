//! Manifest completeness gate (Phase 5.5 — defense for leak #17).
//!
//! Asserts that the committed manifest is well-formed and non-empty. The
//! deeper "every tenant table is in the manifest" gate runs in CI via
//! `backend/scripts/check-rls-coverage.sh --strict --emit-manifest -` (which
//! re-extracts and diffs against the committed file). This test is a cheap,
//! offline guarantee that the manifest snapshot is at least loadable and has
//! the structure tenant-ops depends on.

use tenant_ops::TenantDataManifest;

#[test]
fn committed_manifest_is_loadable_and_non_trivial() {
    // Tests run with the crate dir as CWD; jump up two levels to reach
    // backend/manifests/tenant-data-manifest.json.
    let candidate = std::path::PathBuf::from("../../manifests/tenant-data-manifest.json");
    if !candidate.exists() {
        eprintln!(
            "Manifest not present at {:?} — skipping (expected in CI but ok locally).",
            candidate
        );
        return;
    }
    let m = TenantDataManifest::load(&candidate).expect("load manifest");
    assert!(!m.is_empty(), "manifest must not be empty");
    assert!(
        m.len() > 50,
        "manifest has only {} tables — far below the expected ~300+ for this schema; \
         regenerate via `bash backend/scripts/check-rls-coverage.sh --emit-manifest \
         backend/manifests/tenant-data-manifest.json`",
        m.len()
    );
    assert_eq!(
        m.manifest_version, 1,
        "manifest_version=1 is the only known schema"
    );
    let own_org_count = m.own_org_tables().count();
    assert!(
        own_org_count > 0,
        "manifest must have at least one OwnOrg table — purge plan would be empty"
    );
    // A direct sanity probe: the agency_domains table is the keystone of Phase 1
    // tenant resolution; it had better be in the manifest.
    assert!(
        m.find("agency_domains").is_some(),
        "agency_domains must be in the tenant-data manifest"
    );
}
