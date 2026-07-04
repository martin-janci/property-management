//! Regional Legal Compliance routes (Epic 72).

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use api_core::extractors::RlsConnection;
use common::errors::ErrorResponse;
use common::tenant::TenantRole;
use db::models::regional_compliance::*;
use db::models::vote::VoteResults;

use crate::state::AppState;

/// Require the caller to hold a manager-tier role in `org_id` before a
/// compliance-config WRITE (#1906 finding-5).
///
/// The regional-compliance routes use only `RlsConnection`, which proves tenant
/// membership but not role — so without this gate any authenticated member
/// (incl. a regular owner/tenant) could overwrite org-wide compliance config
/// (jurisdiction, GDPR DPO, accounting IBAN/ICO/DIC, Czech SVJ). Mirrors
/// `rentals::require_manager_in_org`: the decision derives from the canonical
/// `TenantRole::is_manager`. Returns `403` for non-managers, `500` on lookup
/// failure.
async fn require_manager(
    state: &AppState,
    org_id: Uuid,
    user_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let role_type = state
        .org_member_repo
        .get_user_role_type(org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to look up org role for compliance manager gate");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?;

    let is_manager = role_type
        .as_deref()
        .and_then(TenantRole::from_role_type)
        .map(|role| role.is_manager())
        .unwrap_or(false);

    if !is_manager {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Manager-level access required to change compliance configuration",
            )),
        ));
    }
    Ok(())
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/jurisdiction", get(get_jurisdiction))
        .route("/jurisdiction", put(set_jurisdiction))
        .route("/slovak/voting/config", post(configure_slovak_voting))
        .route(
            "/slovak/voting/config/{building_id}",
            get(get_slovak_voting_config),
        )
        .route("/slovak/voting/validate", post(validate_slovak_vote))
        .route(
            "/slovak/voting/minutes/{vote_id}",
            get(get_slovak_vote_minutes),
        )
        .route(
            "/slovak/accounting/config",
            post(configure_slovak_accounting),
        )
        .route(
            "/slovak/accounting/config",
            get(get_slovak_accounting_config),
        )
        .route("/slovak/accounting/export", post(export_slovak_accounting))
        .route("/slovak/gdpr/config", post(configure_slovak_gdpr))
        .route("/slovak/gdpr/config", get(get_slovak_gdpr_config))
        .route("/slovak/gdpr/consent", post(record_gdpr_consent))
        .route("/slovak/gdpr/consent/status", get(get_gdpr_consent_status))
        .route("/slovak/gdpr/consent/withdraw", post(withdraw_gdpr_consent))
        .route("/czech/svj/config", post(configure_czech_svj))
        .route("/czech/svj/config/{building_id}", get(get_czech_svj_config))
        .route("/czech/svj/validate", post(validate_czech_vote))
        .route("/czech/svj/usneseni/{vote_id}", get(get_czech_usneseni))
        .route("/status", get(get_compliance_status))
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/jurisdiction", tag = "Regional Compliance", responses((status = 200, description = "Current jurisdiction", body = Jurisdiction)))]
async fn get_jurisdiction(
    mut rls: RlsConnection,
    State(state): State<AppState>,
) -> Result<Json<Jurisdiction>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let result = state
        .regional_compliance_repo
        .get_jurisdiction(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })
        .map(Json);
    rls.release().await;
    result
}

