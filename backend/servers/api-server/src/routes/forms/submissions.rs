//! Form submission handlers: submit, list, detail, review, and download
//! tracking.

use super::types::{
    validate_signature_image, ListSubmissionsQuery, ReviewSubmissionRequest,
    SubmissionDetailResponse, SubmitFormRequest,
};
use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::StatusCode,
    Json,
};
use common::errors::ErrorResponse;
use db::models::{
    form_status, FormSubmissionParams, ReviewSubmission, SubmissionListQuery,
    SubmissionListResponse, SubmitForm, SubmitFormResponse,
};
use uuid::Uuid;

/// Submit a form (Story 54.3).
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/submit",
    params(("id" = Uuid, Path, description = "Form ID")),
    request_body = SubmitFormRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, description = "Form submitted", body = SubmitFormResponse),
        (status = 400, description = "Invalid submission", body = ErrorResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn submit_form(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    headers: axum::http::HeaderMap,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    Path(id): Path<Uuid>,
    Json(req): Json<SubmitFormRequest>,
) -> Result<(StatusCode, Json<SubmitFormResponse>), (StatusCode, Json<ErrorResponse>)> {
    // Resolve the client IP through the trusted-proxy allowlist (issue #2789):
    // forwarding headers (`X-Forwarded-For` / `CF-Connecting-IP`) are only
    // believed when the socket peer is a trusted proxy, otherwise a client could
    // forge the source IP recorded on the submission / signature. Always yields
    // an address (falls back to the socket peer), matching the prior contract of
    // an always-populated `Option<String>`.
    let ip_address = Some(crate::client_ip::resolve_client_ip(
        &headers,
        addr,
        &state.trusted_proxies,
    ));

    // Extract User-Agent from headers
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let repo = &state.form_repo;

    // Get form details
    let form = match repo.get_with_details(rls.conn(), org_id, id).await {
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

    // Verify form is published
    if form.form.status != form_status::PUBLISHED {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Can only submit published forms",
            )),
        ));
    }

    // Check if user already submitted and multiple submissions not allowed
    if !form.form.allow_multiple_submissions {
        let has_submitted = match repo
            .has_user_submitted(&mut **rls.conn(), id, user_id)
            .await
        {
            Ok(has_submitted) => has_submitted,
            Err(e) => {
                tracing::error!("Failed to check submission: {:?}", e);
                rls.release().await;
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to check submission",
                    )),
                ));
            }
        };

        if has_submitted {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "You have already submitted this form",
                )),
            ));
        }
    }

    // Check deadline
    if let Some(deadline) = form.form.submission_deadline {
        if chrono::Utc::now() > deadline {
            rls.release().await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new(
                    "BAD_REQUEST",
                    "Submission deadline has passed",
                )),
            ));
        }
    }

    // Validate required fields
    for field in &form.fields {
        if field.required {
            let field_value = req.data.get(&field.field_key);
            if field_value.is_none()
                || field_value.map(|v| v.is_null()).unwrap_or(true)
                || field_value
                    .and_then(|v| v.as_str())
                    .map(|s| s.is_empty())
                    .unwrap_or(false)
            {
                rls.release().await;
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::new(
                        "BAD_REQUEST",
                        format!("Field '{}' is required", field.label),
                    )),
                ));
            }
        }
    }

    // Check signature requirement
    if form.form.require_signatures && req.signature_data.is_none() {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Signature is required for this form",
            )),
        ));
    }

    let submit_data = SubmitForm {
        data: req.data,
        attachments: req.attachments.map(|atts| {
            atts.into_iter()
                .map(|a| db::models::FormAttachment {
                    field_key: a.field_key,
                    file_id: a.file_id,
                    filename: a.filename,
                    mime_type: a.mime_type,
                    size: a.size,
                })
                .collect()
        }),
        signature_data: match req.signature_data {
            Some(s) => {
                // Validate signature image before processing
                validate_signature_image(&s.signature_image)?;
                Some(db::models::SignatureData {
                    signature_image: s.signature_image,
                    signed_at: chrono::Utc::now(),
                    ip_address: ip_address.clone(),
                    user_agent: user_agent.clone(),
                })
            }
            None => None,
        },
    };

    let confirmation_message = form.form.confirmation_message.clone();

    let out = repo
        .submit(
            &mut **rls.conn(),
            FormSubmissionParams {
                org_id,
                form_id: id,
                user_id,
                building_id: None, // could be extracted from user context if needed
                unit_id: None,     // could be extracted from user context if needed
                data: submit_data,
                ip_address: ip_address.clone(),
                user_agent: user_agent.clone(),
            },
        )
        .await
        .map(|submission| {
            (
                StatusCode::CREATED,
                Json(SubmitFormResponse {
                    id: submission.id,
                    message: "Form submitted successfully".to_string(),
                    confirmation_message,
                }),
            )
        })
        .map_err(|e| {
            tracing::error!("Failed to submit form: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to submit form",
                )),
            )
        });
    rls.release().await;
    out
}

