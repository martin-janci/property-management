//! Form routes (Epic 54: Forms Management).
//!
//! Provides endpoints for form templates, fields, and submissions.
//!
//! # RLS (PAP-76)
//!
//! Migration `00179` put `FORCE ROW LEVEL SECURITY` on the form tables so
//! every query MUST run on a connection that has `app.current_org_id` set.
//! Each handler acquires an [`RlsConnection`] (authenticates the caller,
//! validates tenant membership, sets org/user GUCs on a dedicated connection)
//! and passes `&mut **rls.conn()` to the repository. `rls.release()` clears
//! the context before the connection returns to the pool.
//!
//! # Module layout
//!
//! The route handlers are split into cohesive submodules to keep each file
//! focused and reduce churn surface:
//! - [`types`] — request/response/query types, constants, validation helpers.
//! - [`crud`] — form CRUD plus publish/archive/statistics handlers.
//! - [`fields`] — form field handlers.
//! - [`submissions`] — form submission + download-tracking handlers.

mod crud;
mod fields;
mod submissions;
mod types;

// Re-export the public request/response/query types so the module's public
// API surface (`routes::forms::CreateFormRequest`, etc.) is unchanged by the
// split.
pub use types::{
    AttachmentRequest, AvailableFormsResponse, CreateFormFieldRequest, CreateFormRequest,
    FieldActionResponse, FieldOptionRequest, FieldOrderItem, FieldsResponse, FormActionResponse,
    FormDetailResponse, FormStatisticsResponse, ListFormsQuery, ListSubmissionsQuery,
    ReorderFieldsRequest, ReviewSubmissionRequest, SignatureDataRequest, SubmissionDetailResponse,
    SubmitFormRequest, UpdateFormFieldRequest, UpdateFormRequest,
};

use crate::state::AppState;
use axum::{
    routing::{delete, get, post, put},
    Router,
};

/// Create forms router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Form CRUD
        .route("/", post(crud::create_form))
        .route("/", get(crud::list_forms))
        .route("/available", get(crud::list_available_forms))
        .route("/statistics", get(crud::get_statistics))
        .route("/{id}", get(crud::get_form))
        .route("/{id}", put(crud::update_form))
        .route("/{id}", delete(crud::delete_form))
        // Publishing
        .route("/{id}/publish", post(crud::publish_form))
        .route("/{id}/archive", post(crud::archive_form))
        // Fields
        .route("/{id}/fields", get(fields::list_fields))
        .route("/{id}/fields", post(fields::add_field))
        .route("/{id}/fields/reorder", post(fields::reorder_fields))
        .route("/{id}/fields/{field_id}", put(fields::update_field))
        .route("/{id}/fields/{field_id}", delete(fields::delete_field))
        // Submissions
        .route("/{id}/submit", post(submissions::submit_form))
        .route("/{id}/submissions", get(submissions::list_submissions))
        .route(
            "/{id}/submissions/{submission_id}",
            get(submissions::get_submission),
        )
        .route(
            "/{id}/submissions/{submission_id}/review",
            post(submissions::review_submission),
        )
        // Download tracking
        .route("/{id}/download", post(submissions::record_download))
}
