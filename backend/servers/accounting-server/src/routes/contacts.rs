//! Contacts & CRM routes (EPIC-ACC-03 / UC-ACC-03). OWNED BY: BE-Contacts+Catalog.
//!
//! Resource-server handlers: `AuthUser` + `RlsConnection`, RBAC via
//! `accounting_core::authz`, persistence via
//! `db::repositories::acc_contacts::AccContactsRepository`. `nest`ed under
//! `/api/v1/contacts`.
//!
//! Every mutating handler records an append-only audit entry
//! (`accounting_core::audit` -> `platform_repo.record_audit_rls`) in the SAME
//! transaction as the mutation (UC-ACC-16.7). Reads require Manager+; mutations
//! require Manager+ too (ACC MVP gate — `authz::{READ,MUTATE}_MIN_ROLE`).

use accounting_core::{audit, authz};
use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use db::models::acc_contacts_ext::{AccContactAddress, AccContactExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Connection;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create/update payload for a contact (UC-ACC-03.1/.9).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContactRequest {
    pub name: String,
    pub kind: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub iban: Option<String>,
    pub default_currency: Option<String>,
    pub default_price_level: Option<String>,
    pub default_payment_terms_days: Option<i32>,
    pub default_language: Option<String>,
}

/// Request to merge a source contact into a target (UC-ACC-03.5).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MergeContactRequest {
    pub target_id: Uuid,
}

/// Validate the `kind` discriminator against the 00195 CHECK
/// (`customer | supplier | both`). Keeps a bad enum out of the DB and returns
/// `400` rather than a `500` from a constraint violation (UC-ACC-03.1).
fn valid_contact_kind(kind: &str) -> bool {
    matches!(kind, "customer" | "supplier" | "both")
}

/// Build a fresh [`AccContactExt`] for an INSERT from the request + tenant.
/// `id`/`created_at`/`updated_at` are placeholders the DB overwrites; the
/// CRM-only fields not present on the create form default to None / sensible
/// values (active, no merge target).
fn new_contact_from_request(tenant_id: Uuid, req: &ContactRequest) -> AccContactExt {
    AccContactExt {
        id: Uuid::nil(),
        tenant_id,
        name: req.name.clone(),
        ico: req.ico.clone(),
        dic: req.dic.clone(),
        email: req.email.clone(),
        phone: req.phone.clone(),
        address: req.address.clone(),
        iban: req.iban.clone(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        kind: req.kind.clone(),
        is_active: true,
        default_currency: req.default_currency.clone(),
        default_price_level: req.default_price_level.clone(),
        default_payment_terms_days: req.default_payment_terms_days,
        default_discount_pct: None,
        default_language: req.default_language.clone(),
        vat_verified_at: None,
        vat_verified_valid: None,
        credit_limit: None,
        notes: None,
        merged_into_id: None,
    }
}

/// List contacts for the active company (UC-ACC-03.1). Hides deactivated/merged
/// contacts so pickers only show live parties.
#[utoipa::path(
    get,
    path = "/api/v1/contacts",
    responses(
        (status = 200, description = "Contacts", body = [AccContactExt]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Contacts"
)]
pub async fn list_contacts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccContactExt>>, StatusCode> {
    // UC-ACC-16.2: server-side RBAC regardless of UI state.
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let contacts = state
        .contacts_repo
        .list_contacts_ext_rls(&mut **rls, false)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list contacts: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(contacts))
}

/// Get a single contact (UC-ACC-03.1).
#[utoipa::path(
    get,
    path = "/api/v1/contacts/{id}",
    params(("id" = Uuid, Path, description = "Contact id")),
    responses(
        (status = 200, description = "Contact", body = AccContactExt),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    tag = "Contacts"
)]
pub async fn get_contact(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<AccContactExt>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let contact = state
        .contacts_repo
        .find_contact_ext_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find contact: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    contact.map(Json).ok_or(StatusCode::NOT_FOUND)
}

/// Create a contact (UC-ACC-03.1). Persists + audits in one transaction.
#[utoipa::path(
    post,
    path = "/api/v1/contacts",
    request_body = ContactRequest,
    responses(
        (status = 201, description = "Created", body = AccContactExt),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Contacts"
)]
pub async fn create_contact(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    _auth: AuthUser,
    Json(req): Json<ContactRequest>,
) -> Result<(StatusCode, Json<AccContactExt>), StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    if req.name.trim().is_empty() || !valid_contact_kind(&req.kind) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let tenant_id = rls.tenant_id();
    let actor_id = rls.user_id();
    let to_insert = new_contact_from_request(tenant_id, &req);

    // Mutation + audit entry share one transaction (UC-ACC-16.7 append-only).
    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let created = state
        .contacts_repo
        .create_contact_ext_rls(&mut *tx, to_insert)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create contact: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::AuditBuilder::new(tenant_id, Some(actor_id), "create", "contact")
        .entity_id(created.id)
        .diff(json!({ "name": created.name, "kind": created.kind }))
        .build();
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(created)))
}

