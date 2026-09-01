//! Service Provider Marketplace routes (Epic 68).
//!
//! Stories:
//! - 68.1: Service Provider Profiles
//! - 68.2: Search & Discovery
//! - 68.3: Request for Quote (RFQ)
//! - 68.4: Provider Verification
//! - 68.5: Reviews & Ratings

use api_core::extractors::AuthUser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    CreateProviderQuote, CreateProviderReview, CreateProviderVerification, CreateRequestForQuote,
    CreateServiceProviderProfile, ExpiringVerification, ManagerMarketplaceDashboard,
    MarketplaceSearchQuery, MarketplaceStatistics, ModerateReviewRequest, ProviderBadge,
    ProviderDashboard, ProviderProfileComplete, ProviderQuote, ProviderReview,
    ProviderReviewResponse, ProviderReviewWithResponse, ProviderSearchResult, ProviderVerification,
    QuoteComparisonView, RatingBreakdown, RequestForQuote, ReviewQuery, ReviewVerificationRequest,
    RfqInvitation, RfqQuery, ServiceProviderProfile, UpdateProviderQuote, UpdateProviderReview,
    UpdateRequestForQuote, UpdateServiceProviderProfile, VerificationQuery, VerificationQueueItem,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use utoipa::IntoParams;
use uuid::Uuid;

use crate::routes::pagination::clamp_limit;
use crate::state::AppState;

/// Rating dimension configuration
pub mod rating_dimensions {
    /// Number of rating dimensions used for calculating overall rating.
    /// Must match the number of dimensions in DIMENSION_NAMES.
    pub const COUNT: f64 = 4.0;

    /// Names of rating dimensions that contribute to overall rating.
    /// Used for validation and documentation.
    pub const DIMENSION_NAMES: &[&str] = &["quality", "timeliness", "communication", "value"];
}

/// Number of rating dimensions (deprecated - use rating_dimensions::COUNT).
const RATING_DIMENSIONS_COUNT: f64 = rating_dimensions::COUNT;

/// Verify the authenticated user belongs to `org_id` before serving
/// organization-scoped marketplace data (RFQs, quotes, reviews).
///
/// Mirrors the `verify_org_access` idiom in `routes/financial.rs`. Returns
/// `404` (not `403`) on a non-member so cross-tenant probing cannot
/// distinguish "missing" from "forbidden". RFQ/quote/review lookups in this
/// module otherwise key on the object id alone, which lets a member of org A
/// read or mutate org B's records (cross-org IDOR).
async fn verify_org_access(
    state: &AppState,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let is_member = state
        .org_member_repo
        .is_member(org_id, user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to check org membership");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Database error")),
            )
        })?;

    if !is_member {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Resource not found")),
        ));
    }

    Ok(())
}

/// Load a quote by id and verify the caller is the provider that submitted it.
///
/// Quotes are mutated only by their submitting provider. The repo
/// update/withdraw methods key on the quote id alone, so without this gate any
/// provider could edit or withdraw another provider's quote (IDOR). Returns
/// 404 when the quote is missing or owned by a different provider.
async fn verify_quote_owner(
    state: &AppState,
    user_id: Uuid,
    quote_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let quote = state
        .marketplace_repo
        .find_quote_by_id(quote_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to load quote");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Quote {} not found", quote_id),
                )),
            )
        })?;

    let provider = state
        .marketplace_repo
        .find_profile_by_user_id(user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    match provider {
        Some(p) if p.id == quote.provider_id => Ok(()),
        _ => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                format!("Quote {} not found", quote_id),
            )),
        )),
    }
}

/// Load an RFQ by id and verify the caller belongs to its organization.
///
/// Returns 404 when the RFQ is missing or owned by another tenant. Used by the
/// RFQ mutation/quote routes (`update`/`delete`/`award`/`cancel`/quote
/// listings) whose repo methods key on the RFQ id alone.
async fn load_rfq_for_org(
    state: &AppState,
    user_id: Uuid,
    rfq_id: Uuid,
) -> Result<RequestForQuote, (StatusCode, Json<ErrorResponse>)> {
    let rfq = state
        .marketplace_repo
        .find_rfq_by_id(rfq_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to load RFQ");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("RFQ {} not found", rfq_id),
                )),
            )
        })?;

    verify_org_access(state, user_id, rfq.organization_id).await?;

    Ok(rfq)
}

/// Resolve the caller's service-provider profile id, or 403 if they don't have
/// one. Provider-owned resources (invitations, quotes, verifications) are gated
/// on this id so a provider can only act on its own rows.
async fn current_provider_id(
    state: &AppState,
    user_id: Uuid,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    let provider = state
        .marketplace_repo
        .find_profile_by_user_id(user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_PROVIDER",
                    "You must have a provider profile to perform this action",
                )),
            )
        })?;

    Ok(provider.id)
}

