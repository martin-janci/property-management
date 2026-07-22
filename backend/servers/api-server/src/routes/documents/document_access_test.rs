use super::{
    document_access_allowed, scope_membership_allows, validate_document_org_scope,
    validate_file_key_org_scope, validate_upload_url_request,
};
use axum::http::StatusCode;
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

// ----------------------------------------------------------------------------
// GH #1413 — building/unit scope resolved from caller membership.
//
// `document_access_allowed` (above) stays membership-free; building/unit
// resolution lives in `scope_membership_allows`, OR'd into the download/preview
// handlers so a building/unit member is no longer 404'd. These pin that helper.
// ----------------------------------------------------------------------------

#[test]
fn building_scope_granted_to_building_member() {
    let b = Uuid::new_v4();
    let doc = make_doc(
        "building",
        Uuid::new_v4(),
        json!([]),
        json!([b.to_string()]),
    );
    // Member of the targeted building is granted.
    assert!(scope_membership_allows(&doc, &[b], &[]));
    // No membership → denied.
    assert!(!scope_membership_allows(&doc, &[], &[]));
    // Membership in a different building → denied.
    assert!(!scope_membership_allows(&doc, &[Uuid::new_v4()], &[]));
}

#[test]
fn unit_scope_granted_to_unit_member() {
    let u = Uuid::new_v4();
    let doc = make_doc("unit", Uuid::new_v4(), json!([]), json!([u.to_string()]));
    // Resident/owner of the targeted unit is granted.
    assert!(scope_membership_allows(&doc, &[], &[u]));
    // Building membership does not grant unit scope.
    assert!(!scope_membership_allows(&doc, &[Uuid::new_v4()], &[]));
}

#[test]
fn membership_never_grants_non_building_unit_scopes() {
    for scope in ["organization", "role", "users", "private"] {
        let doc = make_doc(scope, Uuid::new_v4(), json!([]), json!([]));
        assert!(!scope_membership_allows(
            &doc,
            &[Uuid::new_v4()],
            &[Uuid::new_v4()]
        ));
    }
}

// ----------------------------------------------------------------------------
// gap-84-1 — presigned direct-to-S3 upload URL request validation.
//
// `validate_upload_url_request` is the pure up-front guard `create_upload_url`
// applies to the client's declared MIME type / size before minting a presigned
// PUT URL. These pin the accept/reject contract without needing an S3 client.
// ----------------------------------------------------------------------------

#[test]
fn upload_url_request_accepts_allowed_type_and_size() {
    // Allowed MIME with a within-limit size; the validated size is echoed back.
    assert_eq!(
        validate_upload_url_request("application/pdf", Some(1024)).expect("valid request"),
        1024
    );
    // Exactly at the cap is allowed.
    assert!(validate_upload_url_request("image/jpeg", Some(db::models::MAX_FILE_SIZE)).is_ok());
}

#[test]
fn upload_url_request_rejects_missing_size() {
    // GH #2320: size_bytes is required — it is signed into the presigned PUT
    // URL as Content-Length, which is what actually enforces the 50 MiB cap
    // on the direct-to-S3 path.
    let err = validate_upload_url_request("image/png", None)
        .expect_err("missing size_bytes must be rejected");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

#[test]
fn upload_url_request_rejects_disallowed_mime_type() {
    let err = validate_upload_url_request("application/x-msdownload", Some(10))
        .expect_err("executable MIME must be rejected");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

#[test]
fn upload_url_request_rejects_oversize_hint() {
    let too_big = db::models::MAX_FILE_SIZE + 1;
    let err = validate_upload_url_request("application/pdf", Some(too_big))
        .expect_err("oversize hint must be rejected before minting a URL");
    assert_eq!(err.0, StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn upload_url_request_rejects_negative_size() {
    let err = validate_upload_url_request("application/pdf", Some(-1))
        .expect_err("negative size must be rejected");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

// ----------------------------------------------------------------------------
// GH #2320 — org-scoped file_key guard at document registration.
//
// `validate_file_key_org_scope` is the pure guard `create_document` applies to
// the client-supplied `file_key` before persisting it. Without it, any org
// member could register a "document" pointing at an arbitrary bucket object
// (another org's files, `messages/{thread_id}/…` attachments) and exfiltrate
// it via the presigned download/preview handlers — a bucket-wide IDOR
// (mirrors the messaging guard from #1791/#1770).
// ----------------------------------------------------------------------------

// ----------------------------------------------------------------------------
// #2422 — application-layer org scoping for get_document.
//
// `get_document` reads via `find_by_id_with_details_rls`, whose SQL scopes only
// by `id` and leans on the `documents` FORCE-RLS policy for org isolation. The
// FORCE policy is the primary control but fails open on a BYPASSRLS/superuser
// pool, so `validate_document_org_scope` re-checks the org application-side.
// These pin the accept/reject contract (404, shared error shape) without a DB —
// mirroring the inline guard in get_download_url / get_preview_url /
// intelligence.rs.
// ----------------------------------------------------------------------------

#[test]
fn document_org_scope_accepts_same_org() {
    let org_id = Uuid::new_v4();
    assert!(validate_document_org_scope(org_id, org_id).is_ok());
}

#[test]
fn document_org_scope_rejects_foreign_org_as_404() {
    let document_org = Uuid::new_v4();
    let tenant_org = Uuid::new_v4();
    let err = validate_document_org_scope(document_org, tenant_org)
        .expect_err("foreign-org document must be rejected");
    // A cross-org id must be indistinguishable from a missing one — 404, not a
    // 403, so the existence of the document is not leaked.
    assert_eq!(err.0, StatusCode::NOT_FOUND);
}

#[test]
fn file_key_own_org_prefix_accepted() {
    let org_id = Uuid::new_v4();
    // Shape emitted by integrations::generate_storage_key.
    let key = format!("{org_id}/2026/07/{}_report.pdf", Uuid::new_v4());
    assert!(validate_file_key_org_scope(&key, org_id).is_ok());
}

#[test]
fn file_key_foreign_org_prefix_rejected() {
    let org_id = Uuid::new_v4();
    let other_org = Uuid::new_v4();
    let key = format!("{other_org}/2026/07/{}_secret.pdf", Uuid::new_v4());
    let err = validate_file_key_org_scope(&key, org_id)
        .expect_err("foreign-org file_key must be rejected");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

#[test]
fn file_key_messages_prefix_rejected() {
    let org_id = Uuid::new_v4();
    let key = format!("messages/{}/{}", Uuid::new_v4(), Uuid::new_v4());
    let err = validate_file_key_org_scope(&key, org_id)
        .expect_err("message-attachment file_key must be rejected");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

#[test]
fn file_key_with_dot_dot_rejected_even_under_own_prefix() {
    let org_id = Uuid::new_v4();
    let key = format!("{org_id}/../{}/2026/07/escape.pdf", Uuid::new_v4());
    let err =
        validate_file_key_org_scope(&key, org_id).expect_err("path traversal must be rejected");
    assert_eq!(err.0, StatusCode::BAD_REQUEST);
}

#[test]
fn file_key_empty_and_bare_prefix_variants_rejected() {
    let org_id = Uuid::new_v4();
    // Empty key.
    assert!(validate_file_key_org_scope("", org_id).is_err());
    // Org id without the trailing slash separator.
    assert!(validate_file_key_org_scope(&org_id.to_string(), org_id).is_err());
    // Prefix-substring trick: `{org_id}abc/...` must not match `{org_id}/`.
    let tricky = format!("{org_id}abc/2026/07/file.pdf");
    assert!(validate_file_key_org_scope(&tricky, org_id).is_err());
}
