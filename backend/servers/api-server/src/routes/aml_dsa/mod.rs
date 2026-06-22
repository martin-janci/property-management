//! Advanced Compliance (AML/DSA) routes (Epic 67).
//!
//! Handles AML risk assessment, Enhanced Due Diligence, DSA transparency
//! reporting, and content moderation dashboard endpoints.
//!
//! The endpoint groups are split into per-story submodules; this module owns
//! the shared router wiring. Handler signatures, paths, and SQL are unchanged
//! from the original single-file form.

#![allow(clippy::type_complexity)]

use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

mod aml;
mod dsa;
mod edd;
mod moderation;
mod shared;

use aml::{
    create_aml_assessment, get_aml_assessment, get_aml_thresholds, get_country_risks,
    list_aml_assessments, review_aml_assessment,
};
use dsa::{
    download_dsa_report, generate_dsa_report, get_dsa_metrics, get_dsa_report, list_dsa_reports,
    publish_dsa_report,
};
use edd::{
    add_edd_note, complete_edd, get_edd_record, initiate_edd, list_pending_edd,
    upload_edd_document, verify_edd_document,
};
use moderation::{
    assign_moderation_case, decide_appeal, file_appeal, get_action_templates, get_moderation_case,
    get_moderation_queue, get_moderation_stats, report_content, take_moderation_action,
};

/// Create the AML/DSA compliance router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Story 67.1: AML Risk Assessment
        .route("/aml/assess", post(create_aml_assessment))
        .route("/aml/assessments", get(list_aml_assessments))
        .route("/aml/assessments/{id}", get(get_aml_assessment))
        .route("/aml/assessments/{id}/review", post(review_aml_assessment))
        .route("/aml/country-risks", get(get_country_risks))
        .route("/aml/thresholds", get(get_aml_thresholds))
        // Story 67.2: Enhanced Due Diligence
        .route("/edd", post(initiate_edd))
        .route("/edd/{id}", get(get_edd_record))
        .route("/edd/{id}/documents", post(upload_edd_document))
        .route(
            "/edd/{id}/documents/{doc_id}/verify",
            post(verify_edd_document),
        )
        .route("/edd/{id}/notes", post(add_edd_note))
        .route("/edd/{id}/complete", post(complete_edd))
        .route("/edd/pending", get(list_pending_edd))
        // Story 67.3: DSA Transparency Reports
        .route("/dsa/reports", get(list_dsa_reports))
        .route("/dsa/reports", post(generate_dsa_report))
        .route("/dsa/reports/{id}", get(get_dsa_report))
        .route("/dsa/reports/{id}/publish", post(publish_dsa_report))
        .route("/dsa/reports/{id}/download", get(download_dsa_report))
        .route("/dsa/metrics", get(get_dsa_metrics))
        // Story 67.4: Content Moderation Dashboard
        .route("/moderation/queue", get(get_moderation_queue))
        .route("/moderation/queue/stats", get(get_moderation_stats))
        .route("/moderation/cases/{id}", get(get_moderation_case))
        .route(
            "/moderation/cases/{id}/assign",
            post(assign_moderation_case),
        )
        .route(
            "/moderation/cases/{id}/action",
            post(take_moderation_action),
        )
        .route("/moderation/cases/{id}/appeal", post(file_appeal))
        .route("/moderation/cases/{id}/appeal/decide", post(decide_appeal))
        .route("/moderation/report", post(report_content))
        .route("/moderation/templates", get(get_action_templates))
}
