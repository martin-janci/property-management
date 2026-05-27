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
