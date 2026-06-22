//! Form field handlers: list, add, update, delete, reorder.

use super::types::{
    CreateFormFieldRequest, FieldActionResponse, FieldsResponse, ReorderFieldsRequest,
    UpdateFormFieldRequest,
};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{form_status, CreateFormField, UpdateFormField};
use uuid::Uuid;

/// List form fields.
#[utoipa::path(
    get,
    path = "/api/v1/forms/{id}/fields",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form fields", body = FieldsResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn list_fields(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<FieldsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    // Verify form exists and user has access
    match repo.get(&mut **rls.conn(), org_id, id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get form: {:?}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
            ));
        }
    }

    let out = repo
        .get_fields(&mut **rls.conn(), id)
        .await
        .map(|fields| Json(FieldsResponse { fields }))
        .map_err(|e| {
            tracing::error!("Failed to get fields: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get fields")),
            )
        });
    rls.release().await;
    out
}

/// Add a field to a form.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/fields",
    params(("id" = Uuid, Path, description = "Form ID")),
    request_body = CreateFormFieldRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Field added", body = FieldActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn add_field(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateFormFieldRequest>,
) -> Result<(StatusCode, Json<FieldActionResponse>), (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can add fields",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    // Verify form exists and is editable
    let form = match repo.get(&mut **rls.conn(), org_id, id).await {
        Ok(Some(form)) => form,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get form: {:?}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
            ));
        }
    };

    if form.status != form_status::DRAFT {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only add fields to draft forms",
            )),
        ));
    }

    let field_data = CreateFormField {
        field_key: req.field_key,
        label: req.label,
        field_type: req.field_type,
        required: req.required,
        help_text: req.help_text,
        placeholder: req.placeholder,
        default_value: req.default_value,
        validation_rules: req
            .validation_rules
            .map(|v| serde_json::from_value(v).unwrap_or_default()),
        options: req.options.map(|opts| {
            opts.into_iter()
                .map(|o| db::models::FieldOption {
                    value: o.value,
                    label: o.label,
                })
                .collect()
        }),
        field_order: req.field_order,
        width: req.width,
        section: req.section,
        conditional_display: None,
    };

    let out = repo
        .create_field(&mut **rls.conn(), id, field_data, req.field_order)
        .await
        .map(|field| {
            (
                StatusCode::CREATED,
                Json(FieldActionResponse {
                    message: "Field added successfully".to_string(),
                    field,
                }),
            )
        })
        .map_err(|e| {
            tracing::error!("Failed to add field: {:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Failed to add field - field key may already exist",
                )),
            )
        });
    rls.release().await;
    out
}

/// Update a form field.
#[utoipa::path(
    put,
    path = "/api/v1/forms/{id}/fields/{field_id}",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ("field_id" = Uuid, Path, description = "Field ID")
    ),
    request_body = UpdateFormFieldRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Field updated", body = FieldActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Field not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn update_field(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((id, field_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateFormFieldRequest>,
) -> Result<Json<FieldActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can update fields",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    // Verify form exists and is editable
    let form = match repo.get(&mut **rls.conn(), org_id, id).await {
        Ok(Some(form)) => form,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get form: {:?}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
            ));
        }
    };

    if form.status != form_status::DRAFT {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only update fields in draft forms",
            )),
        ));
    }

    let update_data = UpdateFormField {
        label: req.label,
        field_type: req.field_type,
        required: req.required,
        help_text: req.help_text,
        placeholder: req.placeholder,
        default_value: req.default_value,
        validation_rules: req
            .validation_rules
            .map(|v| serde_json::from_value(v).unwrap_or_default()),
        options: req.options.map(|opts| {
            opts.into_iter()
                .map(|o| db::models::FieldOption {
                    value: o.value,
                    label: o.label,
                })
                .collect()
        }),
        field_order: req.field_order,
        width: req.width,
        section: req.section,
        conditional_display: None,
    };

    let out = repo
        .update_field(&mut **rls.conn(), id, field_id, update_data)
        .await
        .map(|field| {
            Json(FieldActionResponse {
                message: "Field updated successfully".to_string(),
                field,
            })
        })
        .map_err(|e| {
            tracing::error!("Failed to update field: {:?}", e);
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Field not found")),
            )
        });
    rls.release().await;
    out
}

/// Delete a form field.
#[utoipa::path(
    delete,
    path = "/api/v1/forms/{id}/fields/{field_id}",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ("field_id" = Uuid, Path, description = "Field ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 204, description = "Field deleted"),
        (status = 404, description = "Field not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn delete_field(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((id, field_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete fields",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    // Verify form exists and is editable
    let form = match repo.get(&mut **rls.conn(), org_id, id).await {
        Ok(Some(form)) => form,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get form: {:?}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
            ));
        }
    };

    if form.status != form_status::DRAFT {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only delete fields from draft forms",
            )),
        ));
    }

    let out = repo
        .delete_field(&mut **rls.conn(), id, field_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::error!("Failed to delete field: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete field",
                )),
            )
        });
    rls.release().await;
    out
}

/// Reorder form fields.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/fields/reorder",
    params(("id" = Uuid, Path, description = "Form ID")),
    request_body = ReorderFieldsRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Fields reordered"),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn reorder_fields(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<ReorderFieldsRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can reorder fields",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    // Verify form exists and is editable
    let form = match repo.get(&mut **rls.conn(), org_id, id).await {
        Ok(Some(form)) => form,
        Ok(None) => {
            rls.release().await;
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
            ));
        }
        Err(e) => {
            tracing::error!("Failed to get form: {:?}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
            ));
        }
    };

    if form.status != form_status::DRAFT {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only reorder fields in draft forms",
            )),
        ));
    }

    let field_orders: Vec<(Uuid, i32)> = req
        .field_orders
        .into_iter()
        .map(|fo| (fo.field_id, fo.order))
        .collect();

    let out = repo
        .reorder_fields(&mut **rls.conn(), id, field_orders)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| {
            tracing::error!("Failed to reorder fields: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to reorder fields",
                )),
            )
        });
    rls.release().await;
    out
}
