//! Product & price-list catalog routes (EPIC-ACC-04 / UC-ACC-04). OWNED BY: BE-Contacts+Catalog.
//!
//! Resource-server handlers: `AuthUser` + `RlsConnection`, RBAC via
//! `accounting_core::authz`, persistence via
//! `db::repositories::acc_catalog::AccCatalogRepository`. `nest`ed under
//! `/api/v1/catalog`.
//!
//! Mutations record an append-only audit entry (`accounting_core::audit` ->
//! `platform_repo.record_audit_rls`) in the same transaction (UC-ACC-16.7).

use accounting_core::{audit, authz};
use api_core::extractors::{AuthUser, RlsConnection};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use db::models::acc_catalog::{AccCatalogItem, AccItemCategory, AccPriceLevel};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::Connection;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Create/update payload for a catalog item (UC-ACC-04.1/.3).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CatalogItemRequest {
    pub code: Option<String>,
    pub name: String,
    pub description: Option<String>,
    pub kind: String,
    pub unit_id: Option<Uuid>,
    pub vat_rate: Decimal,
    pub unit_price: Decimal,
    pub price_is_gross: bool,
    pub category_id: Option<Uuid>,
    pub stock_tracked: bool,
}

/// Create payload for a price level (UC-ACC-04.2).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PriceLevelRequest {
    pub name: String,
    pub price: Decimal,
    pub price_is_gross: bool,
    pub currency: String,
}

/// Validate item `kind` against the 00195 CHECK (`goods | service`) so a bad
/// value returns `400` rather than a constraint `500` (UC-ACC-04.1).
fn valid_item_kind(kind: &str) -> bool {
    matches!(kind, "goods" | "service")
}

/// Build a fresh [`AccCatalogItem`] for an INSERT from the request + tenant.
/// `id`/timestamps are DB-defaulted; a created item is active by default.
/// Used by both explicit create and inline quick-add (UC-ACC-04.1/.8).
fn new_item_from_request(tenant_id: Uuid, req: &CatalogItemRequest) -> AccCatalogItem {
    AccCatalogItem {
        id: Uuid::nil(),
        tenant_id,
        code: req.code.clone(),
        name: req.name.clone(),
        description: req.description.clone(),
        kind: req.kind.clone(),
        unit_id: req.unit_id,
        vat_rate: req.vat_rate,
        unit_price: req.unit_price,
        price_is_gross: req.price_is_gross,
        category_id: req.category_id,
        stock_tracked: req.stock_tracked,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// List catalog items (UC-ACC-04.1). Hides deactivated items from the picker.
#[utoipa::path(
    get,
    path = "/api/v1/catalog/items",
    responses(
        (status = 200, description = "Catalog items", body = [AccCatalogItem]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Catalog"
)]
pub async fn list_items(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccCatalogItem>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let items = state
        .catalog_repo
        .list_items_rls(&mut **rls, false)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list catalog items: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(items))
}

/// Get a catalog item (UC-ACC-04.1).
#[utoipa::path(
    get,
    path = "/api/v1/catalog/items/{id}",
    params(("id" = Uuid, Path, description = "Item id")),
    responses(
        (status = 200, description = "Item", body = AccCatalogItem),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found")
    ),
    tag = "Catalog"
)]
pub async fn get_item(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<AccCatalogItem>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let item = state
        .catalog_repo
        .find_item_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find catalog item: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    item.map(Json).ok_or(StatusCode::NOT_FOUND)
}

/// Create a catalog item (UC-ACC-04.1/.8 inline quick-add). Persisting from a
/// document line creates a FULL, reusable catalog item — the same path as an
/// explicit create.
#[utoipa::path(
    post,
    path = "/api/v1/catalog/items",
    request_body = CatalogItemRequest,
    responses(
        (status = 201, description = "Created", body = AccCatalogItem),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Catalog"
)]
pub async fn create_item(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    _auth: AuthUser,
    Json(req): Json<CatalogItemRequest>,
) -> Result<(StatusCode, Json<AccCatalogItem>), StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    if req.name.trim().is_empty() || !valid_item_kind(&req.kind) || req.vat_rate < Decimal::ZERO {
        return Err(StatusCode::BAD_REQUEST);
    }

    let tenant_id = rls.tenant_id();
    let actor_id = rls.user_id();
    let to_insert = new_item_from_request(tenant_id, &req);

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let created = state
        .catalog_repo
        .create_item_rls(&mut *tx, to_insert)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create catalog item: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let entry = audit::AuditBuilder::new(tenant_id, Some(actor_id), "create", "catalog_item")
        .entity_id(created.id)
        .diff(json!({ "name": created.name, "code": created.code, "kind": created.kind }))
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