/// List submissions for a form (Story 54.4).
#[utoipa::path(
    get,
    path = "/api/v1/forms/{id}/submissions",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ListSubmissionsQuery
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of submissions", body = SubmissionListResponse),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn list_submissions(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
    Query(query): Query<ListSubmissionsQuery>,
) -> Result<Json<SubmissionListResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can view submissions",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let repo = &state.form_repo;

    let sub_query = SubmissionListQuery {
        form_id: Some(id),
        status: query.status,
        building_id: query.building_id,
        unit_id: query.unit_id,
        submitted_by: query.submitted_by,
        from_date: query.from_date,
        to_date: query.to_date,
        page: query.page,
        per_page: query.per_page,
    };

    let out = repo
        .list_submissions(rls.conn(), org_id, sub_query)
        .await
        .map(|(submissions, total)| {
            Json(SubmissionListResponse {
                submissions,
                total,
                page: query.page.unwrap_or(1),
                per_page: query.per_page.unwrap_or(20),
            })
        })
        .map_err(|e| {
            tracing::error!("Failed to list submissions: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to list submissions",
                )),
            )
        });
    rls.release().await;
    out
}

/// Get submission details.
#[utoipa::path(
    get,
    path = "/api/v1/forms/{id}/submissions/{submission_id}",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ("submission_id" = Uuid, Path, description = "Submission ID")
    ),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Submission details", body = SubmissionDetailResponse),
        (status = 404, description = "Submission not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn get_submission(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_id, submission_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<SubmissionDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let is_manager = rls.role().is_manager();
    let repo = &state.form_repo;

    let out = repo
        .get_submission(&mut **rls.conn(), org_id, submission_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get submission: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get submission",
                )),
            )
        })
        .and_then(|opt| {
            opt.ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Submission not found")),
                )
            })
        })
        .and_then(|submission| {
            // Non-managers can only view their own submissions
            if !is_manager && submission.submission.submitted_by != user_id {
                Err((
                    StatusCode::FORBIDDEN,
                    Json(ErrorResponse::new(
                        "FORBIDDEN",
                        "You can only view your own submissions",
                    )),
                ))
            } else {
                Ok(Json(SubmissionDetailResponse { submission }))
            }
        });
    rls.release().await;
    out
}

/// Review a submission (approve/reject).
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/submissions/{submission_id}/review",
    params(
        ("id" = Uuid, Path, description = "Form ID"),
        ("submission_id" = Uuid, Path, description = "Submission ID")
    ),
    request_body = ReviewSubmissionRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Submission reviewed", body = SubmissionDetailResponse),
        (status = 400, description = "Invalid review", body = ErrorResponse),
        (status = 404, description = "Submission not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn review_submission(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path((_id, submission_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<ReviewSubmissionRequest>,
) -> Result<Json<SubmissionDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    if !rls.role().is_manager() {
        rls.release().await;
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Only managers can review submissions",
            )),
        ));
    }

    // Validate status
    if req.status != "approved" && req.status != "rejected" {
        rls.release().await;
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(
                "BAD_REQUEST",
                "Status must be 'approved' or 'rejected'",
            )),
        ));
    }

    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let repo = &state.form_repo;

    let review_data = ReviewSubmission {
        status: req.status,
        review_notes: req.review_notes,
    };

    if let Err(e) = repo
        .review_submission(
            &mut **rls.conn(),
            org_id,
            submission_id,
            user_id,
            review_data,
        )
        .await
    {
        tracing::error!("Failed to review submission: {:?}", e);
        rls.release().await;
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Submission not found")),
        ));
    }

    // Get updated submission
    let out = repo
        .get_submission(&mut **rls.conn(), org_id, submission_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get submission: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "INTERNAL_ERROR",
                    "Failed to get submission",
                )),
            )
        })
        .and_then(|opt| {
            opt.ok_or_else(|| {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Submission not found")),
                )
            })
        })
        .map(|submission| Json(SubmissionDetailResponse { submission }));
    rls.release().await;
    out
}

/// Record form download (Story 54.2).
#[utoipa::path(
    post,
    path = "/api/v1/forms/{id}/download",
    params(("id" = Uuid, Path, description = "Form ID")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Download recorded"),
        (status = 404, description = "Form not found", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
    ),
    tag = "Forms"
)]
pub(super) async fn record_download(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let repo = &state.form_repo;

    let out = repo
        .record_download(&mut **rls.conn(), org_id, id, user_id, None, None)
        .await
        .and_then(|inserted| {
            if inserted {
                Ok(StatusCode::OK)
            } else {
                Err(sqlx::Error::RowNotFound)
            }
        })
        .map_err(|e| {
            if matches!(e, sqlx::Error::RowNotFound) {
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::new("NOT_FOUND", "Form not found")),
                )
            } else {
                tracing::error!("Failed to record download: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::new(
                        "INTERNAL_ERROR",
                        "Failed to record download",
                    )),
                )
            }
        });
    rls.release().await;
    out
}
