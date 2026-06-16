use api_core::extractors::RlsConnection;
use axum_extra::extract::Multipart;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use crate::state::AppState;
use db::models::accounting::{BankStatement, BankStatementLine};
use uuid::Uuid;

/// Upload a bank statement file (CSV).
pub async fn upload_statement(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<BankStatement>, StatusCode> {
    let mut filename = String::new();
    let mut data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            filename = field.file_name().unwrap_or_default().to_string();
            data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?.to_vec();
        }
    }

    if data.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let statement = state.accounting_service.process_statement_upload(rls.tenant_id(), filename, &data)
        .await
        .map_err(|e| {
            tracing::error!("Failed to process statement upload: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(statement))
}

/// List bank statements.
pub async fn list_statements(
    mut rls: RlsConnection,
    State(state): State<AppState>,
) -> Result<Json<Vec<BankStatement>>, StatusCode> {
    let statements = state.accounting_repo.list_bank_statements_rls(&mut **rls).await
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
    let lines = state.accounting_repo.list_bank_statement_lines_rls(&mut **rls, id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    rls.release().await;
    Ok(Json(lines))
}