/// Update a catalog item (UC-ACC-04.1). Editing does NOT mutate past documents
/// — those carry their own snapshotted line values.
#[utoipa::path(
    patch,
    path = "/api/v1/catalog/items/{id}",
    params(("id" = Uuid, Path, description = "Item id")),
    request_body = CatalogItemRequest,
    responses(
        (status = 200, description = "Updated", body = AccCatalogItem),
        (status = 400, description = "Validation failed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    tag = "Catalog"
)]
pub async fn update_item(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
    _auth: AuthUser,
    Json(req): Json<CatalogItemRequest>,
) -> Result<Json<AccCatalogItem>, StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    if req.name.trim().is_empty() || !valid_item_kind(&req.kind) || req.vat_rate < Decimal::ZERO {
        return Err(StatusCode::BAD_REQUEST);
    }

    let tenant_id = rls.tenant_id();
    let actor_id = rls.user_id();

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start transaction: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Preserve `is_active` (managed by deactivate) by loading the current row.
    let mut existing = match state
        .catalog_repo
        .find_item_rls(&mut *tx, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find catalog item: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })? {
        Some(i) => i,
        None => return Err(StatusCode::NOT_FOUND),
    };

    existing.code = req.code.clone();
    existing.name = req.name.clone();
    existing.description = req.description.clone();
    existing.kind = req.kind.clone();
    existing.unit_id = req.unit_id;
    existing.vat_rate = req.vat_rate;
    existing.unit_price = req.unit_price;
    existing.price_is_gross = req.price_is_gross;
    existing.category_id = req.category_id;
    existing.stock_tracked = req.stock_tracked;

    let updated = match state
        .catalog_repo
        .update_item_rls(&mut *tx, id, existing)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update catalog item: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })? {
        Some(i) => i,
        None => return Err(StatusCode::NOT_FOUND),
    };

    let entry = audit::AuditBuilder::new(tenant_id, Some(actor_id), "update", "catalog_item")
        .entity_id(id)
        .diff(json!({ "name": updated.name, "unit_price": updated.unit_price.to_string() }))
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

/// List categories (UC-ACC-04.4).
#[utoipa::path(
    get,
    path = "/api/v1/catalog/categories",
    responses(
        (status = 200, description = "Categories", body = [AccItemCategory]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Catalog"
)]
pub async fn list_categories(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccItemCategory>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let categories = state
        .catalog_repo
        .list_categories_rls(&mut **rls)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list categories: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(categories))
}

/// List price levels for an item (UC-ACC-04.2).
#[utoipa::path(
    get,
    path = "/api/v1/catalog/items/{id}/price-levels",
    params(("id" = Uuid, Path, description = "Item id")),
    responses(
        (status = 200, description = "Price levels", body = [AccPriceLevel]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Catalog"
)]
pub async fn list_price_levels(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccPriceLevel>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let levels = state
        .catalog_repo
        .list_price_levels_rls(&mut **rls, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list price levels: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(levels))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_item_kind() {
        // UC-ACC-04.1: kind must satisfy the 00195 CHECK (goods | service).
        assert!(valid_item_kind("goods"));
        assert!(valid_item_kind("service"));
        assert!(!valid_item_kind("subscription"));
        assert!(!valid_item_kind(""));
    }

    #[test]
    fn inline_quick_add_persists_full_active_item() {
        // UC-ACC-04.8: an item created inline from a document line is a full,
        // active catalog item (reusable later), not a partial stub.
        let req = CatalogItemRequest {
            code: Some("SKU-1".into()),
            name: "Consulting hour".into(),
            description: Some("Senior rate".into()),
            kind: "service".into(),
            unit_id: None,
            vat_rate: Decimal::new(2000, 2),    // 20.00 %
            unit_price: Decimal::new(10000, 2), // 100.00 (net)
            price_is_gross: false,
            category_id: None,
            stock_tracked: false,
        };
        let tenant = Uuid::new_v4();
        let item = new_item_from_request(tenant, &req);
        assert_eq!(item.tenant_id, tenant);
        assert!(item.is_active);
        assert_eq!(item.code.as_deref(), Some("SKU-1"));
        assert_eq!(item.kind, "service");
        assert!(!item.price_is_gross);
        assert!(!item.stock_tracked);
    }
}