/// Gate platform-moderation endpoints (verification review, badge award/revoke,
/// verification queues) on the platform-admin role. These operate on cross-org
/// provider trust data and were previously callable by any authenticated user
/// (PAP-140), letting an org member approve verifications or revoke badges
/// platform-wide.
fn require_platform_admin(user: &AuthUser) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    if user.is_platform_admin() {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "FORBIDDEN",
                "Platform administrator role required",
            )),
        ))
    }
}

/// Create marketplace router with all sub-routes.
pub fn router() -> Router<AppState> {
    Router::new()
        // Provider Profile routes (Story 68.1)
        .route("/providers", post(create_profile))
        .route("/providers", get(search_providers))
        .route("/providers/me", get(get_my_profile))
        .route("/providers/me", patch(update_my_profile))
        .route("/providers/me/dashboard", get(get_provider_dashboard))
        .route("/providers/statistics", get(get_marketplace_statistics))
        .route("/providers/{id}", get(get_provider))
        .route("/providers/{id}/complete", get(get_provider_complete))
        // RFQ routes (Story 68.3)
        .route("/rfqs", post(create_rfq))
        .route("/rfqs", get(list_rfqs))
        .route("/rfqs/{id}", get(get_rfq))
        .route("/rfqs/{id}", patch(update_rfq))
        .route("/rfqs/{id}", delete(delete_rfq))
        .route("/rfqs/{id}/quotes", get(list_rfq_quotes))
        .route("/rfqs/{id}/compare", get(compare_quotes))
        .route("/rfqs/{id}/award", post(award_quote))
        .route("/rfqs/{id}/cancel", post(cancel_rfq))
        // Quote routes (Story 68.3)
        .route("/quotes", post(submit_quote))
        .route("/quotes/my", get(list_my_quotes))
        .route("/quotes/{id}", get(get_quote))
        .route("/quotes/{id}", patch(update_quote))
        .route("/quotes/{id}", delete(withdraw_quote))
        // Invitation routes (Story 68.3)
        .route("/invitations", get(list_my_invitations))
        .route("/invitations/{id}/view", post(mark_invitation_viewed))
        .route("/invitations/{id}/decline", post(decline_invitation))
        // Verification routes (Story 68.4)
        .route("/verifications", post(submit_verification))
        .route("/verifications", get(list_verifications))
        .route("/verifications/queue", get(get_verification_queue))
        .route("/verifications/expiring", get(get_expiring_verifications))
        .route("/verifications/{id}", get(get_verification))
        .route("/verifications/{id}/review", post(review_verification))
        // Badge routes (Story 68.4)
        .route("/providers/{id}/badges", get(list_provider_badges))
        .route("/providers/{id}/badges", post(award_badge))
        .route("/badges/{id}", delete(revoke_badge))
        // Review routes (Story 68.5)
        .route("/providers/{id}/reviews", post(create_review))
        .route("/providers/{id}/reviews", get(list_provider_reviews))
        .route("/providers/{id}/ratings", get(get_rating_breakdown))
        .route("/reviews", get(list_reviews))
        .route("/reviews/{id}", get(get_review))
        .route("/reviews/{id}", patch(update_review))
        .route("/reviews/{id}", delete(delete_review))
        .route("/reviews/{id}/respond", post(respond_to_review))
        .route("/reviews/{id}/moderate", post(moderate_review))
        .route("/reviews/{id}/helpful", post(mark_review_helpful))
        // Manager dashboard (Story 68.2)
        .route("/dashboard", get(get_manager_dashboard))
}

// ==================== Request/Response Types ====================

/// Organization query parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct OrgQuery {
    pub organization_id: Uuid,
}

/// Create profile request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateProfileRequest {
    #[serde(flatten)]
    pub data: CreateServiceProviderProfile,
}

