use super::document_access_allowed;
use chrono::Utc;
use db::models::Document;
use serde_json::json;
use uuid::Uuid;

fn make_doc(
    access_scope: &str,
    created_by: Uuid,
    access_roles: serde_json::Value,
    access_target_ids: serde_json::Value,
) -> Document {
    Document {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        folder_id: None,
        title: "Test".into(),
        description: None,
        category: "general".into(),
        file_key: "test/key".into(),
        file_name: "test.pdf".into(),
        mime_type: "application/pdf".into(),
        size_bytes: 0,
        access_scope: access_scope.into(),
        access_roles,
        access_target_ids,
        created_by,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        version_number: 1,
        parent_document_id: None,
        is_current_version: true,
        template_id: None,
        generation_metadata: None,
    }
}

#[test]
fn creator_always_allowed() {
    let uid = Uuid::new_v4();
    let doc = make_doc("users", uid, json!([]), json!([]));
    assert!(document_access_allowed(&doc, uid, "tenant"));
}

#[test]
fn organization_scope_allows_any_user() {
    let doc = make_doc("organization", Uuid::new_v4(), json!([]), json!([]));
    assert!(document_access_allowed(&doc, Uuid::new_v4(), "tenant"));
}

#[test]
fn role_scope_allows_matching_role() {
    let doc = make_doc(
        "role",
        Uuid::new_v4(),
        json!(["property_manager", "accountant"]),
        json!([]),
    );
    assert!(document_access_allowed(
        &doc,
        Uuid::new_v4(),
        "property_manager"
    ));
    assert!(!document_access_allowed(&doc, Uuid::new_v4(), "tenant"));
}

#[test]
fn users_scope_allows_listed_user_id() {
    let uid = Uuid::new_v4();
    let other = Uuid::new_v4();
    let doc = make_doc("users", Uuid::new_v4(), json!([]), json!([uid.to_string()]));
    assert!(document_access_allowed(&doc, uid, "tenant"));
    assert!(!document_access_allowed(&doc, other, "tenant"));
}

#[test]
fn unknown_scope_denies_non_creator() {
    let doc = make_doc("private", Uuid::new_v4(), json!([]), json!([]));
    assert!(!document_access_allowed(&doc, Uuid::new_v4(), "tenant"));
}

// ----------------------------------------------------------------------------
// Story 7A.3 acceptance-criteria coverage
// ----------------------------------------------------------------------------

/// AC-2: a document restricted to the "owner" role must NOT be visible to a
/// tenant. Guards the role-mismatch denial path of the download/preview gate.
#[test]
fn ac2_role_scoped_owners_only_denies_tenant() {
    let doc = make_doc("role", Uuid::new_v4(), json!(["owner"]), json!([]));
    // A non-creator owner is allowed...
    assert!(document_access_allowed(&doc, Uuid::new_v4(), "owner"));
    // ...but a tenant is denied (the document is not their list / download).
    assert!(!document_access_allowed(&doc, Uuid::new_v4(), "tenant"));
}

/// AC-3 (negative): a document shared with a specific set of users must NOT be
/// accessible to a user who is not in that set, even when several users are
/// listed.
#[test]
fn ac3_user_scope_denies_user_not_in_share_list() {
    let shared_a = Uuid::new_v4();
    let shared_b = Uuid::new_v4();
    let outsider = Uuid::new_v4();
    let doc = make_doc(
        "users",
        Uuid::new_v4(),
        json!([]),
        json!([shared_a.to_string(), shared_b.to_string()]),
    );
    assert!(document_access_allowed(&doc, shared_a, "tenant"));
    assert!(document_access_allowed(&doc, shared_b, "tenant"));
    assert!(!document_access_allowed(&doc, outsider, "tenant"));
}

/// AC-3: the document creator retains access even when the `users` share list
/// does not include them (creator-always-allowed wins over scope).
#[test]
fn ac3_creator_retains_access_when_not_in_user_list() {
    let creator = Uuid::new_v4();
    let doc = make_doc(
        "users",
        creator,
        json!([]),
        json!([Uuid::new_v4().to_string()]),
    );
    assert!(document_access_allowed(&doc, creator, "tenant"));
}

/// AC-1 / building & unit scope — regression guard.
///
/// `document_access_allowed` is the *in-memory* gate used by the download /
/// preview handlers. It deliberately resolves only creator / organization /
/// role / users scope, because building- and unit-scope resolution needs the
/// caller's building/unit membership, which the SQL gate
/// (`DocumentRepository::list_accessible_rls` / `check_access_rls`) owns.
///
/// This test pins that contract: a non-creator with no membership context is
/// denied a building/unit-scoped document by the in-memory gate. If a future
/// change wires building/unit membership into this helper, update this guard
/// alongside it (see story 7A.3 AC-1 — the read path must then grant building
/// residents access via the SQL gate).
#[test]
fn building_and_unit_scope_denied_by_in_memory_gate() {
    let building_doc = make_doc(
        "building",
        Uuid::new_v4(),
        json!([]),
        json!([Uuid::new_v4().to_string()]),
    );
    assert!(!document_access_allowed(
        &building_doc,
        Uuid::new_v4(),
        "owner"
    ));

    let unit_doc = make_doc(
        "unit",
        Uuid::new_v4(),
        json!([]),
        json!([Uuid::new_v4().to_string()]),
    );
    assert!(!document_access_allowed(&unit_doc, Uuid::new_v4(), "owner"));

    // The creator still gets in regardless of building/unit scope.
    let creator = Uuid::new_v4();
    let creator_doc = make_doc("building", creator, json!([]), json!([]));
    assert!(document_access_allowed(&creator_doc, creator, "tenant"));
}
