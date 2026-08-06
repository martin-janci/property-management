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
use db::models::vote::{Vote, VoteResults};

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

/// Voting figures for an official minutes / usneseni document, derived from the
/// **persisted** vote record.
///
/// Both `get_slovak_vote_minutes` (Slovak zapisnica, §14 z. 182/1993 Z.z.) and
/// `get_czech_usneseni` (Czech usneseni, §1206 z. 89/2012 Sb.) previously filled
/// these legally-binding documents with hardcoded mock values — a fixed
/// 2024-01-15 meeting date, 100/75 ownership shares, a 60/10/5
/// for/against/abstain split and `result_approved: true` — regardless of what
/// the vote actually decided. This derives every figure from the vote's
/// persisted counts, its `quorum_met` flag and its calculated `VoteResults`, so
/// the generated document reflects the real outcome. Ownership-share fields are
/// sourced from the eligible/participation counts (the only per-vote tally the
/// vote record carries); per-question and top-line for/against/abstain figures
/// come from the calculated result options.
struct VoteMinutesTally {
    meeting_date: NaiveDate,
    total_ownership_shares: Decimal,
    participating_shares: Decimal,
    participation_percentage: Decimal,
    quorum_met: bool,
    votes_for: Decimal,
    votes_against: Decimal,
    abstentions: Decimal,
    result_approved: bool,
    questions: Vec<QuestionMinutes>,
}