/// Search providers query with all filters.
#[derive(Debug, Deserialize, IntoParams)]
pub struct SearchProvidersQuery {
    pub category: Option<String>,
    pub query: Option<String>,
    pub location: Option<String>,
    pub region: Option<String>,
    pub min_rating: Option<Decimal>,
    pub max_hourly_rate: Option<Decimal>,
    pub min_hourly_rate: Option<Decimal>,
    pub verified_only: Option<bool>,
    pub emergency_only: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&SearchProvidersQuery> for MarketplaceSearchQuery {
    fn from(q: &SearchProvidersQuery) -> Self {
        MarketplaceSearchQuery {
            category: q.category.clone(),
            categories: None,
            query: q.query.clone(),
            location: q.location.clone(),
            postal_codes: None,
            region: q.region.clone(),
            min_rating: q.min_rating,
            max_hourly_rate: q.max_hourly_rate,
            min_hourly_rate: q.min_hourly_rate,
            verified_only: q.verified_only,
            badges: None,
            emergency_only: q.emergency_only,
            sort_by: q.sort_by.clone(),
            sort_order: q.sort_order.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Create RFQ request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateRfqRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateRequestForQuote,
}

/// List RFQs query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListRfqsQuery {
    pub organization_id: Uuid,
    pub status: Option<String>,
    pub service_category: Option<String>,
    pub building_id: Option<Uuid>,
    pub is_urgent: Option<bool>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListRfqsQuery> for RfqQuery {
    fn from(q: &ListRfqsQuery) -> Self {
        RfqQuery {
            status: q.status.clone(),
            service_category: q.service_category.clone(),
            building_id: q.building_id,
            is_urgent: q.is_urgent,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Award quote request.
#[derive(Debug, Deserialize)]
pub struct AwardQuoteRequest {
    pub quote_id: Uuid,
}

/// Submit quote request wrapper.
#[derive(Debug, Deserialize)]
pub struct SubmitQuoteRequest {
    #[serde(flatten)]
    pub data: CreateProviderQuote,
}

/// List verifications query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListVerificationsQuery {
    pub provider_id: Option<Uuid>,
    pub verification_type: Option<String>,
    pub status: Option<String>,
    pub expiring_days: Option<i32>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListVerificationsQuery> for VerificationQuery {
    fn from(q: &ListVerificationsQuery) -> Self {
        VerificationQuery {
            provider_id: q.provider_id,
            verification_type: q.verification_type.clone(),
            status: q.status.clone(),
            expiring_days: q.expiring_days,
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Expiring verifications query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ExpiringQuery {
    pub days: Option<i32>,
}

/// Award badge request.
#[derive(Debug, Deserialize)]
pub struct AwardBadgeRequest {
    pub badge_type: String,
    pub verification_id: Option<Uuid>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub notes: Option<String>,
}

/// Create review request wrapper.
#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub organization_id: Uuid,
    #[serde(flatten)]
    pub data: CreateProviderReview,
}

/// List reviews query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ListReviewsQuery {
    pub provider_id: Option<Uuid>,
    pub organization_id: Option<Uuid>,
    pub min_rating: Option<i32>,
    pub status: Option<String>,
    pub has_response: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

impl From<&ListReviewsQuery> for ReviewQuery {
    fn from(q: &ListReviewsQuery) -> Self {
        ReviewQuery {
            provider_id: q.provider_id,
            reviewer_id: None,
            organization_id: q.organization_id,
            min_rating: q.min_rating,
            status: q.status.clone(),
            has_response: q.has_response,
            sort_by: q.sort_by.clone(),
            sort_order: q.sort_order.clone(),
            limit: q.limit,
            offset: q.offset,
        }
    }
}

/// Pagination query.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

// ==================== Story 68.1: Provider Profile Endpoints ====================

/// Create a new service provider profile.
async fn create_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateProfileRequest>,
) -> Result<(StatusCode, Json<ServiceProviderProfile>), (StatusCode, Json<ErrorResponse>)> {
    let profile = state
        .marketplace_repo
        .create_profile(user.user_id, payload.data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create profile")),
            )
        })?;

    tracing::info!(
        profile_id = %profile.id,
        user_id = %user.user_id,
        "Provider profile created"
    );

    Ok((StatusCode::CREATED, Json(profile)))
}

/// Search for service providers in the marketplace.
async fn search_providers(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<SearchProvidersQuery>,
) -> Result<Json<Vec<ProviderSearchResult>>, (StatusCode, Json<ErrorResponse>)> {
    let search_query: MarketplaceSearchQuery = (&query).into();

    let results = state
        .marketplace_repo
        .search_providers(&search_query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to search providers");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to search providers")),
            )
        })?;

    Ok(Json(results))
}

/// Get the current user's provider profile.
async fn get_my_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<ServiceProviderProfile>, (StatusCode, Json<ErrorResponse>)> {
    let profile = state
        .marketplace_repo
        .find_profile_by_user_id(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Provider profile not found",
                )),
            )
        })?;

    Ok(Json(profile))
}

