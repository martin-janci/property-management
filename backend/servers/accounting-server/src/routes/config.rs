//! Company & document configuration routes (EPIC-ACC-02 / UC-ACC-02). OWNED BY: BE-Config.
//!
//! Resource-server handlers: `AuthUser` + `RlsConnection`, RBAC via
//! `accounting_core::authz`, persistence via
//! `db::repositories::acc_config::AccConfigRepository`. Numbering allocation is
//! atomic in the repo (UC-ACC-02.3). `nest`ed under `/api/v1/config`.
//!
//! Each mutating handler persists an append-only audit entry
//! (UC-ACC-16.7 / 02.2 "change is dated/audited") via
//! `state.platform_repo.record_audit_rls(...)` in the SAME transaction as the
//! mutation, so a settings/series change is never recorded without committing,
//! nor committed without an audit row.
//!
//! Historical immutability (UC-ACC-02.1/.2/.9): editing settings, VAT rates, or
//! numbering here only affects FUTURE documents — issued documents carry their
//! own snapshot (taken by BE-Invoicing at issue time), so these handlers touch
//! no `invoice*` rows.

use accounting_core::audit::AuditBuilder;
use accounting_core::authz;
use api_core::extractors::{AuthUser, RlsConnection};
use axum::{extract::State, http::StatusCode, Json};
use chrono::Utc;
use db::models::acc_config::{
    AccBankAccount, AccCompanySettings, AccNumberingSeries, AccUnit, AccVatRate,
};
use serde::{Deserialize, Serialize};
use sqlx::Connection;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::state::AppState;

/// Upsert payload for company settings (UC-ACC-02.1/.2/.11).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpsertCompanySettingsRequest {
    pub legal_name: String,
    pub ico: Option<String>,
    pub dic: Option<String>,
    pub vat_id: Option<String>,
    pub address: Option<String>,
    pub tax_mode: String,
    pub vat_payer: bool,
    pub base_currency: String,
    pub rounding_mode: String,
    pub default_payment_terms_days: i32,
    pub default_language: String,
}

/// Create payload for a numbering series (UC-ACC-02.3).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateNumberingSeriesRequest {
    pub doc_type: String,
    pub year: i32,
    pub prefix: String,
    pub format: String,
}

// --- Domain validation (return 400 instead of letting a CHECK raise 500) ---

/// Valid `tax_mode` values per migration 00194 CHECK.
const VALID_TAX_MODES: [&str; 2] = ["tax_records", "accounting"];
/// Valid `rounding_mode` values per migration 00194 CHECK.
const VALID_ROUNDING_MODES: [&str; 3] = ["arithmetic", "bankers", "none"];
/// Valid `doc_type` values per migration 00194 CHECK.
const VALID_DOC_TYPES: [&str; 4] = ["invoice", "proforma", "advance", "credit_note"];

