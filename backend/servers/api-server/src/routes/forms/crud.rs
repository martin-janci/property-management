//! Form CRUD handlers: create, list, available, detail, update, delete,
//! publish, archive, statistics.

use super::types::{
    AvailableFormsResponse, CreateFormRequest, FormActionResponse, FormDetailResponse,
    FormStatisticsResponse, ListFormsQuery, UpdateFormRequest, MAX_DESCRIPTION_LENGTH,
    MAX_FIELDS_PER_FORM, MAX_TITLE_LENGTH,
};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{
    form_status, CreateForm, CreateFormField, CreateFormResponse, FormListQuery, FormListResponse,
    UpdateForm,
};
use uuid::Uuid;

/// Create a new form (Story 54.1).
///
/// Requires manager-level role.
#[utoipa::path(
    post,
    path = "/api/v1/forms",
    request_body = CreateFormRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Form created", body = CreateFormResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden - requires manager role", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn create_form(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Json(req): Json<CreateFormRequest>,
) -> Result<(StatusCode, Json<CreateFormResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Authorization: require manager-level role
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can create forms",
            )),
        ));
    }

    // Validate title
    if req.title.trim().is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("BAD_REQUEST", "Title is required")),
        ));
    }
    if req.title.len() > MAX_TITLE_LENGTH {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!(
                    "Title exceeds maximum length of {} characters",
                    MAX_TITLE_LENGTH
                ),
            )),
        ));
    }

    // Validate description
    if let Some(ref desc) = req.description {
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    format!(
                        "Description exceeds maximum length of {} characters",
                        MAX_DESCRIPTION_LENGTH
                    ),
                )),
            ));
        }
    }

    // Validate fields count
    if req.fields.len() > MAX_FIELDS_PER_FORM {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                format!("Maximum {} fields per form", MAX_FIELDS_PER_FORM),
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let repo = &state.form_repo;

    // Validate and convert fields, returning error on malformed JSON
    let mut fields = Vec::new();
    for f in req.fields {
        // Validate validation_rules JSON if present
        let validation_rules = match f.validation_rules {
            Some(v) => Some(serde_json::from_value(v).map_err(|e| {
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "INVALID_VALIDATION_RULES",
                        format!(
                            "Invalid validation rules JSON for field '{}': {}",
                            f.field_key, e
                        ),
                    )),
                )
            })?),
            None => None,
        };

        fields.push(CreateFormField {
            field_key: f.field_key,
            label: f.label,
            field_type: f.field_type,
            required: f.required,
            help_text: f.help_text,
            placeholder: f.placeholder,
            default_value: f.default_value,
            validation_rules,
            options: f.options.map(|opts| {
                opts.into_iter()
                    .map(|o| db::models::FieldOption {
                        value: o.value,
                        label: o.label,
                    })
                    .collect()
            }),
            field_order: f.field_order,
            width: f.width,
            section: f.section,
            conditional_display: None,
        });
    }

    // Convert request to domain model
    let create_data = CreateForm {
        title: req.title,
        description: req.description,
        category: req.category,
        building_id: req.building_id,
        target_type: req.target_type,
        target_ids: req.target_ids,
        require_signatures: req.require_signatures,
        allow_multiple_submissions: req.allow_multiple_submissions,
        submission_deadline: req.submission_deadline,
        confirmation_message: req.confirmation_message,
        fields,
    };

    let out = repo
        .create(rls.conn(), org_id, user_id, create_data)
        .await
        .map(|form| {
            (
                StatusCode::CREATED,
                Json(CreateFormResponse {
                    id: form.id,
                    message: "Form created successfully".to_string(),
                }),
            )
        })
        .map_err(|e| {
            tracing::error!("Failed to create form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to create form",
                )),
            )
        });
    rls.release().await;
    out
}

/// List all forms (Story 54.1).
#[utoipa::path(
    get,
    path = "/api/v1/forms",
    params(ListFormsQuery),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of forms", body = FormListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn list_forms(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Query(query): Query<ListFormsQuery>,
) -> Result<Json<FormListResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Only managers can see all forms including drafts
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can list all forms",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    let form_query = FormListQuery {
        status: query.status,
        category: query.category,
        building_id: query.building_id,
        search: query.search,
        page: query.page,
        per_page: query.per_page,
        sort_by: query.sort_by,
        sort_order: query.sort_order,
    };

    let out = repo
        .list(rls.conn(), org_id, form_query)
        .await
        .map(|(forms, total)| {
            Json(FormListResponse {
                forms,
                total,
                page: query.page.unwrap_or(1),
                per_page: query.per_page.unwrap_or(20),
            })
        })
        .map_err(|e| {
            tracing::error!("Failed to list forms: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to list forms")),
            )
        });
    rls.release().await;
    out
}