/// Update the current user's provider profile.
async fn update_my_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(data): Json<UpdateServiceProviderProfile>,
) -> Result<Json<ServiceProviderProfile>, (StatusCode, Json<ErrorResponse>)> {
    // First find the profile by user_id
    let existing = state
        .marketplace_repo
        .find_profile_by_user_id(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Provider profile not found",
                )),
            )
        })?;

    let profile = state
        .marketplace_repo
        .update_profile(existing.id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update profile")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Provider profile not found",
                )),
            )
        })?;

    tracing::info!(
        profile_id = %profile.id,
        user_id = %user.user_id,
        "Provider profile updated"
    );

    Ok(Json(profile))
}

/// Get provider dashboard with summary statistics.
async fn get_provider_dashboard(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<ProviderDashboard>, (StatusCode, Json<ErrorResponse>)> {
    let dashboard = state
        .marketplace_repo
        .get_provider_dashboard(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get provider dashboard");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Provider profile not found",
                )),
            )
        })?;

    Ok(Json(dashboard))
}

/// Get marketplace statistics.
async fn get_marketplace_statistics(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<MarketplaceStatistics>, (StatusCode, Json<ErrorResponse>)> {
    let stats = state.marketplace_repo.get_statistics().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get marketplace statistics");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Database error")),
        )
    })?;

    Ok(Json(stats))
}

/// Get a specific provider's profile.
async fn get_provider(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ServiceProviderProfile>, (StatusCode, Json<ErrorResponse>)> {
    let profile = state
        .marketplace_repo
        .find_profile_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get provider");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Provider {} not found", id),
                )),
            )
        })?;

    Ok(Json(profile))
}

/// Get complete provider profile with verifications, badges, and reviews.
async fn get_provider_complete(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ProviderProfileComplete>, (StatusCode, Json<ErrorResponse>)> {
    let complete = state
        .marketplace_repo
        .get_complete_profile(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get complete provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Provider {} not found", id),
                )),
            )
        })?;

    Ok(Json(complete))
}

// ==================== Story 68.3: RFQ Endpoints ====================

/// Create a new request for quote.
async fn create_rfq(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateRfqRequest>,
) -> Result<(StatusCode, Json<RequestForQuote>), (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, user.user_id, payload.organization_id).await?;

    let rfq = state
        .marketplace_repo
        .create_rfq(payload.organization_id, user.user_id, payload.data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create RFQ");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create RFQ")),
            )
        })?;

    tracing::info!(
        rfq_id = %rfq.id,
        org_id = %rfq.organization_id,
        user_id = %user.user_id,
        "RFQ created"
    );

    Ok((StatusCode::CREATED, Json(rfq)))
}

/// List RFQs for an organization.
async fn list_rfqs(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListRfqsQuery>,
) -> Result<Json<Vec<RequestForQuote>>, (StatusCode, Json<ErrorResponse>)> {
    verify_org_access(&state, user.user_id, query.organization_id).await?;

    let rfq_query: RfqQuery = (&query).into();

    let rfqs = state
        .marketplace_repo
        .list_rfqs(query.organization_id, &rfq_query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list RFQs");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list RFQs")),
            )
        })?;

    Ok(Json(rfqs))
}

/// Get a specific RFQ.
async fn get_rfq(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RequestForQuote>, (StatusCode, Json<ErrorResponse>)> {
    let rfq = state
        .marketplace_repo
        .find_rfq_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get RFQ");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("RFQ {} not found", id),
                )),
            )
        })?;

    verify_org_access(&state, user.user_id, rfq.organization_id).await?;

    Ok(Json(rfq))
}

/// Update an RFQ.
async fn update_rfq(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateRequestForQuote>,
) -> Result<Json<RequestForQuote>, (StatusCode, Json<ErrorResponse>)> {
    load_rfq_for_org(&state, user.user_id, id).await?;

    let rfq = state
        .marketplace_repo
        .update_rfq(id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update RFQ");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update RFQ")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("RFQ {} not found", id),
                )),
            )
        })?;

    Ok(Json(rfq))
}

/// Delete an RFQ.
async fn delete_rfq(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    load_rfq_for_org(&state, user.user_id, id).await?;

    let deleted = state.marketplace_repo.delete_rfq(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to delete RFQ");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to delete RFQ")),
        )
    })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                format!("RFQ {} not found or cannot be deleted", id),
            )),
        ))
    }
}

/// List quotes for an RFQ.
async fn list_rfq_quotes(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ProviderQuote>>, (StatusCode, Json<ErrorResponse>)> {
    load_rfq_for_org(&state, user.user_id, id).await?;

    let quotes = state
        .marketplace_repo
        .list_rfq_quotes(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list RFQ quotes");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list quotes")),
            )
        })?;

    Ok(Json(quotes))
}