fn validate_company_settings(req: &UpsertCompanySettingsRequest) -> Result<(), StatusCode> {
    // UC-ACC-02.1: legal name is the minimum identity field for compliant docs.
    if req.legal_name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !VALID_TAX_MODES.contains(&req.tax_mode.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !VALID_ROUNDING_MODES.contains(&req.rounding_mode.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if req.base_currency.len() != 3 {
        return Err(StatusCode::BAD_REQUEST);
    }
    if req.default_payment_terms_days < 0 || req.default_payment_terms_days > 3650 {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

fn validate_numbering_series(req: &CreateNumberingSeriesRequest) -> Result<(), StatusCode> {
    if !VALID_DOC_TYPES.contains(&req.doc_type.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    // UC-ACC-02.3: sane year window for the reset cadence.
    if req.year < 2000 || req.year > 2200 {
        return Err(StatusCode::BAD_REQUEST);
    }
    // UC-ACC-02.3: reject a malformed format template at creation time by
    // probing the pure formatter — better than discovering it at issue time.
    let probe = accounting_core::numbering::NumberParts {
        prefix: &req.prefix,
        year: req.year,
        seq: 1,
    };
    accounting_core::numbering::format_number(&req.format, &probe)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    Ok(())
}

/// Get the company settings for the active company (UC-ACC-02.1).
#[utoipa::path(
    get,
    path = "/api/v1/config/company",
    responses(
        (status = 200, description = "Company settings", body = AccCompanySettings),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not configured yet")
    ),
    tag = "Config"
)]
pub async fn get_company_settings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<AccCompanySettings>, StatusCode> {
    // UC-ACC-16.2: read gate is enforced server-side regardless of UI state.
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let settings = state
        .config_repo
        .get_company_settings_rls(&mut **rls)
        .await
        .map_err(|e| {
            tracing::error!("Failed to load company settings: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    rls.release().await;
    Ok(Json(settings))
}

/// Upsert the company settings (UC-ACC-02.1/.2/.11).
#[utoipa::path(
    put,
    path = "/api/v1/config/company",
    request_body = UpsertCompanySettingsRequest,
    responses(
        (status = 200, description = "Saved settings", body = AccCompanySettings),
        (status = 400, description = "Bad request (validation)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    tag = "Config"
)]
pub async fn upsert_company_settings(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(req): Json<UpsertCompanySettingsRequest>,
) -> Result<Json<AccCompanySettings>, StatusCode> {
    // UC-ACC-16.2: only manager-level+ may change company config.
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;
    validate_company_settings(&req)?;

    let tenant_id = rls.tenant_id();
    let now = Utc::now();

    // Build the row to persist. id/created_at/updated_at are placeholders the DB
    // overrides via defaults/trigger/RETURNING. Fields not exposed by the DTO
    // (email/phone/web/logo/brand/notes/footer) are preserved as None here; they
    // have their own (future) endpoints — see contract-notes/be-config.md.
    let settings = AccCompanySettings {
        id: Uuid::nil(),
        tenant_id,
        legal_name: req.legal_name.clone(),
        ico: req.ico.clone(),
        dic: req.dic.clone(),
        vat_id: req.vat_id.clone(),
        address: req.address.clone(),
        email: None,
        phone: None,
        web: None,
        logo_url: None,
        brand_color: None,
        tax_mode: req.tax_mode.clone(),
        vat_payer: req.vat_payer,
        base_currency: req.base_currency.clone(),
        rounding_mode: req.rounding_mode.clone(),
        default_payment_terms_days: req.default_payment_terms_days,
        default_invoice_note: None,
        default_footer: None,
        default_language: req.default_language.clone(),
        created_at: now,
        updated_at: now,
    };

    // Mutation + audit in one transaction (UC-ACC-02.2 "change is dated/audited";
    // UC-ACC-16.7 append-only): both commit together or neither does.
    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start tx for company settings: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let saved = state
        .config_repo
        .upsert_company_settings_rls(&mut *tx, tenant_id, settings)
        .await
        .map_err(|e| {
            tracing::error!("Failed to upsert company settings: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let audit = AuditBuilder::new(tenant_id, Some(auth.user_id), "update", "company_settings")
        .entity_id(saved.id)
        .diff(serde_json::json!({
            "tax_mode": saved.tax_mode,
            "vat_payer": saved.vat_payer,
            "base_currency": saved.base_currency,
            "rounding_mode": saved.rounding_mode,
        }))
        .build();
    state
        .platform_repo
        .record_audit_rls(&mut *tx, audit)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit for company settings: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit company settings tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok(Json(saved))
}

/// List numbering series (UC-ACC-02.3).
#[utoipa::path(
    get,
    path = "/api/v1/config/numbering",
    responses(
        (status = 200, description = "Numbering series", body = [AccNumberingSeries]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Config"
)]
pub async fn list_numbering_series(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccNumberingSeries>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let series = state
        .config_repo
        .list_numbering_series_rls(&mut **rls)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list numbering series: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(series))
}

/// Create a numbering series (UC-ACC-02.3).
#[utoipa::path(
    post,
    path = "/api/v1/config/numbering",
    request_body = CreateNumberingSeriesRequest,
    responses(
        (status = 201, description = "Created series", body = AccNumberingSeries),
        (status = 400, description = "Bad request (validation)"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 409, description = "Series already exists for doc_type/year")
    ),
    tag = "Config"
)]
pub async fn create_numbering_series(
    State(state): State<AppState>,
    mut rls: RlsConnection,
    auth: AuthUser,
    Json(req): Json<CreateNumberingSeriesRequest>,
) -> Result<(StatusCode, Json<AccNumberingSeries>), StatusCode> {
    authz::require_role(rls.role(), authz::MUTATE_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;
    validate_numbering_series(&req)?;

    let tenant_id = rls.tenant_id();
    let now = Utc::now();

    // A new series always starts at next_value = 1 (gapless from the first
    // issuance, UC-ACC-02.3). The unique (tenant_id, doc_type, year) key gives
    // the yearly reset for free — a fresh row per year restarts the sequence.
    let series = AccNumberingSeries {
        id: Uuid::nil(),
        tenant_id,
        doc_type: req.doc_type.clone(),
        year: req.year,
        prefix: req.prefix.clone(),
        format: req.format.clone(),
        next_value: 1,
        created_at: now,
        updated_at: now,
    };

    let mut tx = rls.begin().await.map_err(|e| {
        tracing::error!("Failed to start tx for numbering series: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let created = state
        .config_repo
        .create_numbering_series_rls(&mut *tx, series)
        .await
        .map_err(|e| {
            // The unique (tenant_id, doc_type, year) collision is a client error.
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.is_unique_violation() {
                    return StatusCode::CONFLICT;
                }
            }
            tracing::error!("Failed to create numbering series: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let audit = AuditBuilder::new(tenant_id, Some(auth.user_id), "create", "numbering_series")
        .entity_id(created.id)
        .diff(serde_json::json!({
            "doc_type": created.doc_type,
            "year": created.year,
            "prefix": created.prefix,
            "format": created.format,
        }))
        .build();
    state
        .platform_repo
        .record_audit_rls(&mut *tx, audit)
        .await
        .map_err(|e| {
            tracing::error!("Failed to record audit for numbering series: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit numbering series tx: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    rls.release().await;
    Ok((StatusCode::CREATED, Json(created)))
}

/// List units (UC-ACC-02.8).
#[utoipa::path(
    get,
    path = "/api/v1/config/units",
    responses(
        (status = 200, description = "Units", body = [AccUnit]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Config"
)]
pub async fn list_units(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccUnit>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let units = state
        .config_repo
        .list_units_rls(&mut **rls)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list units: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(units))
}

/// List VAT rates (UC-ACC-02.9).
#[utoipa::path(
    get,
    path = "/api/v1/config/vat-rates",
    responses(
        (status = 200, description = "VAT rates", body = [AccVatRate]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Config"
)]
pub async fn list_vat_rates(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccVatRate>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let rates = state
        .config_repo
        .list_vat_rates_rls(&mut **rls)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list VAT rates: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(rates))
}

/// List bank accounts (UC-ACC-02.10).
#[utoipa::path(
    get,
    path = "/api/v1/config/bank-accounts",
    responses(
        (status = 200, description = "Bank accounts", body = [AccBankAccount]),
        (status = 401, description = "Unauthorized")
    ),
    tag = "Config"
)]
pub async fn list_bank_accounts(
    State(state): State<AppState>,
    mut rls: RlsConnection,
) -> Result<Json<Vec<AccBankAccount>>, StatusCode> {
    authz::require_role(rls.role(), authz::READ_MIN_ROLE).map_err(|_| StatusCode::FORBIDDEN)?;

    let accounts = state
        .config_repo
        .list_bank_accounts_rls(&mut **rls)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list bank accounts: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    rls.release().await;
    Ok(Json(accounts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_settings() -> UpsertCompanySettingsRequest {
        UpsertCompanySettingsRequest {
            legal_name: "Acme s.r.o.".to_string(),
            ico: Some("12345678".to_string()),
            dic: None,
            vat_id: None,
            address: Some("Main St 1, Bratislava".to_string()),
            tax_mode: "tax_records".to_string(),
            vat_payer: false,
            base_currency: "EUR".to_string(),
            rounding_mode: "arithmetic".to_string(),
            default_payment_terms_days: 14,
            default_language: "sk".to_string(),
        }
    }

    #[test]
    fn valid_company_settings_pass() {
        assert!(validate_company_settings(&ok_settings()).is_ok());
    }

    #[test]
    fn empty_legal_name_rejected() {
        // UC-ACC-02.1 mandatory-field guard.
        let mut r = ok_settings();
        r.legal_name = "   ".to_string();
        assert_eq!(validate_company_settings(&r), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn bad_tax_mode_rejected() {
        let mut r = ok_settings();
        r.tax_mode = "double_entry".to_string();
        assert_eq!(validate_company_settings(&r), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn bad_rounding_mode_rejected() {
        let mut r = ok_settings();
        r.rounding_mode = "round_up".to_string();
        assert_eq!(validate_company_settings(&r), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn bad_currency_length_rejected() {
        let mut r = ok_settings();
        r.base_currency = "EU".to_string();
        assert_eq!(validate_company_settings(&r), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn both_tax_modes_accepted() {
        let mut r = ok_settings();
        r.tax_mode = "accounting".to_string();
        assert!(validate_company_settings(&r).is_ok());
    }

    fn ok_series() -> CreateNumberingSeriesRequest {
        CreateNumberingSeriesRequest {
            doc_type: "invoice".to_string(),
            year: 2026,
            prefix: "INV".to_string(),
            format: "{prefix}{year}{seq:04}".to_string(),
        }
    }

    #[test]
    fn valid_series_pass() {
        assert!(validate_numbering_series(&ok_series()).is_ok());
    }

    #[test]
    fn bad_doc_type_rejected() {
        let mut r = ok_series();
        r.doc_type = "receipt".to_string();
        assert_eq!(validate_numbering_series(&r), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn malformed_format_rejected() {
        // UC-ACC-02.3: probe rejects a malformed template at creation time.
        let mut r = ok_series();
        r.format = "{prefix}{seq:04".to_string();
        assert_eq!(validate_numbering_series(&r), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn out_of_range_year_rejected() {
        let mut r = ok_series();
        r.year = 1900;
        assert_eq!(validate_numbering_series(&r), Err(StatusCode::BAD_REQUEST));
    }

    #[test]
    fn all_doc_types_accepted() {
        for dt in ["invoice", "proforma", "advance", "credit_note"] {
            let mut r = ok_series();
            r.doc_type = dt.to_string();
            assert!(
                validate_numbering_series(&r).is_ok(),
                "doc_type {dt} should pass"
            );
        }
    }
}