/// List available forms for users (Story 54.2).
#[utoipa::path(
    get,
    path = "/api/v1/forms/available",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of available forms", body = AvailableFormsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn list_available_forms(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<AvailableFormsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    let out = repo
        .list_available_forms(&mut **rls.conn(), org_id, None, "")
        .await
        .map(|forms| Json(AvailableFormsResponse { forms }))
        .map_err(|e| {
            tracing::error!("Failed to list available forms: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list available forms",
                )),
            )
        });
    rls.release().await;
    out
}

/// Get form details.
#[utoipa::path(
    get,
    path = "/api/v1/forms/{id}",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form details", body = FormDetailResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn get_form(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<FormDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let is_manager = rls.role().is_manager();
    let repo = &state.form_repo;

    let out = repo
        .get_with_details(rls.conn(), org_id, id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("INTERNAL_ERROR", "Failed to get form")),
            )
        })
        .and_then(|opt| {
            opt.ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
                )
            })
        })
        .and_then(|form| {
            // Non-managers can only see published forms
            if !is_manager && form.form.status != form_status::PUBLISHED {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
                ))
            } else {
                Ok(Json(FormDetailResponse { form }))
            }
        });
    rls.release().await;
    out
}

/// Update a form.
#[utoipa::path(
    put,
    path = "/api/v1/forms/{id}",
    params(("id" = Uuid, Path, description = "Form ID")),
    request_body = UpdateFormRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form updated", body = FormActionResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn update_form(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateFormRequest>,
) -> Result<Json<FormActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can update forms",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let repo = &state.form_repo;

    // Check if form exists and is editable
    let existing = match repo.get(&mut **rls.conn(), org_id, id).await {
        Ok(Some(existing)) => existing,
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

    if existing.status != form_status::DRAFT {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Only draft forms can be edited",
            )),
        ));
    }

    let update_data = UpdateForm {
        title: req.title,
        description: req.description,
        category: req.category,
        building_id: req.building_id,
        target_type: req.target_type,
        target_ids: req.target_ids,
        require_signatures: req.require_signatures,
        allow_multiple_submissions: req.allow_multiple_submissions,
        submission_deadline: req.submission_deadline,
        confirmation_message: req.confirmation_message,
    };

    let out = repo
        .update(rls.conn(), org_id, id, user_id, update_data)
        .await
        .map(|form| {
            Json(FormActionResponse {
                message: "Form updated successfully".to_string(),
                form,
            })
        })
        .map_err(|e| {
            tracing::error!("Failed to update form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to update form",
                )),
            )
        });
    rls.release().await;
    out
}

/// Delete a form.
#[utoipa::path(
    delete,
    path = "/api/v1/forms/{id}",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form deleted"),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn delete_form(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can delete forms",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    let out = repo
        .delete(&mut **rls.conn(), org_id, id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| {
            tracing::error!("Failed to delete form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to delete form",
                )),
            )
        });
    rls.release().await;
    out
}

/// Publish a form.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/publish",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form published", body = FormActionResponse),
        (status = 400, description = "Cannot publish form", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn publish_form(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<FormActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can publish forms",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let repo = &state.form_repo;

    // Check that form has at least one field
    let fields = match repo.get_fields(&mut **rls.conn(), id).await {
        Ok(fields) => fields,
        Err(e) => {
            tracing::error!("Failed to get form fields: {:?}", e);
            rls.release().await;
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get form fields",
                )),
            ));
        }
    };

    if fields.is_empty() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Cannot publish form without fields",
            )),
        ));
    }

    let out = repo
        .publish(&mut **rls.conn(), org_id, id, user_id)
        .await
        .map(|form| {
            Json(FormActionResponse {
                message: "Form published successfully".to_string(),
                form,
            })
        })
        .map_err(|e| {
            tracing::error!("Failed to publish form: {:?}", e);
            (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Cannot publish form - it may already be published",
                )),
            )
        });
    rls.release().await;
    out
}

/// Archive a form.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/archive",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form archived", body = FormActionResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn archive_form(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<Json<FormActionResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can archive forms",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    let out = repo
        .archive(&mut **rls.conn(), org_id, id)
        .await
        .map(|form| {
            Json(FormActionResponse {
                message: "Form archived successfully".to_string(),
                form,
            })
        })
        .map_err(|e| {
            tracing::error!("Failed to archive form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to archive form",
                )),
            )
        });
    rls.release().await;
    out
}

/// Get form statistics.
#[utoipa::path(
    get,
    path = "/api/v1/forms/statistics",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Form statistics", body = FormStatisticsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn get_statistics(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<FormStatisticsResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can view statistics",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    let out = repo
        .get_statistics(&mut **rls.conn(), org_id)
        .await
        .map(|statistics| Json(FormStatisticsResponse { statistics }))
        .map_err(|e| {
            tracing::error!("Failed to get statistics: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get statistics",
                )),
            )
        });
    rls.release().await;
    out
}