/// Compare quotes for an RFQ side-by-side.
async fn compare_quotes(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<QuoteComparisonView>, (StatusCode, Json<ErrorResponse>)> {
    load_rfq_for_org(&state, user.user_id, id).await?;

    let comparison = state
        .marketplace_repo
        .get_quote_comparison(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get quote comparison");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("RFQ {} not found", id),
                )),
            )
        })?;

    Ok(Json(comparison))
}

/// Award an RFQ to a provider.
async fn award_quote(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AwardQuoteRequest>,
) -> Result<Json<RequestForQuote>, (StatusCode, Json<ErrorResponse>)> {
    load_rfq_for_org(&state, user.user_id, id).await?;

    // IDOR guard: `quote_id` comes from the request body, while the org check
    // above only proves the caller owns the RFQ path param `{id}`. Verify the
    // awarded quote actually belongs to THIS rfq before awarding — otherwise a
    // caller could award a quote from a foreign RFQ (and, transitively, another
    // org) by guessing its id. Surface a mismatch as 404 (same shape as a
    // missing quote) so quote existence is not leaked across tenants.
    let quote = state
        .marketplace_repo
        .find_quote_by_id(data.quote_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to load quote for award");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .filter(|q| q.rfq_id == id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Quote {} not found for RFQ {}", data.quote_id, id),
                )),
            )
        })?;

    let rfq = state
        .marketplace_repo
        .award_rfq(id, quote.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to award RFQ");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to award RFQ")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("RFQ {} not found", id),
                )),
            )
        })?;

    tracing::info!(
        rfq_id = %id,
        quote_id = %data.quote_id,
        user_id = %user.user_id,
        "RFQ awarded"
    );

    Ok(Json(rfq))
}

/// Cancel an RFQ.
async fn cancel_rfq(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RequestForQuote>, (StatusCode, Json<ErrorResponse>)> {
    load_rfq_for_org(&state, user.user_id, id).await?;

    let rfq = state
        .marketplace_repo
        .cancel_rfq(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to cancel RFQ");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to cancel RFQ")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("RFQ {} not found or cannot be cancelled", id),
                )),
            )
        })?;

    tracing::info!(rfq_id = %id, user_id = %user.user_id, "RFQ cancelled");

    Ok(Json(rfq))
}

/// Submit a quote for an RFQ.
async fn submit_quote(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<SubmitQuoteRequest>,
) -> Result<(StatusCode, Json<ProviderQuote>), (StatusCode, Json<ErrorResponse>)> {
    // Get the provider profile for this user
    let provider = state
        .marketplace_repo
        .find_profile_by_user_id(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_PROVIDER",
                    "You must have a provider profile to submit quotes",
                )),
            )
        })?;

    let quote = state
        .marketplace_repo
        .submit_quote(provider.id, payload.data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to submit quote");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to submit quote")),
            )
        })?;

    tracing::info!(
        quote_id = %quote.id,
        rfq_id = %quote.rfq_id,
        provider_id = %provider.id,
        "Quote submitted"
    );

    Ok((StatusCode::CREATED, Json(quote)))
}

/// List quotes submitted by the current provider.
async fn list_my_quotes(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ProviderQuote>>, (StatusCode, Json<ErrorResponse>)> {
    // Get the provider profile for this user
    let provider = state
        .marketplace_repo
        .find_profile_by_user_id(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Provider profile not found",
                )),
            )
        })?;

    let quotes = state
        .marketplace_repo
        .list_provider_quotes(
            provider.id,
            clamp_limit(query.limit.map(i64::from), 20) as i32,
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list quotes");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list quotes")),
            )
        })?;

    Ok(Json(quotes))
}

/// Get a specific quote.
async fn get_quote(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ProviderQuote>, (StatusCode, Json<ErrorResponse>)> {
    let quote = state
        .marketplace_repo
        .find_quote_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get quote");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Quote {} not found", id),
                )),
            )
        })?;

    // A quote belongs to the org that owns its RFQ — gate on RFQ membership so
    // a member of another tenant cannot read it by id.
    load_rfq_for_org(&state, user.user_id, quote.rfq_id).await?;

    Ok(Json(quote))
}

/// Update a quote (before submission or while still editable).
async fn update_quote(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateProviderQuote>,
) -> Result<Json<ProviderQuote>, (StatusCode, Json<ErrorResponse>)> {
    verify_quote_owner(&state, user.user_id, id).await?;

    let quote = state
        .marketplace_repo
        .update_quote(id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update quote");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update quote")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Quote {} not found or cannot be updated", id),
                )),
            )
        })?;

    Ok(Json(quote))
}