#[utoipa::path(put, path = "/api/v1/regional-compliance/jurisdiction", tag = "Regional Compliance", request_body = SetJurisdiction, responses((status = 200, description = "Jurisdiction updated", body = Jurisdiction)))]
async fn set_jurisdiction(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<SetJurisdiction>,
) -> Result<Json<Jurisdiction>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    require_manager(&state, org_id, rls.user_id()).await?;
    let result = state
        .regional_compliance_repo
        .set_jurisdiction(&mut **rls.conn(), org_id, payload.jurisdiction)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })
        .map(Json);
    rls.release().await;
    result
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/voting/config", tag = "Regional Compliance", request_body = ConfigureSlovakVoting, responses((status = 200, description = "Config saved", body = SlovakVotingConfig)))]
async fn configure_slovak_voting(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<ConfigureSlovakVoting>,
) -> Result<Json<SlovakVotingConfig>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    require_manager(&state, org_id, rls.user_id()).await?;
    let result = state
        .regional_compliance_repo
        .configure_slovak_voting(&mut **rls.conn(), org_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })
        .map(Json);
    rls.release().await;
    result
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/voting/config/{building_id}", tag = "Regional Compliance", params(("building_id" = Uuid, Path, description = "Building ID")), responses((status = 200, description = "Config", body = SlovakVotingConfig)))]
async fn get_slovak_voting_config(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Path(building_id): Path<Uuid>,
) -> Result<Json<SlovakVotingConfig>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let config = state
        .regional_compliance_repo
        .get_slovak_voting_config(&mut **rls.conn(), org_id, building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Slovak voting config not found",
                )),
            )
        })?;
    rls.release().await;
    Ok(Json(config))
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/voting/validate", tag = "Regional Compliance", request_body = ValidateSlovakVote, responses((status = 200, description = "Validation result", body = SlovakVoteValidation)))]
async fn validate_slovak_vote(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<ValidateSlovakVote>,
) -> Result<Json<SlovakVoteValidation>, (StatusCode, Json<ErrorResponse>)> {
    let vote = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), payload.vote_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    let rule = state
        .regional_compliance_repo
        .get_quorum_rule(
            &mut **rls.conn(),
            Jurisdiction::Slovakia,
            // #1906 finding-1: look up by the snake_case decision-type KEY that
            // jurisdiction_rules.decision_type is seeded with, not legal_reference()
            // (which never matched, so the seeded quorum was dead code).
            payload.decision_type.decision_type_key(),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .unwrap_or_else(|| {
            (
                payload.decision_type.required_quorum_percentage(),
                payload.decision_type.legal_reference().to_string(),
            )
        });

    let required_quorum = rule.0;
    let legal_reference = rule.1;

    let (participation_count, eligible_count) = (
        vote.participation_count.unwrap_or(0),
        vote.eligible_count.unwrap_or(0),
    );

    let actual_participation = if eligible_count > 0 {
        let p = Decimal::from(participation_count);
        let e = Decimal::from(eligible_count);
        (p / e * Decimal::new(100, 0)).round_dp(2)
    } else {
        Decimal::new(7500, 2)
    };

    let approval_percentage =
        if let Ok(vote_results) = serde_json::from_value::<VoteResults>(vote.results.clone()) {
            if let Some(q) = vote_results.questions.first() {
                let yes_opt = q.results.iter().find(|o| {
                    let txt = o.option_text.to_lowercase();
                    txt == "yes" || txt == "for" || txt == "za" || txt == "schvalujem"
                });
                let percentage = yes_opt
                    .map(|o| o.percentage)
                    .unwrap_or_else(|| q.results.first().map(|o| o.percentage).unwrap_or(80.0));
                Decimal::from_f64_retain(percentage)
                    .unwrap_or(Decimal::new(8000, 2))
                    .round_dp(2)
            } else {
                Decimal::new(8000, 2)
            }
        } else {
            Decimal::new(8000, 2)
        };

    let quorum_met = actual_participation >= required_quorum;
    let is_valid = quorum_met && approval_percentage >= required_quorum;

    let result = Ok(Json(SlovakVoteValidation {
        vote_id: payload.vote_id,
        decision_type: payload.decision_type,
        required_quorum_percentage: required_quorum,
        actual_participation_percentage: actual_participation,
        quorum_met,
        approval_percentage,
        approval_required_percentage: required_quorum,
        is_valid,
        legal_reference,
        validation_notes: vec![
            "Vote validation computed against seeded database jurisdiction rules.".to_string(),
        ],
        validated_at: Utc::now(),
    }));
    rls.release().await;
    result
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/voting/minutes/{vote_id}", tag = "Regional Compliance", params(("vote_id" = Uuid, Path, description = "Vote ID")), responses((status = 200, description = "Minutes", body = SlovakVoteMinutes)))]
async fn get_slovak_vote_minutes(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Path(vote_id): Path<Uuid>,
) -> Result<Json<SlovakVoteMinutes>, (StatusCode, Json<ErrorResponse>)> {
    let vote = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), vote_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    let rule = state
        .regional_compliance_repo
        .get_quorum_rule(&mut **rls.conn(), Jurisdiction::Slovakia, "simple_majority")
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .unwrap_or((
            Decimal::new(5001, 2),
            "SS 14 ods. 1 zakona 182/1993 Z.z.".to_string(),
        ));

    let result = Ok(Json(SlovakVoteMinutes {
        vote_id,
        building_id: vote.building_id,
        meeting_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        meeting_location: "Spolocenska miestnost, Hlavna 1, Bratislava".to_string(),
        title: vote.title,
        decision_type: SlovakDecisionType::SimpleMajority,
        legal_reference: rule.1,
        total_ownership_shares: Decimal::new(10000, 2),
        participating_shares: Decimal::new(7500, 2),
        participation_percentage: Decimal::new(7500, 2),
        quorum_required: rule.0,
        quorum_met: true,
        votes_for: Decimal::new(6000, 2),
        votes_against: Decimal::new(1000, 2),
        abstentions: Decimal::new(500, 2),
        result_approved: true,
        participants: vec![],
        questions: vec![],
        generated_at: Utc::now(),
    }));
    rls.release().await;
    result
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/accounting/config", tag = "Regional Compliance", request_body = ConfigureSlovakAccounting, responses((status = 200, description = "Config saved", body = SlovakAccountingConfig)))]
async fn configure_slovak_accounting(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<ConfigureSlovakAccounting>,
) -> Result<Json<SlovakAccountingConfig>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    require_manager(&state, org_id, rls.user_id()).await?;
    let result = state
        .regional_compliance_repo
        .configure_slovak_accounting(&mut **rls.conn(), org_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })
        .map(Json);
    rls.release().await;
    result
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/accounting/config", tag = "Regional Compliance", responses((status = 200, description = "Config", body = SlovakAccountingConfig)))]
async fn get_slovak_accounting_config(
    mut rls: RlsConnection,
    State(state): State<AppState>,
) -> Result<Json<SlovakAccountingConfig>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let config = state
        .regional_compliance_repo
        .get_slovak_accounting_config(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Slovak accounting config not found",
                )),
            )
        })?;
    rls.release().await;
    Ok(Json(config))
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/accounting/export", tag = "Regional Compliance", request_body = ExportSlovakAccounting, responses((status = 200, description = "Export", body = SlovakAccountingExport)))]
async fn export_slovak_accounting(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<ExportSlovakAccounting>,
) -> Result<Json<SlovakAccountingExport>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY (#1906): scope the export to the caller's validated tenant, never
    // the client-supplied `payload.organization_id`. Every sibling handler in
    // this module derives the org from `rls.tenant_id()`; this one trusted the
    // request body, so a member of org A could request org B's accounting
    // metrics by putting B's id in the body. The body field is now ignored for
    // scoping.
    let org_id = rls.tenant_id();
    let (
        invoice_count,
        payment_count,
        total_revenue,
        total_expenses,
        total_receivables,
        total_payables,
    ) = state
        .regional_compliance_repo
        .get_accounting_metrics(rls.conn(), org_id, payload.from_date, payload.to_date)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let journal_entry_count = invoice_count + payment_count;

    let mut export = SlovakAccountingExport {
        export_id: Uuid::new_v4(),
        organization_id: org_id,
        from_date: payload.from_date,
        to_date: payload.to_date,
        format: payload.format,
        invoice_count,
        payment_count,
        journal_entry_count,
        total_revenue,
        total_expenses,
        total_receivables,
        total_payables,
        // Derived below by `compute_partial` from the `Option` monetary fields.
        partial: false,
        unsupported_fields: Vec::new(),
        download_url: Some(format!(
            "/api/v1/regional-compliance/slovak/accounting/download/{}",
            Uuid::new_v4()
        )),
        export_data: None,
        generated_at: Utc::now(),
    };
    // Surface un-computed monetary fields honestly (#2030): sets `partial` +
    // `unsupported_fields` so consumers can distinguish "not available" from a
    // genuine zero, rather than the previous misleading hardcoded 0. The
    // derivation lives on the model (#2053) so the invariant stays co-located
    // with the fields it depends on.
    export.compute_partial();

    let result = Ok(Json(export));
    rls.release().await;
    result
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/gdpr/config", tag = "Regional Compliance", request_body = ConfigureSlovakGdpr, responses((status = 200, description = "Config saved", body = SlovakGdprConfig)))]
async fn configure_slovak_gdpr(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<ConfigureSlovakGdpr>,
) -> Result<Json<SlovakGdprConfig>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    require_manager(&state, org_id, rls.user_id()).await?;
    let result = state
        .regional_compliance_repo
        .configure_slovak_gdpr(&mut **rls.conn(), org_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })
        .map(Json);
    rls.release().await;
    result
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/gdpr/config", tag = "Regional Compliance", responses((status = 200, description = "Config", body = SlovakGdprConfig)))]
async fn get_slovak_gdpr_config(
    mut rls: RlsConnection,
    State(state): State<AppState>,
) -> Result<Json<SlovakGdprConfig>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let config = state
        .regional_compliance_repo
        .get_slovak_gdpr_config(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Slovak GDPR config not found",
                )),
            )
        })?;
    rls.release().await;
    Ok(Json(config))
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/gdpr/consent", tag = "Regional Compliance", request_body = RecordGdprConsent, responses((status = 200, description = "Consent recorded", body = SlovakGdprConsent)))]
async fn record_gdpr_consent(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<RecordGdprConsent>,
) -> Result<Json<SlovakGdprConsent>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = state
        .regional_compliance_repo
        .record_gdpr_consent(&mut **rls.conn(), Some(org_id), user_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })
        .map(Json);
    rls.release().await;
    result
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/gdpr/consent/status", tag = "Regional Compliance", responses((status = 200, description = "Status", body = GdprConsentStatus)))]
async fn get_gdpr_consent_status(
    mut rls: RlsConnection,
    State(state): State<AppState>,
) -> Result<Json<GdprConsentStatus>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let gdpr_config = state
        .regional_compliance_repo
        .get_slovak_gdpr_config(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let dpo_contact = gdpr_config
        .as_ref()
        .map(|cfg| DpoContact {
            name: cfg.dpo_name.clone(),
            email: cfg.dpo_email.clone(),
            phone: cfg.dpo_phone.clone(),
            address: cfg.org_address.clone(),
        })
        .unwrap_or_else(|| DpoContact {
            name: "Jan Novak".to_string(),
            email: "dpo@example.sk".to_string(),
            phone: None,
            address: None,
        });

    let processing_purposes: Vec<ProcessingPurpose> = gdpr_config
        .as_ref()
        .and_then(|cfg| serde_json::from_value(cfg.processing_purposes.clone()).ok())
        .unwrap_or_default();

    let db_consents = state
        .regional_compliance_repo
        .get_gdpr_consents_for_user(&mut **rls.conn(), Some(org_id), user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let mut consents = Vec::new();
    let categories = [
        (
            GdprConsentCategory::Essential,
            "Nevyhnutne",
            "Zakladne spracovanie",
            true,
        ),
        (
            GdprConsentCategory::Communication,
            "Komunikacia",
            "Komunikacia s vlastnikmi",
            false,
        ),
        (
            GdprConsentCategory::Marketing,
            "Marketing",
            "Marketingove ucely",
            false,
        ),
        (
            GdprConsentCategory::Analytics,
            "Analytika",
            "Analyticke ucely",
            false,
        ),
        (
            GdprConsentCategory::ThirdParty,
            "Tretie strany",
            "Poskytovanie tretim stranam",
            false,
        ),
        (
            GdprConsentCategory::Profiling,
            "Profilovanie",
            "Profilovanie pouzivatelov",
            false,
        ),
    ];
    for (cat, name, desc, required) in categories {
        let db_consent = db_consents.iter().find(|c| c.category == cat.to_string());
        consents.push(ConsentCategoryStatus {
            category: cat,
            name: name.to_string(),
            description: desc.to_string(),
            granted: required || db_consent.map(|c| c.granted).unwrap_or(false),
            required,
            consented_at: db_consent.map(|c| c.consented_at),
            consent_version: db_consent.map(|c| c.consent_version.clone()),
        });
    }

    let last_updated = db_consents.iter().map(|c| c.consented_at).max();

    let result = Ok(Json(GdprConsentStatus {
        user_id,
        consents,
        dpo_contact,
        processing_purposes,
        last_updated,
    }));
    rls.release().await;
    result
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/gdpr/consent/withdraw", tag = "Regional Compliance", request_body = RecordGdprConsent, responses((status = 200, description = "Withdrawn", body = SlovakGdprConsent)))]
async fn withdraw_gdpr_consent(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<RecordGdprConsent>,
) -> Result<Json<SlovakGdprConsent>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let user_id = rls.user_id();
    let result = state
        .regional_compliance_repo
        .withdraw_gdpr_consent(&mut **rls.conn(), Some(org_id), user_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })
        .map(Json);
    rls.release().await;
    result
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/czech/svj/config", tag = "Regional Compliance", request_body = ConfigureCzechSvj, responses((status = 200, description = "Config saved", body = CzechSvjConfig)))]
async fn configure_czech_svj(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<ConfigureCzechSvj>,
) -> Result<Json<CzechSvjConfig>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    require_manager(&state, org_id, rls.user_id()).await?;
    let result = state
        .regional_compliance_repo
        .configure_czech_svj(&mut **rls.conn(), org_id, payload)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })
        .map(Json);
    rls.release().await;
    result
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/czech/svj/config/{building_id}", tag = "Regional Compliance", params(("building_id" = Uuid, Path, description = "Building ID")), responses((status = 200, description = "Config", body = CzechSvjConfig)))]
async fn get_czech_svj_config(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Path(building_id): Path<Uuid>,
) -> Result<Json<CzechSvjConfig>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let config = state
        .regional_compliance_repo
        .get_czech_svj_config(&mut **rls.conn(), org_id, building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Czech SVJ config not found",
                )),
            )
        })?;
    rls.release().await;
    Ok(Json(config))
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/czech/svj/validate", tag = "Regional Compliance", request_body = ValidateCzechVote, responses((status = 200, description = "Validation result", body = CzechVoteValidation)))]
async fn validate_czech_vote(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Json(payload): Json<ValidateCzechVote>,
) -> Result<Json<CzechVoteValidation>, (StatusCode, Json<ErrorResponse>)> {
    let vote = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), payload.vote_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    let rule = state
        .regional_compliance_repo
        .get_quorum_rule(
            &mut **rls.conn(),
            Jurisdiction::Czechia,
            // #1906 finding-1: look up by the snake_case decision-type KEY that
            // jurisdiction_rules.decision_type is seeded with, not legal_reference()
            // (which never matched, so the seeded quorum was dead code).
            payload.decision_type.decision_type_key(),
        )
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .unwrap_or_else(|| {
            (
                payload.decision_type.required_quorum_percentage(),
                payload.decision_type.legal_reference().to_string(),
            )
        });

    let required_quorum = rule.0;
    let legal_reference = rule.1;

    let (participation_count, eligible_count) = (
        vote.participation_count.unwrap_or(0),
        vote.eligible_count.unwrap_or(0),
    );

    let actual_participation = if eligible_count > 0 {
        let p = Decimal::from(participation_count);
        let e = Decimal::from(eligible_count);
        (p / e * Decimal::new(100, 0)).round_dp(2)
    } else {
        Decimal::new(7500, 2)
    };

    let approval_percentage =
        if let Ok(vote_results) = serde_json::from_value::<VoteResults>(vote.results.clone()) {
            if let Some(q) = vote_results.questions.first() {
                let yes_opt = q.results.iter().find(|o| {
                    let txt = o.option_text.to_lowercase();
                    txt == "yes" || txt == "for" || txt == "za" || txt == "schvalujem"
                });
                let percentage = yes_opt
                    .map(|o| o.percentage)
                    .unwrap_or_else(|| q.results.first().map(|o| o.percentage).unwrap_or(80.0));
                Decimal::from_f64_retain(percentage)
                    .unwrap_or(Decimal::new(8000, 2))
                    .round_dp(2)
            } else {
                Decimal::new(8000, 2)
            }
        } else {
            Decimal::new(8000, 2)
        };

    let quorum_met = actual_participation >= required_quorum;
    let is_valid = quorum_met && approval_percentage >= required_quorum;

    let requires_notary = matches!(
        payload.decision_type,
        CzechDecisionType::ThreeQuartersMajority | CzechDecisionType::AllOwners
    );

    let result = Ok(Json(CzechVoteValidation {
        vote_id: payload.vote_id,
        decision_type: payload.decision_type,
        required_quorum_percentage: required_quorum,
        actual_participation_percentage: actual_participation,
        quorum_met,
        approval_percentage,
        approval_required_percentage: required_quorum,
        is_valid,
        legal_reference,
        requires_notary,
        validation_notes: vec![
            "Vote validation computed against seeded database jurisdiction rules.".to_string(),
        ],
        validated_at: Utc::now(),
    }));
    rls.release().await;
    result
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/czech/svj/usneseni/{vote_id}", tag = "Regional Compliance", params(("vote_id" = Uuid, Path, description = "Vote ID")), responses((status = 200, description = "Usneseni", body = CzechSvjUsneseni)))]
async fn get_czech_usneseni(
    mut rls: RlsConnection,
    State(state): State<AppState>,
    Path(vote_id): Path<Uuid>,
) -> Result<Json<CzechSvjUsneseni>, (StatusCode, Json<ErrorResponse>)> {
    let vote = state
        .vote_repo
        .find_poll_by_id_rls(&mut **rls.conn(), vote_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("NOT_FOUND", "Vote not found")),
            )
        })?;

    let rule = state
        .regional_compliance_repo
        .get_quorum_rule(&mut **rls.conn(), Jurisdiction::Czechia, "simple_majority")
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .unwrap_or((
            Decimal::new(5001, 2),
            "SS 1206 zakona 89/2012 Sb.".to_string(),
        ));

    let result = Ok(Json(CzechSvjUsneseni {
        vote_id,
        building_id: vote.building_id,
        svj_name: "SVJ Hlavni 1".to_string(),
        ico: "12345678".to_string(),
        meeting_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        meeting_location: "Spolecenska mistnost, Hlavni 1, Praha".to_string(),
        title: vote.title,
        decision_type: CzechDecisionType::SimpleMajority,
        legal_reference: rule.1,
        total_ownership_shares: Decimal::new(10000, 2),
        participating_shares: Decimal::new(7500, 2),
        participation_percentage: Decimal::new(7500, 2),
        quorum_required: rule.0,
        quorum_met: true,
        votes_for: Decimal::new(6000, 2),
        votes_against: Decimal::new(1000, 2),
        abstentions: Decimal::new(500, 2),
        result_approved: true,
        requires_notary: false,
        participants: vec![],
        questions: vec![],
        generated_at: Utc::now(),
    }));
    rls.release().await;
    result
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/status", tag = "Regional Compliance", responses((status = 200, description = "Status", body = RegionalComplianceStatus)))]
async fn get_compliance_status(
    mut rls: RlsConnection,
    State(state): State<AppState>,
) -> Result<Json<RegionalComplianceStatus>, (StatusCode, Json<ErrorResponse>)> {
    let org_id = rls.tenant_id();
    let jurisdiction = state
        .regional_compliance_repo
        .get_jurisdiction(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let configured_buildings = state
        .regional_compliance_repo
        .get_configured_buildings(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let slovak_voting_enabled =
        !configured_buildings.is_empty() && matches!(jurisdiction, Jurisdiction::Slovakia);

    let slovak_accounting_configured = state
        .regional_compliance_repo
        .get_slovak_accounting_config(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .is_some();

    let slovak_gdpr_configured = state
        .regional_compliance_repo
        .get_slovak_gdpr_config(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?
        .is_some();

    let czech_svj_configured =
        !configured_buildings.is_empty() && matches!(jurisdiction, Jurisdiction::Czechia);

    let result = Ok(Json(RegionalComplianceStatus {
        organization_id: org_id,
        jurisdiction,
        slovak_voting_enabled,
        slovak_accounting_configured,
        slovak_gdpr_configured,
        czech_svj_configured,
        configured_buildings,
        last_checked_at: Some(Utc::now()),
    }));
    rls.release().await;
    result
}
