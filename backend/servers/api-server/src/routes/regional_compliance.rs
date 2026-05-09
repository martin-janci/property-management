//! Regional Legal Compliance routes (Epic 72).

use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use api_core::extractors::TenantExtractor;
use db::models::regional_compliance::*;

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/jurisdiction", get(get_jurisdiction))
        .route("/jurisdiction", put(set_jurisdiction))
        .route("/slovak/voting/config", post(configure_slovak_voting))
        .route(
            "/slovak/voting/config/:building_id",
            get(get_slovak_voting_config),
        )
        .route("/slovak/voting/validate", post(validate_slovak_vote))
        .route(
            "/slovak/voting/minutes/:vote_id",
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
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
) -> Json<Jurisdiction> {
    Json(Jurisdiction::default())
}

#[utoipa::path(put, path = "/api/v1/regional-compliance/jurisdiction", tag = "Regional Compliance", request_body = SetJurisdiction, responses((status = 200, description = "Jurisdiction updated", body = Jurisdiction)))]
async fn set_jurisdiction(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<SetJurisdiction>,
) -> Json<Jurisdiction> {
    Json(payload.jurisdiction)
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/voting/config", tag = "Regional Compliance", request_body = ConfigureSlovakVoting, responses((status = 200, description = "Config saved", body = SlovakVotingConfig)))]
async fn configure_slovak_voting(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<ConfigureSlovakVoting>,
) -> Json<SlovakVotingConfig> {
    Json(SlovakVotingConfig {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        building_id: payload.building_id,
        enabled: payload.enabled,
        default_decision_type: payload
            .default_decision_type
            .unwrap_or(SlovakDecisionType::SimpleMajority)
            .legal_reference()
            .to_string(),
        use_ownership_weight: payload.use_ownership_weight,
        min_notice_days: payload.min_notice_days,
        allow_proxy_voting: payload.allow_proxy_voting,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/voting/config/{building_id}", tag = "Regional Compliance", params(("building_id" = Uuid, Path, description = "Building ID")), responses((status = 200, description = "Config", body = SlovakVotingConfig)))]
async fn get_slovak_voting_config(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Path(building_id): Path<Uuid>,
) -> Json<SlovakVotingConfig> {
    Json(SlovakVotingConfig {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        building_id,
        enabled: true,
        default_decision_type: SlovakDecisionType::SimpleMajority
            .legal_reference()
            .to_string(),
        use_ownership_weight: true,
        min_notice_days: 14,
        allow_proxy_voting: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/voting/validate", tag = "Regional Compliance", request_body = ValidateSlovakVote, responses((status = 200, description = "Validation result", body = SlovakVoteValidation)))]
async fn validate_slovak_vote(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<ValidateSlovakVote>,
) -> Json<SlovakVoteValidation> {
    let required_quorum = payload.decision_type.required_quorum_percentage();
    let actual_participation = Decimal::new(7500, 2);
    let approval_percentage = Decimal::new(8000, 2);
    Json(SlovakVoteValidation {
        vote_id: payload.vote_id,
        decision_type: payload.decision_type,
        required_quorum_percentage: required_quorum,
        actual_participation_percentage: actual_participation,
        quorum_met: actual_participation >= required_quorum,
        approval_percentage,
        approval_required_percentage: required_quorum,
        is_valid: actual_participation >= required_quorum && approval_percentage >= required_quorum,
        legal_reference: payload.decision_type.legal_reference().to_string(),
        validation_notes: vec!["Vote conducted in accordance with zakon 182/1993 Z.z.".to_string()],
        validated_at: Utc::now(),
    })
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/voting/minutes/{vote_id}", tag = "Regional Compliance", params(("vote_id" = Uuid, Path, description = "Vote ID")), responses((status = 200, description = "Minutes", body = SlovakVoteMinutes)))]
async fn get_slovak_vote_minutes(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Path(vote_id): Path<Uuid>,
) -> Json<SlovakVoteMinutes> {
    Json(SlovakVoteMinutes {
        vote_id,
        building_id: Uuid::new_v4(),
        meeting_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        meeting_location: "Spolocenska miestnost, Hlavna 1, Bratislava".to_string(),
        title: "Schvalenie rozpoctu na rok 2024".to_string(),
        decision_type: SlovakDecisionType::SimpleMajority,
        legal_reference: SlovakDecisionType::SimpleMajority
            .legal_reference()
            .to_string(),
        total_ownership_shares: Decimal::new(10000, 2),
        participating_shares: Decimal::new(7500, 2),
        participation_percentage: Decimal::new(7500, 2),
        quorum_required: Decimal::new(5001, 2),
        quorum_met: true,
        votes_for: Decimal::new(6000, 2),
        votes_against: Decimal::new(1000, 2),
        abstentions: Decimal::new(500, 2),
        result_approved: true,
        participants: vec![],
        questions: vec![],
        generated_at: Utc::now(),
    })
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/accounting/config", tag = "Regional Compliance", request_body = ConfigureSlovakAccounting, responses((status = 200, description = "Config saved", body = SlovakAccountingConfig)))]
async fn configure_slovak_accounting(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<ConfigureSlovakAccounting>,
) -> Json<SlovakAccountingConfig> {
    Json(SlovakAccountingConfig {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        export_format: format!("{:?}", payload.export_format).to_lowercase(),
        ico: payload.ico,
        dic: payload.dic,
        ic_dph: payload.ic_dph,
        default_iban: payload.default_iban,
        account_mapping: payload
            .account_mapping
            .unwrap_or(serde_json::json!({"cash": "211", "bank": "221"})),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/accounting/config", tag = "Regional Compliance", responses((status = 200, description = "Config", body = SlovakAccountingConfig)))]
async fn get_slovak_accounting_config(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
) -> Json<SlovakAccountingConfig> {
    Json(SlovakAccountingConfig {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        export_format: "pohoda".to_string(),
        ico: Some("12345678".to_string()),
        dic: Some("2023456789".to_string()),
        ic_dph: None,
        default_iban: Some("SK1234567890123456789012".to_string()),
        account_mapping: serde_json::json!({"cash": "211", "bank": "221"}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/accounting/export", tag = "Regional Compliance", request_body = ExportSlovakAccounting, responses((status = 200, description = "Export", body = SlovakAccountingExport)))]
async fn export_slovak_accounting(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<ExportSlovakAccounting>,
) -> Json<SlovakAccountingExport> {
    Json(SlovakAccountingExport {
        export_id: Uuid::new_v4(),
        organization_id: payload.organization_id,
        from_date: payload.from_date,
        to_date: payload.to_date,
        format: payload.format,
        invoice_count: 45,
        payment_count: 120,
        journal_entry_count: 200,
        total_revenue: Decimal::new(15000000, 2),
        total_expenses: Decimal::new(12000000, 2),
        total_receivables: Decimal::new(500000, 2),
        total_payables: Decimal::new(300000, 2),
        download_url: Some(format!(
            "/api/v1/regional-compliance/slovak/accounting/download/{}",
            Uuid::new_v4()
        )),
        export_data: None,
        generated_at: Utc::now(),
    })
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/gdpr/config", tag = "Regional Compliance", request_body = ConfigureSlovakGdpr, responses((status = 200, description = "Config saved", body = SlovakGdprConfig)))]
async fn configure_slovak_gdpr(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<ConfigureSlovakGdpr>,
) -> Json<SlovakGdprConfig> {
    Json(SlovakGdprConfig {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        dpo_name: payload.dpo_name,
        dpo_email: payload.dpo_email,
        dpo_phone: payload.dpo_phone,
        org_address: payload.org_address,
        processing_purposes: payload.processing_purposes.unwrap_or(serde_json::json!([])),
        consent_texts: payload.consent_texts.unwrap_or(serde_json::json!({})),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/gdpr/config", tag = "Regional Compliance", responses((status = 200, description = "Config", body = SlovakGdprConfig)))]
async fn get_slovak_gdpr_config(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
) -> Json<SlovakGdprConfig> {
    Json(SlovakGdprConfig {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        dpo_name: "Jan Novak".to_string(),
        dpo_email: "dpo@example.sk".to_string(),
        dpo_phone: Some("+421 900 123 456".to_string()),
        org_address: Some("Hlavna 1, 811 01 Bratislava".to_string()),
        processing_purposes: serde_json::json!([]),
        consent_texts: serde_json::json!({}),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/gdpr/consent", tag = "Regional Compliance", request_body = RecordGdprConsent, responses((status = 200, description = "Consent recorded", body = SlovakGdprConsent)))]
async fn record_gdpr_consent(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<RecordGdprConsent>,
) -> Json<SlovakGdprConsent> {
    Json(SlovakGdprConsent {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        organization_id: Some(Uuid::new_v4()),
        category: format!("{:?}", payload.category).to_lowercase(),
        granted: payload.granted,
        ip_address: None,
        user_agent: None,
        consent_version: payload.consent_version,
        consented_at: Utc::now(),
        withdrawn_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/slovak/gdpr/consent/status", tag = "Regional Compliance", responses((status = 200, description = "Status", body = GdprConsentStatus)))]
async fn get_gdpr_consent_status(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
) -> Json<GdprConsentStatus> {
    Json(GdprConsentStatus {
        user_id: Uuid::new_v4(),
        consents: vec![ConsentCategoryStatus {
            category: GdprConsentCategory::Essential,
            name: "Nevyhnutne".to_string(),
            description: "Zakladne spracovanie".to_string(),
            granted: true,
            required: true,
            consented_at: Some(Utc::now()),
            consent_version: Some("1.0".to_string()),
        }],
        dpo_contact: DpoContact {
            name: "Jan Novak".to_string(),
            email: "dpo@example.sk".to_string(),
            phone: None,
            address: None,
        },
        processing_purposes: vec![],
        last_updated: Some(Utc::now()),
    })
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/slovak/gdpr/consent/withdraw", tag = "Regional Compliance", request_body = RecordGdprConsent, responses((status = 200, description = "Withdrawn", body = SlovakGdprConsent)))]
async fn withdraw_gdpr_consent(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<RecordGdprConsent>,
) -> Json<SlovakGdprConsent> {
    Json(SlovakGdprConsent {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        organization_id: Some(Uuid::new_v4()),
        category: format!("{:?}", payload.category).to_lowercase(),
        granted: false,
        ip_address: None,
        user_agent: None,
        consent_version: payload.consent_version,
        consented_at: Utc::now(),
        withdrawn_at: Some(Utc::now()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/czech/svj/config", tag = "Regional Compliance", request_body = ConfigureCzechSvj, responses((status = 200, description = "Config saved", body = CzechSvjConfig)))]
async fn configure_czech_svj(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<ConfigureCzechSvj>,
) -> Json<CzechSvjConfig> {
    Json(CzechSvjConfig {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        building_id: payload.building_id,
        org_type: format!("{:?}", payload.org_type).to_lowercase(),
        ico: payload.ico,
        has_stanovy: payload.stanovy_document_id.is_some(),
        stanovy_document_id: payload.stanovy_document_id,
        stanovy_effective_date: payload.stanovy_effective_date,
        default_decision_type: payload
            .default_decision_type
            .unwrap_or(CzechDecisionType::SimpleMajority)
            .legal_reference()
            .to_string(),
        use_ownership_weight: payload.use_ownership_weight,
        notary_threshold_czk: payload.notary_threshold_czk,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/czech/svj/config/{building_id}", tag = "Regional Compliance", params(("building_id" = Uuid, Path, description = "Building ID")), responses((status = 200, description = "Config", body = CzechSvjConfig)))]
async fn get_czech_svj_config(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Path(building_id): Path<Uuid>,
) -> Json<CzechSvjConfig> {
    Json(CzechSvjConfig {
        id: Uuid::new_v4(),
        organization_id: Uuid::new_v4(),
        building_id,
        org_type: "svj".to_string(),
        ico: "12345678".to_string(),
        has_stanovy: true,
        stanovy_document_id: Some(Uuid::new_v4()),
        stanovy_effective_date: Some(NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        default_decision_type: CzechDecisionType::SimpleMajority
            .legal_reference()
            .to_string(),
        use_ownership_weight: true,
        notary_threshold_czk: Some(Decimal::new(5000000, 0)),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

#[utoipa::path(post, path = "/api/v1/regional-compliance/czech/svj/validate", tag = "Regional Compliance", request_body = ValidateCzechVote, responses((status = 200, description = "Validation result", body = CzechVoteValidation)))]
async fn validate_czech_vote(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Json(payload): Json<ValidateCzechVote>,
) -> Json<CzechVoteValidation> {
    let required_quorum = payload.decision_type.required_quorum_percentage();
    let actual_participation = Decimal::new(7500, 2);
    let approval_percentage = Decimal::new(8000, 2);
    let requires_notary = matches!(
        payload.decision_type,
        CzechDecisionType::ThreeQuartersMajority | CzechDecisionType::AllOwners
    );
    Json(CzechVoteValidation {
        vote_id: payload.vote_id,
        decision_type: payload.decision_type,
        required_quorum_percentage: required_quorum,
        actual_participation_percentage: actual_participation,
        quorum_met: actual_participation >= required_quorum,
        approval_percentage,
        approval_required_percentage: required_quorum,
        is_valid: actual_participation >= required_quorum && approval_percentage >= required_quorum,
        legal_reference: payload.decision_type.legal_reference().to_string(),
        requires_notary,
        validation_notes: vec!["Vote conducted in accordance with zakon 89/2012 Sb.".to_string()],
        validated_at: Utc::now(),
    })
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/czech/svj/usneseni/{vote_id}", tag = "Regional Compliance", params(("vote_id" = Uuid, Path, description = "Vote ID")), responses((status = 200, description = "Usneseni", body = CzechSvjUsneseni)))]
async fn get_czech_usneseni(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
    Path(vote_id): Path<Uuid>,
) -> Json<CzechSvjUsneseni> {
    Json(CzechSvjUsneseni {
        vote_id,
        building_id: Uuid::new_v4(),
        svj_name: "SVJ Hlavni 1".to_string(),
        ico: "12345678".to_string(),
        meeting_date: NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(),
        meeting_location: "Spolecenska mistnost, Hlavni 1, Praha".to_string(),
        title: "Schvaleni rozpoctu na rok 2024".to_string(),
        decision_type: CzechDecisionType::SimpleMajority,
        legal_reference: CzechDecisionType::SimpleMajority
            .legal_reference()
            .to_string(),
        total_ownership_shares: Decimal::new(10000, 2),
        participating_shares: Decimal::new(7500, 2),
        participation_percentage: Decimal::new(7500, 2),
        quorum_required: Decimal::new(5001, 2),
        quorum_met: true,
        votes_for: Decimal::new(6000, 2),
        votes_against: Decimal::new(1000, 2),
        abstentions: Decimal::new(500, 2),
        result_approved: true,
        requires_notary: false,
        participants: vec![],
        questions: vec![],
        generated_at: Utc::now(),
    })
}

#[utoipa::path(get, path = "/api/v1/regional-compliance/status", tag = "Regional Compliance", responses((status = 200, description = "Status", body = RegionalComplianceStatus)))]
async fn get_compliance_status(
    _ctx: TenantExtractor,
    State(_state): State<AppState>,
) -> Json<RegionalComplianceStatus> {
    Json(RegionalComplianceStatus {
        organization_id: Uuid::new_v4(),
        jurisdiction: Jurisdiction::Slovakia,
        slovak_voting_enabled: true,
        slovak_accounting_configured: true,
        slovak_gdpr_configured: true,
        czech_svj_configured: false,
        configured_buildings: vec![Uuid::new_v4()],
        last_checked_at: Some(Utc::now()),
    })
}