/// Withdraw a submitted quote.
async fn withdraw_quote(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    verify_quote_owner(&state, user.user_id, id).await?;

    let withdrawn = state
        .marketplace_repo
        .withdraw_quote(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to withdraw quote");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to withdraw quote")),
            )
        })?;

    if withdrawn {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                format!("Quote {} not found or cannot be withdrawn", id),
            )),
        ))
    }
}

/// List RFQ invitations for the current provider.
async fn list_my_invitations(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<RfqInvitation>>, (StatusCode, Json<ErrorResponse>)> {
    // Get the provider profile for this user
    let provider = state
        .marketplace_repo
        .find_profile_by_user_id(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    "Provider profile not found",
                )),
            )
        })?;

    let invitations = state
        .marketplace_repo
        .list_provider_invitations(
            provider.id,
            clamp_limit(query.limit.map(i64::from), 20) as i32,
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list invitations");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list invitations")),
            )
        })?;

    Ok(Json(invitations))
}

/// Mark an invitation as viewed.
async fn mark_invitation_viewed(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RfqInvitation>, (StatusCode, Json<ErrorResponse>)> {
    // Only the invited provider may act on the invitation — key the update on
    // the caller's verified provider id (PAP-140).
    let provider_id = current_provider_id(&state, user.user_id).await?;

    let invitation = state
        .marketplace_repo
        .mark_invitation_viewed(id, provider_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to mark invitation viewed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Invitation {} not found", id),
                )),
            )
        })?;

    Ok(Json(invitation))
}

/// Decline an RFQ invitation.
#[derive(Debug, Deserialize)]
pub struct DeclineInvitationRequest {
    pub reason: Option<String>,
}

async fn decline_invitation(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<DeclineInvitationRequest>,
) -> Result<Json<RfqInvitation>, (StatusCode, Json<ErrorResponse>)> {
    // Only the invited provider may decline — key on the caller's verified
    // provider id (PAP-140; previously had zero ownership check).
    let provider_id = current_provider_id(&state, user.user_id).await?;

    let invitation = state
        .marketplace_repo
        .decline_invitation(id, provider_id, data.reason)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to decline invitation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Invitation {} not found", id),
                )),
            )
        })?;

    Ok(Json(invitation))
}

// ==================== Story 68.4: Verification Endpoints ====================

/// Submit a verification document.
async fn submit_verification(
    State(state): State<AppState>,
    user: AuthUser,
    Json(payload): Json<CreateProviderVerification>,
) -> Result<(StatusCode, Json<ProviderVerification>), (StatusCode, Json<ErrorResponse>)> {
    // Get the provider profile for this user
    let provider = state
        .marketplace_repo
        .find_profile_by_user_id(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_PROVIDER",
                    "You must have a provider profile to submit verifications",
                )),
            )
        })?;

    let verification = state
        .marketplace_repo
        .submit_verification(provider.id, payload)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to submit verification");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to submit verification",
                )),
            )
        })?;

    tracing::info!(
        verification_id = %verification.id,
        provider_id = %provider.id,
        "Verification submitted"
    );

    Ok((StatusCode::CREATED, Json(verification)))
}

/// List verifications.
async fn list_verifications(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ListVerificationsQuery>,
) -> Result<Json<Vec<ProviderVerification>>, (StatusCode, Json<ErrorResponse>)> {
    // Cross-provider verification listing is a platform-moderation view (PAP-140).
    require_platform_admin(&user)?;

    let verification_query: VerificationQuery = (&query).into();

    let verifications = state
        .marketplace_repo
        .list_verifications(&verification_query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list verifications");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to list verifications",
                )),
            )
        })?;

    Ok(Json(verifications))
}

/// Get verification queue for admin review.
async fn get_verification_queue(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<VerificationQueueItem>>, (StatusCode, Json<ErrorResponse>)> {
    require_platform_admin(&user)?;

    let queue = state
        .marketplace_repo
        .get_verification_queue(
            clamp_limit(query.limit.map(i64::from), 20) as i32,
            query.offset.unwrap_or(0),
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get verification queue");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    Ok(Json(queue))
}

/// Get expiring verifications.
async fn get_expiring_verifications(
    State(state): State<AppState>,
    user: AuthUser,
    Query(query): Query<ExpiringQuery>,
) -> Result<Json<Vec<ExpiringVerification>>, (StatusCode, Json<ErrorResponse>)> {
    require_platform_admin(&user)?;

    let days = query.days.unwrap_or(30);

    let expiring = state
        .marketplace_repo
        .get_expiring_verifications(days)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get expiring verifications");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    Ok(Json(expiring))
}

/// Get a specific verification.
async fn get_verification(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ProviderVerification>, (StatusCode, Json<ErrorResponse>)> {
    let verification = state
        .marketplace_repo
        .find_verification_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get verification");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Verification {} not found", id),
                )),
            )
        })?;

    // Verification documents carry sensitive data (document numbers, URLs). Only
    // the owning provider or a platform admin (reviewer) may read one by id;
    // anyone else gets 404 so the row's existence isn't disclosed (PAP-140).
    if !user.is_platform_admin() {
        let provider_id = current_provider_id(&state, user.user_id).await?;
        if provider_id != verification.provider_id {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Verification {} not found", id),
                )),
            ));
        }
    }

    Ok(Json(verification))
}