/// Update a contact (UC-ACC-03.1/.9). Loads the current row, overlays the
/// request fields, persists, and audits — all in one transaction.
#[utoipa::path(
    patch,
    path = "/api/v1/contacts/{id}",
    params(("id" = Uuid, Path, description = "Contact id")),
    request_body = ContactRequest,
    responses(
        (status = 200, description = "Updated", body = AccContactExt),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Contacts"
)]
pub async fn update_contact(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    _auth: AuthUser,
    Json(req): Json<ContactRequest>,
) -> Result<Json<AccContactExt>, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    if req.name.trim().is_empty() || !valid_contact_kind(&req.kind) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let tenant_id = rls.tenant_id();
    let actor_id = rls.user_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Load current to preserve CRM-only fields (discount/credit/verification/
    // merge target) the create form does not carry.
    let mut existing = match state
        .contacts_repo
        .find_contact_ext_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find contact: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })? {
        Some(c) => c,
        None => return Err(StatusCode::NOT_FOUND),
    };

    // Overlay editable fields from the request; keep the rest as stored.
    existing.name = req.name.clone();
    existing.kind = req.kind.clone();
    existing.ico = req.ico.clone();
    existing.dic = req.dic.clone();
    existing.email = req.email.clone();
    existing.phone = req.phone.clone();
    existing.address = req.address.clone();
    existing.iban = req.iban.clone();
    existing.default_currency = req.default_currency.clone();
    existing.default_price_level = req.default_price_level.clone();
    existing.default_payment_terms_days = req.default_payment_terms_days;
    existing.default_language = req.default_language.clone();

    let updated = match state
        .contacts_repo
        .update_contact_ext_rls(&mut *tx, id, existing)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update contact: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })? {
        Some(c) => c,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let entry = audit::AuditBuilder::new(tenant_id, Some(actor_id), "update", "contact")
        .entity_id(id)
        .diff(json!({ "name": updated.name, "kind": updated.kind }))
        .build();
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(Json(updated))
}

/// Deactivate (soft-delete) a contact (UC-ACC-03.1). Never hard-deletes: the
/// contact is hidden from pickers but its historical documents stay intact.
#[utoipa::path(
    delete,
    path = "/api/v1/contacts/{id}",
    params(("id" = Uuid, Path, description = "Contact id")),
    responses(
        (status = 204, description = "Deactivated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Contacts"
)]
pub async fn deactivate_contact(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    _auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let tenant_id = rls.tenant_id();
    let actor_id = rls.user_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    state
        .contacts_repo
        .deactivate_contact_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to deactivate contact: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::AuditBuilder::new(tenant_id, Some(actor_id), "deactivate", "contact")
        .entity_id(id)
        .build();
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}

/// Merge a contact into another (UC-ACC-03.5). Re-points documents/history to
/// the surviving contact transactionally with no data loss, then marks the
/// source merged + inactive. The re-key, the flag, and the audit entry all live
/// in one transaction.
#[utoipa::path(
    post,
    path = "/api/v1/contacts/{id}/merge",
    params(("id" = Uuid, Path, description = "Source contact id")),
    request_body = MergeContactRequest,
    responses(
        (status = 204, description = "Merged"),
        (status = 400, description = "Cannot merge a contact into itself"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Contacts"
)]
pub async fn merge_contact(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    _auth: AuthUser,
    Json(req): Json<MergeContactRequest>,
) -> Result<StatusCode, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    // A contact can never be merged into itself (would orphan the surviving row).
    if id == req.target_id {
        return Err(StatusCode::BAD_REQUEST);
    }

    let tenant_id = rls.tenant_id();
    let actor_id = rls.user_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    state
        .contacts_repo
        .merge_contacts_rls(&mut tx, id, req.target_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to merge contacts: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::AuditBuilder::new(tenant_id, Some(actor_id), "merge", "contact")
        .entity_id(id)
        .diff(json!({ "merged_into_id": req.target_id }))
        .build();
    state
        .platform_repo
        .record_audit_rls(&mut *tx, entry)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(StatusCode::NO_CONTENT)
}

/// List a contact's addresses (UC-ACC-03.4).
#[utoipa::path(
    get,
    path = "/api/v1/contacts/{id}/addresses",
    params(("id" = Uuid, Path, description = "Contact id")),
    responses(
        (status = 200, description = "Addresses", body = [AccContactAddress]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Contacts"
)]
pub async fn list_addresses(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccContactAddress>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let addresses = state
        .contacts_repo
        .list_addresses_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list addresses: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(addresses))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_contact_kind() {
        // UC-ACC-03.1: kind must satisfy the 00195 CHECK constraint.
        assert!(valid_contact_kind("customer"));
        assert!(valid_contact_kind("supplier"));
        assert!(valid_contact_kind("both"));
        assert!(!valid_contact_kind("vendor"));
        assert!(!valid_contact_kind(""));
    }

    #[test]
    fn new_contact_defaults_active_and_unverified() {
        // UC-ACC-03.1/.2: a freshly created contact is active with no VAT
        // verification yet (graceful-degradation default — verification is async).
        let req = ContactRequest {
            name: "ACME s.r.o.".into(),
            kind: "customer".into(),
            ico: Some("12345678".into()),
            dic: None,
            email: None,
            phone: None,
            address: None,
            iban: None,
            default_currency: Some("EUR".into()),
            default_price_level: Some("wholesale".into()),
            default_payment_terms_days: Some(14),
            default_language: Some("sk".into()),
        };
        let tenant = Uuid::new_v4();
        let c = new_contact_from_request(tenant, &req);
        assert_eq!(c.tenant_id, tenant);
        assert!(c.is_active);
        assert_eq!(c.vat_verified_at, None);
        assert_eq!(c.vat_verified_valid, None);
        assert_eq!(c.merged_into_id, None);
        assert_eq!(c.default_currency.as_deref(), Some("EUR"));
        assert_eq!(c.default_price_level.as_deref(), Some("wholesale"));
        assert_eq!(c.default_payment_terms_days, Some(14));
    }
}
