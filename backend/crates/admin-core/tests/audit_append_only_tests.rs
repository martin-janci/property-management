//! Audit append-only tests.
//!
//! The trigger that REJECTs UPDATE/DELETE on `audit_logs` lives in migration
//! 00138. This test crate does not bring up a Postgres instance, so we
//! verify two things:
//!
//!   1. The migration file exists.
//!   2. It contains the trigger function + UPDATE / DELETE triggers (so the
//!      migration cannot drift out from under us without a test failure).
//!
//! The DB-level enforcement test (a real INSERT then an UPDATE that must
//! fail with `feature_not_supported`) lives in
//! `backend/servers/api-server/tests/admin_routes_tests.rs::audit_logs_*`
//! once the test harness has a live Postgres handle.

use std::fs;
use std::path::PathBuf;

fn migration_path() -> PathBuf {
    // Tests run with CWD = crate root (backend/crates/admin-core).
    PathBuf::from("../db/migrations/00138_create_capability_grants.sql")
}

fn migration_text() -> String {
    fs::read_to_string(migration_path()).unwrap_or_else(|e| {
        panic!(
            "failed to read migration {:?}: {}\nCWD: {:?}",
            migration_path(),
            e,
            std::env::current_dir().unwrap()
        )
    })
}

#[test]
fn migration_file_exists() {
    assert!(
        migration_path().exists(),
        "migration 00138 must exist at {:?}",
        migration_path()
    );
}

#[test]
fn migration_defines_audit_logs_reject_function() {
    let sql = migration_text();
    assert!(
        sql.contains("CREATE OR REPLACE FUNCTION audit_logs_reject_mutation"),
        "migration must define the trigger function"
    );
    assert!(
        sql.contains("RAISE EXCEPTION"),
        "trigger function must RAISE EXCEPTION on mutation"
    );
}

#[test]
fn migration_attaches_update_and_delete_triggers() {
    let sql = migration_text();
    assert!(
        sql.contains("trg_audit_logs_no_update"),
        "must attach the UPDATE trigger to audit_logs"
    );
    assert!(
        sql.contains("trg_audit_logs_no_delete"),
        "must attach the DELETE trigger to audit_logs"
    );
    assert!(
        sql.contains("BEFORE UPDATE ON audit_logs"),
        "UPDATE trigger must fire BEFORE UPDATE"
    );
    assert!(
        sql.contains("BEFORE DELETE ON audit_logs"),
        "DELETE trigger must fire BEFORE DELETE"
    );
}

#[test]
fn migration_protects_capability_grants_against_self_grant_at_app_layer() {
    // The DB does not enforce no-self-grant — the app layer does. This is
    // an architectural choice (the brainstorming session calls it out as a
    // capability-registry concern, not a DB constraint). Confirm the SQL
    // does NOT silently add a constraint that would conflict with the app
    // layer's validation.
    let sql = migration_text();
    assert!(
        !sql.contains("CHECK (granted_by != user_id)"),
        "no-self-grant is enforced at the app layer; do not add a DB CHECK that conflicts"
    );
}

#[test]
fn migration_marks_capability_grants_unique_per_live_grant() {
    // Defense: a user must not hold the same capability twice
    // simultaneously. Revoked rows are excluded.
    let sql = migration_text();
    assert!(
        sql.contains("UNIQUE INDEX") && sql.contains("WHERE revoked_at IS NULL"),
        "capability_grants must have a unique index excluding revoked rows"
    );
}