/// Review a verification (admin only).
async fn review_verification(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ReviewVerificationRequest>,
) -> Result<Json<ProviderVerification>, (StatusCode, Json<ErrorResponse>)> {
    // Approving/rejecting a verification and awarding badges is platform
    // moderation, not an org-scoped action (PAP-140).
    require_platform_admin(&user)?;

    let verification = state
        .marketplace_repo
        .review_verification(
            id,
            user.user_id,
            &data.status,
            data.rejection_reason,
            data.notes,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to review verification");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to review verification",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Verification {} not found", id),
                )),
            )
        })?;

    // Award badges if specified
    if let Some(badges) = data.award_badges {
        for badge_type in badges {
            let _ = state
                .marketplace_repo
                .award_badge(
                    verification.provider_id,
                    user.user_id,
                    &badge_type,
                    Some(id),
                    None,
                    None,
                )
                .await;
        }
    }

    tracing::info!(
        verification_id = %id,
        status = %data.status,
        reviewed_by = %user.user_id,
        "Verification reviewed"
    );

    Ok(Json(verification))
}

/// List badges for a provider.
async fn list_provider_badges(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ProviderBadge>>, (StatusCode, Json<ErrorResponse>)> {
    let badges = state
        .marketplace_repo
        .list_provider_badges(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list badges");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list badges")),
            )
        })?;

    Ok(Json(badges))
}

/// Award a badge to a provider (admin only).
async fn award_badge(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<AwardBadgeRequest>,
) -> Result<(StatusCode, Json<ProviderBadge>), (StatusCode, Json<ErrorResponse>)> {
    // Badges are global provider-trust markers — platform moderation only (PAP-140).
    require_platform_admin(&user)?;

    let badge = state
        .marketplace_repo
        .award_badge(
            id,
            user.user_id,
            &data.badge_type,
            data.verification_id,
            data.expires_at,
            data.notes,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to award badge");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to award badge")),
            )
        })?;

    tracing::info!(
        badge_id = %badge.id,
        provider_id = %id,
        badge_type = %data.badge_type,
        awarded_by = %user.user_id,
        "Badge awarded"
    );

    Ok((StatusCode::CREATED, Json(badge)))
}

/// Revoke a badge.
async fn revoke_badge(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    require_platform_admin(&user)?;

    let revoked = state.marketplace_repo.revoke_badge(id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to revoke badge");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DB_ERROR", "Failed to revoke badge")),
        )
    })?;

    if revoked {
        tracing::info!(badge_id = %id, revoked_by = %user.user_id, "Badge revoked");
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                format!("Badge {} not found", id),
            )),
        ))
    }
}

// ==================== Story 68.5: Review Endpoints ====================

/// Create a review for a provider.
async fn create_review(
    State(state): State<AppState>,
    user: AuthUser,
    Path(provider_id): Path<Uuid>,
    Json(payload): Json<CreateReviewRequest>,
) -> Result<(StatusCode, Json<ProviderReview>), (StatusCode, Json<ErrorResponse>)> {
    // The review is stamped with `organization_id` from the body; verify the
    // caller actually belongs to that org so they can't post a review under
    // another tenant's name (PAP-140).
    verify_org_access(&state, user.user_id, payload.organization_id).await?;

    let review = state
        .marketplace_repo
        .create_review(
            provider_id,
            user.user_id,
            payload.organization_id,
            payload.data,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create review");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to create review")),
            )
        })?;

    tracing::info!(
        review_id = %review.id,
        provider_id = %provider_id,
        reviewer_id = %user.user_id,
        "Review created"
    );

    Ok((StatusCode::CREATED, Json(review)))
}

/// List reviews for a provider.
async fn list_provider_reviews(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<ProviderReviewWithResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let reviews = state
        .marketplace_repo
        .list_provider_reviews(id, clamp_limit(query.limit.map(i64::from), 20) as i32)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list reviews");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list reviews")),
            )
        })?;

    Ok(Json(reviews))
}