fn derive_vote_minutes_tally(
    vote: &Vote,
    required_quorum: Decimal,
    approved_label: &str,
    rejected_label: &str,
) -> VoteMinutesTally {
    let eligible = vote.eligible_count.unwrap_or(0).max(0);
    let participating = vote.participation_count.unwrap_or(0).max(0);
    let total_ownership_shares = Decimal::from(eligible);
    let participating_shares = Decimal::from(participating);
    let participation_percentage = if eligible > 0 {
        (participating_shares / total_ownership_shares * Decimal::new(100, 0)).round_dp(2)
    } else {
        Decimal::ZERO
    };

    let mut questions = Vec::new();
    let (mut votes_for, mut votes_against, mut abstentions) =
        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);

    if let Ok(results) = serde_json::from_value::<VoteResults>(vote.results.clone()) {
        for (idx, q) in results.questions.iter().enumerate() {
            let (mut q_for, mut q_against, mut q_abstain) =
                (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO);
            for opt in &q.results {
                let count = Decimal::from(opt.count.max(0));
                match opt.option_text.to_lowercase().as_str() {
                    "yes" | "for" | "za" | "ano" | "schvalujem" => q_for += count,
                    "no" | "against" | "proti" | "ne" | "neschvalujem" => q_against += count,
                    _ => q_abstain += count,
                }
            }
            questions.push(QuestionMinutes {
                question_text: q.question_text.clone(),
                votes_for: q_for,
                votes_against: q_against,
                abstentions: q_abstain,
                result: if q_for > q_against {
                    approved_label.to_string()
                } else {
                    rejected_label.to_string()
                },
            });
            // The top-line for/against/abstain figures mirror the primary
            // (first) decision question.
            if idx == 0 {
                votes_for = q_for;
                votes_against = q_against;
                abstentions = q_abstain;
            }
        }
    }

    // Prefer the persisted quorum flag; fall back to the computed participation
    // when the vote was never formally closed/calculated.
    let quorum_met = vote
        .quorum_met
        .unwrap_or(participation_percentage >= required_quorum);
    let result_approved = quorum_met && votes_for > votes_against;

    VoteMinutesTally {
        // The meeting/decision date is the date the vote closed, not a constant.
        meeting_date: vote.end_at.date_naive(),
        total_ownership_shares,
        participating_shares,
        participation_percentage,
        quorum_met,
        votes_for,
        votes_against,
        abstentions,
        result_approved,
        questions,
    }
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

    // Meeting location is the building's real address, not a fixed placeholder.
    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), vote.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;
    let meeting_location = building
        .as_ref()
        .map(|b| format!("{}, {}", b.street, b.city))
        .unwrap_or_default();

    let tally = derive_vote_minutes_tally(&vote, rule.0, "schvalene", "neschvalene");

    let result = Ok(Json(SlovakVoteMinutes {
        vote_id,
        building_id: vote.building_id,
        meeting_date: tally.meeting_date,
        meeting_location,
        title: vote.title,
        decision_type: SlovakDecisionType::SimpleMajority,
        legal_reference: rule.1,
        total_ownership_shares: tally.total_ownership_shares,
        participating_shares: tally.participating_shares,
        participation_percentage: tally.participation_percentage,
        quorum_required: rule.0,
        quorum_met: tally.quorum_met,
        votes_for: tally.votes_for,
        votes_against: tally.votes_against,
        abstentions: tally.abstentions,
        result_approved: tally.result_approved,
        participants: vec![],
        questions: tally.questions,
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
    // Named-struct result (#2122): field access below is compiler-checked, so a
    // revenue↔receivables (or expenses↔payables) transposition can no longer be
    // introduced silently the way a positional tuple destructure allowed.
    let metrics = state
        .regional_compliance_repo
        .get_accounting_metrics(rls.conn(), org_id, payload.from_date, payload.to_date)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;

    let journal_entry_count = metrics.invoice_count + metrics.payment_count;

    // Build via the smart constructor (#2086): `partial` + `unsupported_fields`
    // are derived inside `new` from the `Option` monetary fields, so this
    // handler cannot ship a "looks complete but isn't" export by forgetting a
    // mutator. Un-computed fields serialize as JSON `null` and are enumerated in
    // `unsupported_fields`, letting consumers distinguish "not available" from a
    // genuine zero (#2030).
    let export = SlovakAccountingExport::new(SlovakAccountingExportInput {
        export_id: Uuid::new_v4(),
        organization_id: org_id,
        from_date: payload.from_date,
        to_date: payload.to_date,
        format: payload.format,
        invoice_count: metrics.invoice_count,
        payment_count: metrics.payment_count,
        journal_entry_count,
        total_revenue: metrics.total_revenue,
        total_expenses: metrics.total_expenses,
        total_receivables: metrics.total_receivables,
        total_payables: metrics.total_payables,
        download_url: Some(format!(
            "/api/v1/regional-compliance/slovak/accounting/download/{}",
            Uuid::new_v4()
        )),
        export_data: None,
        generated_at: Utc::now(),
    });

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
    let org_id = rls.tenant_id();
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

    // SVJ identity (ICO) comes from the building's persisted Czech SVJ config,
    // not a hardcoded "12345678".
    let svj_config = state
        .regional_compliance_repo
        .get_czech_svj_config(&mut **rls.conn(), org_id, vote.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;
    let ico = svj_config
        .as_ref()
        .map(|c| c.ico.clone())
        .unwrap_or_default();

    // SVJ name is the organization name; meeting location is the building's
    // real address.
    let org = state
        .org_repo
        .find_by_id_rls(&mut **rls.conn(), org_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;
    let svj_name = org.as_ref().map(|o| o.name.clone()).unwrap_or_default();

    let building = state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), vote.building_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", e.to_string())),
            )
        })?;
    let meeting_location = building
        .as_ref()
        .map(|b| format!("{}, {}", b.street, b.city))
        .unwrap_or_default();

    let tally = derive_vote_minutes_tally(&vote, rule.0, "schvaleno", "neschvaleno");

    let result = Ok(Json(CzechSvjUsneseni {
        vote_id,
        building_id: vote.building_id,
        svj_name,
        ico,
        meeting_date: tally.meeting_date,
        meeting_location,
        title: vote.title,
        decision_type: CzechDecisionType::SimpleMajority,
        legal_reference: rule.1,
        total_ownership_shares: tally.total_ownership_shares,
        participating_shares: tally.participating_shares,
        participation_percentage: tally.participation_percentage,
        quorum_required: rule.0,
        quorum_met: tally.quorum_met,
        votes_for: tally.votes_for,
        votes_against: tally.votes_against,
        abstentions: tally.abstentions,
        result_approved: tally.result_approved,
        requires_notary: false,
        participants: vec![],
        questions: tally.questions,
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;

    /// Build a persisted-shaped `Vote` for the minutes-derivation tests.
    fn sample_vote(
        results: serde_json::Value,
        eligible: Option<i32>,
        participation: Option<i32>,
        quorum_met: Option<bool>,
    ) -> Vote {
        Vote {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            building_id: Uuid::new_v4(),
            title: "Annual General Meeting 2026".to_string(),
            description: None,
            start_at: None,
            end_at: Utc.with_ymd_and_hms(2026, 3, 20, 18, 0, 0).unwrap(),
            status: "closed".to_string(),
            quorum_type: "simple_majority".to_string(),
            quorum_percentage: Some(50),
            allow_delegation: false,
            anonymous_voting: false,
            participation_count: participation,
            eligible_count: eligible,
            quorum_met,
            results,
            results_calculated_at: None,
            created_by: Uuid::new_v4(),
            published_by: None,
            published_at: None,
            cancelled_by: None,
            cancelled_at: None,
            cancellation_reason: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Regression for the hardcoded-vote-minutes bug: every figure in the
    /// generated legal document must come from the persisted vote record, never
    /// the old mock constants (2024-01-15 date, 100/75 shares, 60/10/5 split,
    /// always-approved).
    #[test]
    fn tally_is_sourced_from_persisted_vote_not_hardcoded() {
        let results = json!({
            "vote_id": Uuid::new_v4(),
            "participation_count": 8,
            "eligible_count": 10,
            "participation_rate": 80.0,
            "quorum_met": true,
            "calculated_at": "2026-03-20T18:05:00Z",
            "questions": [{
                "question_id": Uuid::new_v4(),
                "question_text": "Approve the 2026 repair fund?",
                "question_type": "yes_no",
                "total_votes": 8,
                "weighted_total": 8.0,
                "winner": null,
                "results": [
                    {"option_id": Uuid::new_v4(), "option_text": "Za", "count": 6, "weighted_count": 6.0, "percentage": 75.0},
                    {"option_id": Uuid::new_v4(), "option_text": "Proti", "count": 1, "weighted_count": 1.0, "percentage": 12.5},
                    {"option_id": Uuid::new_v4(), "option_text": "Zdrzujem sa", "count": 1, "weighted_count": 1.0, "percentage": 12.5}
                ]
            }]
        });
        let vote = sample_vote(results, Some(10), Some(8), Some(true));

        let tally =
            derive_vote_minutes_tally(&vote, Decimal::new(5001, 2), "schvalene", "neschvalene");

        // Meeting date is the vote's close date, NOT the old hardcoded constant.
        assert_eq!(
            tally.meeting_date,
            NaiveDate::from_ymd_opt(2026, 3, 20).unwrap()
        );
        assert_ne!(
            tally.meeting_date,
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
        // Shares are sourced from the persisted eligible/participation counts.
        assert_eq!(tally.total_ownership_shares, Decimal::from(10));
        assert_eq!(tally.participating_shares, Decimal::from(8));
        assert_eq!(tally.participation_percentage, Decimal::new(8000, 2));
        // For/against/abstain come from the real result options (6/1/1), not 60/10/5.
        assert_eq!(tally.votes_for, Decimal::from(6));
        assert_eq!(tally.votes_against, Decimal::from(1));
        assert_eq!(tally.abstentions, Decimal::from(1));
        assert!(tally.quorum_met);
        assert!(tally.result_approved);
        assert_eq!(tally.questions.len(), 1);
        assert_eq!(tally.questions[0].result, "schvalene");
    }

    /// A rejected / failed vote must render as rejected — the old code always
    /// returned `result_approved: true`.
    #[test]
    fn tally_reflects_a_rejected_vote() {
        let results = json!({
            "vote_id": Uuid::new_v4(),
            "participation_count": 2,
            "eligible_count": 10,
            "participation_rate": 20.0,
            "quorum_met": false,
            "calculated_at": "2026-03-20T18:05:00Z",
            "questions": [{
                "question_id": Uuid::new_v4(),
                "question_text": "Approve?",
                "question_type": "yes_no",
                "total_votes": 2,
                "weighted_total": 2.0,
                "winner": null,
                "results": [
                    {"option_id": Uuid::new_v4(), "option_text": "Yes", "count": 0, "weighted_count": 0.0, "percentage": 0.0},
                    {"option_id": Uuid::new_v4(), "option_text": "No", "count": 2, "weighted_count": 2.0, "percentage": 100.0}
                ]
            }]
        });
        let vote = sample_vote(results, Some(10), Some(2), Some(false));

        let tally =
            derive_vote_minutes_tally(&vote, Decimal::new(5001, 2), "schvaleno", "neschvaleno");

        assert_eq!(tally.votes_for, Decimal::ZERO);
        assert_eq!(tally.votes_against, Decimal::from(2));
        assert!(!tally.quorum_met);
        assert!(!tally.result_approved);
        assert_eq!(tally.questions[0].result, "neschvaleno");
    }

    /// A vote with no calculated results must not fabricate a tally.
    #[test]
    fn tally_handles_empty_results() {
        let vote = sample_vote(json!({}), Some(0), Some(0), None);
        let tally =
            derive_vote_minutes_tally(&vote, Decimal::new(5001, 2), "schvalene", "neschvalene");
        assert_eq!(tally.total_ownership_shares, Decimal::ZERO);
        assert_eq!(tally.participation_percentage, Decimal::ZERO);
        assert_eq!(tally.votes_for, Decimal::ZERO);
        assert!(tally.questions.is_empty());
        assert!(!tally.result_approved);
    }
}
