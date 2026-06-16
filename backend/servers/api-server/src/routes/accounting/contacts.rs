//! Accounting contact routes for MVP.

use api_core::extractors::{AuthUser, RlsConnection};
use axum::{extract::State, http::StatusCode, Json};
use db::models::accounting::Contact;

use crate::state::AppState;

/// List all contacts for the current tenant.
#[utoipa::path(
    get,
    path = "/api/v1/accounting/contacts",
    responses(
        (status = 200, description = "List of contacts", body = [Contact]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "accounting"
)]
pub async fn list_contacts(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    AuthUser { user_id: _user, .. }: AuthUser,
) -> Result<Json<Vec<Contact>>, StatusCode> {
    let contacts = state
        .accounting_repo
        .list_contacts_rls(&mut **rls)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list contacts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(contacts))
}