/// Get rating breakdown for a provider.
async fn get_rating_breakdown(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RatingBreakdown>, (StatusCode, Json<ErrorResponse>)> {
    let breakdown = state
        .marketplace_repo
        .get_rating_breakdown(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get rating breakdown");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    Ok(Json(breakdown))
}

/// List all reviews (with filters).
async fn list_reviews(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<ListReviewsQuery>,
) -> Result<Json<Vec<ProviderReview>>, (StatusCode, Json<ErrorResponse>)> {
    let review_query: ReviewQuery = (&query).into();

    let reviews = state
        .marketplace_repo
        .list_reviews(&review_query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list reviews");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to list reviews")),
            )
        })?;

    Ok(Json(reviews))
}

/// Get a specific review.
async fn get_review(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ProviderReview>, (StatusCode, Json<ErrorResponse>)> {
    let review = state
        .marketplace_repo
        .find_review_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get review");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Review {} not found", id),
                )),
            )
        })?;

    verify_org_access(&state, user.user_id, review.organization_id).await?;

    Ok(Json(review))
}

/// Update a review (by the reviewer).
async fn update_review(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateProviderReview>,
) -> Result<Json<ProviderReview>, (StatusCode, Json<ErrorResponse>)> {
    let review = state
        .marketplace_repo
        .update_review(id, user.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to update review");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to update review")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!(
                        "Review {} not found or you don't have permission to update it",
                        id
                    ),
                )),
            )
        })?;

    Ok(Json(review))
}

/// Delete a review.
async fn delete_review(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let deleted = state
        .marketplace_repo
        .delete_review(id, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to delete review");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to delete review")),
            )
        })?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new(
                "NOT_FOUND",
                format!(
                    "Review {} not found or you don't have permission to delete it",
                    id
                ),
            )),
        ))
    }
}

/// Respond to a review (by the provider).
async fn respond_to_review(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ProviderReviewResponse>,
) -> Result<Json<ProviderReview>, (StatusCode, Json<ErrorResponse>)> {
    // Get the provider profile for this user
    let provider = state
        .marketplace_repo
        .find_profile_by_user_id(user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to find provider profile");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::new(
                    "NOT_PROVIDER",
                    "You must have a provider profile to respond to reviews",
                )),
            )
        })?;

    let review = state
        .marketplace_repo
        .respond_to_review(id, provider.id, &data.response_text)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to respond to review");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DB_ERROR",
                    "Failed to respond to review",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Review {} not found or not for your provider profile", id),
                )),
            )
        })?;

    tracing::info!(review_id = %id, provider_id = %provider.id, "Provider responded to review");

    Ok(Json(review))
}

/// Moderate a review (admin only).
async fn moderate_review(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(data): Json<ModerateReviewRequest>,
) -> Result<Json<ProviderReview>, (StatusCode, Json<ErrorResponse>)> {
    // A review is moderated by a manager of the org that owns it. Load it first
    // and verify membership so a member of another tenant cannot moderate it.
    let existing = state
        .marketplace_repo
        .find_review_by_id(id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to load review for moderation");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Review {} not found", id),
                )),
            )
        })?;

    verify_org_access(&state, user.user_id, existing.organization_id).await?;

    let review = state
        .marketplace_repo
        .moderate_review(id, user.user_id, &data.status, data.moderation_notes)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to moderate review");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Failed to moderate review")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Review {} not found", id),
                )),
            )
        })?;

    tracing::info!(
        review_id = %id,
        status = %data.status,
        moderated_by = %user.user_id,
        "Review moderated"
    );

    Ok(Json(review))
}

/// Mark a review as helpful.
async fn mark_review_helpful(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<ProviderReview>, (StatusCode, Json<ErrorResponse>)> {
    let review = state
        .marketplace_repo
        .mark_review_helpful(id, user.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to mark review helpful");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "NOT_FOUND",
                    format!("Review {} not found", id),
                )),
            )
        })?;

    Ok(Json(review))
}

// ==================== Story 68.2: Manager Dashboard ====================

/// Get manager marketplace dashboard.
async fn get_manager_dashboard(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(query): Query<OrgQuery>,
) -> Result<Json<ManagerMarketplaceDashboard>, (StatusCode, Json<ErrorResponse>)> {
    let dashboard = state
        .marketplace_repo
        .get_manager_dashboard(query.organization_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get manager dashboard");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DB_ERROR", "Database error")),
            )
        })?;

    Ok(Json(dashboard))
}
