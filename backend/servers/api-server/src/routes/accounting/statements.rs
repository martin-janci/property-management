use crate::state::AppState;
use api_core::extractors::RlsConnection;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use axum_extra::extract::Multipart;
use common::TenantRole;
use db::models::accounting::{BankStatement, BankStatementLine};
use uuid::Uuid;

const MAX_UPLOAD_SIZE: usize = 5 * 1024 * 1024; // 5MB

/// Upload a bank statement file (CSV).
pub async fn upload_statement(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<BankStatement>, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut filename = String::new();
    let mut data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            filename = field.file_name().unwrap_or_default().to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            if bytes.len() > MAX_UPLOAD_SIZE {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }

            data = bytes.to_vec();
        }
    }

    if data.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let tenant_id = rls.tenant_id();

    let statement = state
        .accounting_service
        .process_statement_upload(&mut **rls, tenant_id, filename, &data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to process statement upload: {}", e);
            match e {
                common::errors::AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    rls.release().await;
    Ok(Json(statement))
}

/// List bank statements.
pub async fn list_statements(
    mut rls: RlsConnection,
    State(state): State<AppState>,
) -> Result<Json<Vec<BankStatement>>, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let statements = state
        .accounting_repo
        .list_bank_statements_rls(&mut **rls)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    rls.release().await;
    Ok(Json(statements))
}

/// List bank statement lines for a specific statement.
pub async fn list_statement_lines(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<BankStatementLine>>, StatusCode> {
    // Role check: Accounting is manager-only
    if !rls.is_super_admin() && !rls.has_role(TenantRole::Manager) {
        return Err(StatusCode::FORBIDDEN);
    }

    let lines = state
        .accounting_repo
        .list_bank_statement_lines_rls(&mut **rls, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    rls.release().await;
    Ok(Json(lines))
}
