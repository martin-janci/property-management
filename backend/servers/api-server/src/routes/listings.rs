//! Listing routes (UC-31, Epic 15) - Real estate listing management.
//! Extended for Epic 105: Portal Syndication.

use crate::services::SyndicationService;
use crate::state::AppState;
use api_core::extractors::{RlsConnection, TenantExtractor};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use common::TenantRole;
use db::models::{
    listing_status, property_type as prop_type, AuditAction, CreateAuditLog, CreateListing,
    CreateListingFromUnit, CreateListingPhoto, CreateSyndication, Listing, ListingListQuery,
    ListingPhoto, ListingStatistics, ListingSummary, ListingSyndication, ListingWithDetails,
    OrganizationSyndicationStats, PublishListingResponse, ReorderPhotos, SyndicationDashboardQuery,
    SyndicationDashboardResponse, SyndicationResult, SyndicationStatusDashboard, UpdateListing,
    UpdateListingStatus,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

/// Create listings router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_listing))
        .route("/", get(list_listings))
        .route("/from-unit", post(create_from_unit))
        .route("/statistics", get(get_statistics))
        // Epic 105: Syndication dashboard routes
        .route("/syndication/dashboard", get(get_syndication_dashboard))
        .route(
            "/syndication/stats",
            get(get_organization_syndication_stats),
        )
        .route("/{id}", get(get_listing))
        .route("/{id}", put(update_listing))
        .route("/{id}", delete(delete_listing))
        .route("/{id}/status", put(update_status))
        .route("/{id}/publish", post(publish_listing))
        // Phase 4: Global Portal publish state (`is_published`).
        //
        // Distinct from `/{id}/publish` above (which creates external-portal
        // syndications and flips `status` to active — Epic 105 wiring). These
        // routes flip the platform-wide `is_published` flag that gates the
        // 4th RLS context (PlatformHost). Naming is `global-publish` /
        // `global-unpublish` to keep the difference unambiguous in logs and
        // SDKs. See ROADMAP.md Phase 4 and migrations 00135/00136.
        .route("/{id}/global-publish", post(global_publish))
        .route("/{id}/global-unpublish", post(global_unpublish))
        .route("/{id}/photos", get(get_photos))
        .route("/{id}/photos", post(add_photo))
        .route("/{id}/photos/reorder", post(reorder_photos))
        .route("/{id}/photos/{photo_id}", delete(delete_photo))
        .route("/{id}/syndications", get(get_syndications))
        // Epic 105: Per-listing syndication status
        .route(
            "/{id}/syndication/status",
            get(get_listing_syndication_status),
        )
}

/// Paginated listings response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListingsResponse {
    pub listings: Vec<ListingSummary>,
    pub total: i64,
    pub page: i32,
    pub limit: i32,
}

/// Unit data for pre-populating listing.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UnitListingData {
    pub unit_id: Uuid,
    pub building_id: Uuid,
    pub designation: String,
    pub unit_type: String,
    pub size_sqm: Option<Decimal>,
    pub rooms: Option<i32>,
    pub floor: i32,
    pub street: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub building_name: Option<String>,
    pub total_floors: i32,
}

/// Create a new listing (Story 15.1).
#[utoipa::path(
    post,
    path = "/api/v1/listings",
    tag = "Listings",
    request_body = CreateListing,
    responses(
        (status = 201, description = "Listing created", body = Listing),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_listing(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Json(data): Json<CreateListing>,
) -> Result<Json<Listing>, (axum::http::StatusCode, String)> {
    let listing = state
        .listing_repo
        .create(data, tenant.tenant_id, tenant.user_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create listing: {}", e),
            )
        })?;

    Ok(Json(listing))
}

/// Create a listing from existing unit data (Story 15.1).
#[utoipa::path(
    post,
    path = "/api/v1/listings/from-unit",
    tag = "Listings",
    request_body = CreateListingFromUnit,
    responses(
        (status = 201, description = "Listing created from unit", body = Listing),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Unit not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn create_from_unit(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    mut rls: RlsConnection,
    Json(data): Json<CreateListingFromUnit>,
) -> Result<Json<Listing>, (axum::http::StatusCode, String)> {
    // Fetch unit data
    let unit = match state
        .unit_repo
        .find_by_id_rls(&mut **rls.conn(), data.unit_id)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            rls.release().await;
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "Unit not found".to_string(),
            ));
        }
        Err(e) => {
            rls.release().await;
            return Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch unit: {}", e),
            ));
        }
    };

    // Fetch building data for address
    let building = match state
        .building_repo
        .find_by_id_rls(&mut **rls.conn(), unit.building_id)
        .await
    {
        Ok(Some(b)) => b,
        Ok(None) => {
            rls.release().await;
            return Err((
                axum::http::StatusCode::NOT_FOUND,
                "Building not found".to_string(),
            ));
        }
        Err(e) => {
            rls.release().await;
            return Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch building: {}", e),
            ));
        }
    };

    // Map unit_type to property_type
    let property_type = match unit.unit_type.as_str() {
        "apartment" => prop_type::APARTMENT,
        "commercial" => prop_type::COMMERCIAL,
        "parking" => prop_type::PARKING,
        "storage" => prop_type::STORAGE,
        _ => prop_type::OTHER,
    };

    // Generate title from unit data if not provided
    let title = data.title.unwrap_or_else(|| {
        let tx_type = if data.transaction_type == "sale" {
            "For Sale"
        } else {
            "For Rent"
        };
        format!(
            "{} {} - {} {}",
            unit.unit_type_display(),
            tx_type,
            building.street,
            building.city
        )
    });

    // Create listing with pre-populated data
    let create_data = CreateListing {
        unit_id: Some(data.unit_id),
        transaction_type: data.transaction_type,
        title,
        description: data.description,
        property_type: property_type.to_string(),
        size_sqm: unit.size_sqm,
        rooms: data.rooms.or(unit.rooms),
        bathrooms: data.bathrooms,
        floor: Some(unit.floor),
        total_floors: Some(building.total_floors),
        street: building.street,
        city: building.city,
        postal_code: building.postal_code,
        country: building.country,
        latitude: None,
        longitude: None,
        price: data.price,
        currency: data.currency,
        is_negotiable: data.is_negotiable,
        features: data.features,
    };

    let listing = match state
        .listing_repo
        .create(create_data, tenant.tenant_id, tenant.user_id)
        .await
    {
        Ok(l) => l,
        Err(e) => {
            rls.release().await;
            return Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create listing: {}", e),
            ));
        }
    };

    rls.release().await;
    Ok(Json(listing))
}

/// List listings for the organization.
#[utoipa::path(
    get,
    path = "/api/v1/listings",
    tag = "Listings",
    params(ListingListQuery),
    responses(
        (status = 200, description = "Listings list", body = ListingsResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn list_listings(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Query(query): Query<ListingListQuery>,
) -> Result<Json<ListingsResponse>, (axum::http::StatusCode, String)> {
    let listings = state
        .listing_repo
        .list(tenant.tenant_id, &query)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to list listings: {}", e),
            )
        })?;

    let total = state
        .listing_repo
        .count(tenant.tenant_id, &query)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to count listings: {}", e),
            )
        })?;

    Ok(Json(ListingsResponse {
        listings,
        total,
        page: query.page.unwrap_or(1),
        limit: query.limit.unwrap_or(20),
    }))
}

/// Get listing by ID.
#[utoipa::path(
    get,
    path = "/api/v1/listings/{id}",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Listing details", body = ListingWithDetails),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_listing(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<ListingWithDetails>, (axum::http::StatusCode, String)> {
    let listing = state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    let photos = state.listing_repo.get_photos(id).await.unwrap_or_default();
    let syndications = state
        .listing_repo
        .get_syndications(id)
        .await
        .unwrap_or_default();

    Ok(Json(ListingWithDetails {
        listing,
        photos,
        syndications,
    }))
}

/// Update a listing.
#[utoipa::path(
    put,
    path = "/api/v1/listings/{id}",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    request_body = UpdateListing,
    responses(
        (status = 200, description = "Listing updated", body = Listing),
        (status = 400, description = "Cannot update listing in current status"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn update_listing(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateListing>,
) -> Result<Json<Listing>, (axum::http::StatusCode, String)> {
    // Check listing exists and can be edited
    let existing = state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    if !existing.can_edit() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Cannot edit listing in '{}' status", existing.status),
        ));
    }

    let listing = state
        .listing_repo
        .update(id, tenant.tenant_id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update listing: {}", e),
            )
        })?;

    Ok(Json(listing))
}

/// Delete a listing.
#[utoipa::path(
    delete,
    path = "/api/v1/listings/{id}",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 204, description = "Listing deleted"),
        (status = 400, description = "Cannot delete active listing"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_listing(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Check listing exists
    let existing = state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    // Only allow deleting draft listings
    if existing.status != listing_status::DRAFT {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Cannot delete non-draft listing. Archive it instead.".to_string(),
        ));
    }

    state
        .listing_repo
        .delete(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete listing: {}", e),
            )
        })?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Update listing status (Story 15.4, Epic 105: Status Propagation).
#[utoipa::path(
    put,
    path = "/api/v1/listings/{id}/status",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    request_body = UpdateListingStatus,
    responses(
        (status = 200, description = "Status updated", body = Listing),
        (status = 400, description = "Invalid status transition"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn update_status(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<UpdateListingStatus>,
) -> Result<Json<Listing>, (axum::http::StatusCode, String)> {
    // Validate status value
    let valid_statuses = [
        listing_status::DRAFT,
        listing_status::ACTIVE,
        listing_status::PAUSED,
        listing_status::SOLD,
        listing_status::RENTED,
        listing_status::ARCHIVED,
    ];

    if !valid_statuses.contains(&data.status.as_str()) {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Invalid status: {}", data.status),
        ));
    }

    // Get current listing to track previous status
    let existing = state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    let previous_status = existing.status.clone();
    let new_status = data.status.clone();

    let listing = state
        .listing_repo
        .update_status(id, tenant.tenant_id, data)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update status: {}", e),
            )
        })?;

    // Epic 105, Story 105.2: Propagate status change to syndicated portals
    if previous_status != new_status {
        let syndication_service = SyndicationService::new(
            state.background_job_repo.clone(),
            state.listing_repo.clone(),
        );

        if let Err(e) = syndication_service
            .create_status_change_jobs(
                &listing,
                &previous_status,
                &new_status,
                Some(tenant.user_id),
            )
            .await
        {
            tracing::warn!(
                listing_id = %id,
                previous_status = %previous_status,
                new_status = %new_status,
                error = %e,
                "Failed to create status propagation jobs"
            );
            // Don't fail the status update if job creation fails
        }
    }

    Ok(Json(listing))
}

/// Publish listing to portals (Story 15.3, Epic 105: Syndication Jobs).
#[utoipa::path(
    post,
    path = "/api/v1/listings/{id}/publish",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    request_body = CreateSyndication,
    responses(
        (status = 200, description = "Listing published", body = PublishListingResponse),
        (status = 400, description = "Cannot publish listing"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn publish_listing(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateSyndication>,
) -> Result<Json<PublishListingResponse>, (axum::http::StatusCode, String)> {
    // Check listing exists and can be published
    let existing = state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    if !existing.can_publish() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            format!("Cannot publish listing in '{}' status", existing.status),
        ));
    }

    // Update status to active
    let listing = state
        .listing_repo
        .update_status(
            id,
            tenant.tenant_id,
            UpdateListingStatus {
                status: listing_status::ACTIVE.to_string(),
            },
        )
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to update status: {}", e),
            )
        })?;

    // Create syndications
    let syndications = state
        .listing_repo
        .create_syndications(id, data.clone())
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create syndications: {}", e),
            )
        })?;

    // Build syndication results (initially all pending)
    let syndication_results: Vec<SyndicationResult> = syndications
        .iter()
        .map(|s| SyndicationResult {
            portal: s.portal.clone(),
            success: true,
            external_id: None,
            error: None,
        })
        .collect();

    // Epic 105, Story 105.1: Create async syndication jobs for each portal
    let syndication_service = SyndicationService::new(
        state.background_job_repo.clone(),
        state.listing_repo.clone(),
    );

    if let Err(e) = syndication_service
        .create_publish_jobs(&listing, &data.portals, Some(tenant.user_id))
        .await
    {
        tracing::warn!(
            listing_id = %id,
            portals = ?data.portals,
            error = %e,
            "Failed to create syndication publish jobs"
        );
        // Don't fail the publish if job creation fails - syndications are already created
    }

    Ok(Json(PublishListingResponse {
        listing_id: id,
        status: listing.status,
        syndication_results,
    }))
}

/// Get statistics for organization's listings.
#[utoipa::path(
    get,
    path = "/api/v1/listings/statistics",
    tag = "Listings",
    responses(
        (status = 200, description = "Listing statistics", body = ListingStatistics),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_statistics(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
) -> Result<Json<ListingStatistics>, (axum::http::StatusCode, String)> {
    let stats = state
        .listing_repo
        .get_statistics(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get statistics: {}", e),
            )
        })?;

    Ok(Json(stats))
}

// ============================================
// Photo Management (Story 15.2)
// ============================================

/// Get photos for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/listings/{id}/photos",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Listing photos", body = Vec<ListingPhoto>),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_photos(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ListingPhoto>>, (axum::http::StatusCode, String)> {
    // Verify listing exists and belongs to org
    state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    let photos = state.listing_repo.get_photos(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get photos: {}", e),
        )
    })?;

    Ok(Json(photos))
}

/// Add a photo to a listing.
#[utoipa::path(
    post,
    path = "/api/v1/listings/{id}/photos",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    request_body = CreateListingPhoto,
    responses(
        (status = 201, description = "Photo added", body = ListingPhoto),
        (status = 400, description = "Cannot add photos to listing"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn add_photo(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<CreateListingPhoto>,
) -> Result<Json<ListingPhoto>, (axum::http::StatusCode, String)> {
    // Verify listing exists and can be edited
    let listing = state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    if !listing.can_edit() {
        return Err((
            axum::http::StatusCode::BAD_REQUEST,
            "Cannot add photos to listing in current status".to_string(),
        ));
    }

    let photo = state.listing_repo.add_photo(id, data).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to add photo: {}", e),
        )
    })?;

    Ok(Json(photo))
}

/// Reorder photos for a listing.
#[utoipa::path(
    post,
    path = "/api/v1/listings/{id}/photos/reorder",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    request_body = ReorderPhotos,
    responses(
        (status = 200, description = "Photos reordered"),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn reorder_photos(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
    Json(data): Json<ReorderPhotos>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Verify listing exists
    state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    state
        .listing_repo
        .reorder_photos(id, data.photo_ids)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to reorder photos: {}", e),
            )
        })?;

    Ok(axum::http::StatusCode::OK)
}

/// Delete a photo from a listing.
#[utoipa::path(
    delete,
    path = "/api/v1/listings/{id}/photos/{photo_id}",
    tag = "Listings",
    params(
        ("id" = Uuid, Path, description = "Listing ID"),
        ("photo_id" = Uuid, Path, description = "Photo ID")
    ),
    responses(
        (status = 204, description = "Photo deleted"),
        (status = 404, description = "Listing or photo not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn delete_photo(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path((id, photo_id)): Path<(Uuid, Uuid)>,
) -> Result<axum::http::StatusCode, (axum::http::StatusCode, String)> {
    // Verify listing exists
    state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    let deleted = state
        .listing_repo
        .delete_photo(photo_id, id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete photo: {}", e),
            )
        })?;

    if !deleted {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            "Photo not found".to_string(),
        ));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Get syndications for a listing.
#[utoipa::path(
    get,
    path = "/api/v1/listings/{id}/syndications",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Listing syndications", body = Vec<ListingSyndication>),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_syndications(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ListingSyndication>>, (axum::http::StatusCode, String)> {
    // Verify listing exists
    state
        .listing_repo
        .find_by_id_and_org(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    let syndications = state.listing_repo.get_syndications(id).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to get syndications: {}", e),
        )
    })?;

    Ok(Json(syndications))
}

// ============================================
// Epic 105: Syndication Dashboard Endpoints
// ============================================

/// Get syndication status for a specific listing (Story 105.3).
#[utoipa::path(
    get,
    path = "/api/v1/listings/{id}/syndication/status",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Syndication status", body = SyndicationStatusDashboard),
        (status = 404, description = "Listing not found"),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_listing_syndication_status(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<SyndicationStatusDashboard>, (axum::http::StatusCode, String)> {
    let dashboard = state
        .listing_repo
        .get_syndication_status_dashboard(id, tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get syndication status: {}", e),
            )
        })?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Listing not found".to_string(),
            )
        })?;

    Ok(Json(dashboard))
}

/// Get syndication dashboard for organization (Story 105.3).
#[utoipa::path(
    get,
    path = "/api/v1/listings/syndication/dashboard",
    tag = "Listings",
    params(SyndicationDashboardQuery),
    responses(
        (status = 200, description = "Syndication dashboard", body = SyndicationDashboardResponse),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_syndication_dashboard(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Query(query): Query<SyndicationDashboardQuery>,
) -> Result<Json<SyndicationDashboardResponse>, (axum::http::StatusCode, String)> {
    let (listings, total) = state
        .listing_repo
        .get_syndication_dashboard(tenant.tenant_id, &query)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get syndication dashboard: {}", e),
            )
        })?;

    let organization_stats = state
        .listing_repo
        .get_organization_syndication_stats(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get organization stats: {}", e),
            )
        })?;

    Ok(Json(SyndicationDashboardResponse {
        listings,
        total,
        page: query.page.unwrap_or(1),
        limit: query.limit.unwrap_or(20),
        organization_stats,
    }))
}

/// Get organization-wide syndication statistics (Story 105.3).
#[utoipa::path(
    get,
    path = "/api/v1/listings/syndication/stats",
    tag = "Listings",
    responses(
        (status = 200, description = "Organization syndication statistics", body = OrganizationSyndicationStats),
        (status = 401, description = "Unauthorized")
    )
)]
pub async fn get_organization_syndication_stats(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
) -> Result<Json<OrganizationSyndicationStats>, (axum::http::StatusCode, String)> {
    let stats = state
        .listing_repo
        .get_organization_syndication_stats(tenant.tenant_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get syndication stats: {}", e),
            )
        })?;

    Ok(Json(stats))
}

// ============================================================================
// Phase 4: Global Portal Publish State (`is_published`)
// ============================================================================
//
// These two endpoints flip the `is_published` flag that opens the 4th RLS
// context (PlatformHost / global-read across all orgs — see migrations 00135
// and 00136). They are deliberately separate from `/{id}/publish` (which
// queues external-portal syndication jobs — Epic 105).
//
// Auth (closes #851, #809): these handlers use the same `TenantExtractor`
// every other listings handler uses (consistent org/role extraction from the
// JWT), and gate the global-publish action behind manager role — publishing a
// listing to the cross-org global portal is a management action, not something
// any org member (e.g. a resident) may do. The repository UPDATE remains
// org-scoped (UPDATE WHERE organization_id = $caller_org) as a second
// defensive layer against cross-tenant writes.

/// Gate global publish/unpublish to manager-tier roles (issue #809).
///
/// The cross-tenant IDOR is already closed by the org-scoped repository UPDATE
/// (`WHERE organization_id = $caller_org`); this adds the server-derived role
/// check the audit flagged so a non-manager org member cannot push a listing
/// onto the public reality portal. Returns `403` when the caller is not a
/// manager in `org_id`. Full capability gating still lands in Phase 5.
async fn require_listing_publisher(
    state: &AppState,
    user_id: Uuid,
    org_id: Uuid,
) -> Result<(), (axum::http::StatusCode, String)> {
    let is_manager = db::repositories::MembershipRepository::new(state.db.clone())
        .is_manager_in_org(user_id, org_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to verify role: {}", e),
            )
        })?;
    if !is_manager {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "Manager role required to publish to the global portal".to_string(),
        ));
    }
    Ok(())
}

/// Mark a listing as published to the global reality portal.
///
/// Sets `is_published = TRUE` and `published_at = NOW()`. The listing then
/// appears in queries served by the platform-host (PlatformHost) connection
/// context, alongside published listings from every other agency.
#[utoipa::path(
    post,
    path = "/api/v1/listings/{id}/global-publish",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Listing published to global portal", body = Listing),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Manager role required, or listing belongs to another organization"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn global_publish(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<Listing>, (axum::http::StatusCode, String)> {
    // Authz (closes #809): publishing to the cross-org global portal is a
    // manager action. `has_role` is hierarchical, so OrgAdmin / PlatformAdmin /
    // SuperAdmin (all above Manager) also pass.
    if !tenant.has_role(TenantRole::Manager) {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "Manager role required to publish listings to the global portal".to_string(),
        ));
    }
    let org_id = tenant.tenant_id;

    // SECURITY (issue #809): publishing to the global reality portal exposes the
    // listing across every org, so gate it to manager-tier roles (server-derived
    // from `user_memberships`) rather than any authenticated org member. Full
    // capability gating (`Capability::ListingsPublish`) still lands in Phase 5.
    require_listing_publisher(&state, auth.user_id, org_id).await?;

    let previous = state
        .listing_repo
        .find_by_id_and_org(id, org_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Listing not found".to_string(),
        ))?;

    let listing = state
        .listing_repo
        .set_published(id, org_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to publish listing: {}", e),
            )
        })?
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Listing not found".to_string(),
        ))?;

    // Audit. Best-effort: a logging failure must not surface a 500 on the
    // happy path (the publish itself succeeded). We log but don't propagate.
    let audit_result = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(tenant.user_id),
            action: AuditAction::ResourceUpdated,
            resource_type: Some("listing".to_string()),
            resource_id: Some(id),
            org_id: Some(org_id),
            details: Some(json!({
                "operation": "global_publish",
                "phase": "phase-4",
            })),
            old_values: Some(json!({
                "is_published": previous.is_published,
                "published_at": previous.published_at,
            })),
            new_values: Some(json!({
                "is_published": listing.is_published,
                "published_at": listing.published_at,
            })),
            ip_address: None,
            user_agent: None,
        })
        .await;
    if let Err(e) = audit_result {
        tracing::warn!(
            error = %e,
            listing_id = %id,
            org_id = %org_id,
            "Failed to write audit log for global_publish"
        );
    }

    Ok(Json(listing))
}

/// Withdraw a listing from the global reality portal.
///
/// Sets `is_published = FALSE` and clears `published_at`. The listing row
/// itself is preserved on the agency's own subdomain (invariant I-D — a
/// listing exists once; un-publishing only flips the global-read OR-clause
/// off).
#[utoipa::path(
    post,
    path = "/api/v1/listings/{id}/global-unpublish",
    tag = "Listings",
    params(("id" = Uuid, Path, description = "Listing ID")),
    responses(
        (status = 200, description = "Listing withdrawn from global portal", body = Listing),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Manager role required, or listing belongs to another organization"),
        (status = 404, description = "Listing not found")
    )
)]
pub async fn global_unpublish(
    State(state): State<AppState>,
    TenantExtractor(tenant): TenantExtractor,
    Path(id): Path<Uuid>,
) -> Result<Json<Listing>, (axum::http::StatusCode, String)> {
    // Authz (closes #809): withdrawing from the global portal is the inverse
    // management action and is gated identically to `global_publish`.
    if !tenant.has_role(TenantRole::Manager) {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            "Manager role required to withdraw listings from the global portal".to_string(),
        ));
    }
    let org_id = tenant.tenant_id;

    // SECURITY (issue #809): manager-tier gate (see `require_listing_publisher`).
    require_listing_publisher(&state, auth.user_id, org_id).await?;

    let previous = state
        .listing_repo
        .find_by_id_and_org(id, org_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to fetch listing: {}", e),
            )
        })?
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Listing not found".to_string(),
        ))?;

    let listing = state
        .listing_repo
        .set_unpublished(id, org_id)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to unpublish listing: {}", e),
            )
        })?
        .ok_or((
            axum::http::StatusCode::NOT_FOUND,
            "Listing not found".to_string(),
        ))?;

    let audit_result = state
        .audit_log_repo
        .create(CreateAuditLog {
            user_id: Some(tenant.user_id),
            action: AuditAction::ResourceUpdated,
            resource_type: Some("listing".to_string()),
            resource_id: Some(id),
            org_id: Some(org_id),
            details: Some(json!({
                "operation": "global_unpublish",
                "phase": "phase-4",
            })),
            old_values: Some(json!({
                "is_published": previous.is_published,
                "published_at": previous.published_at,
            })),
            new_values: Some(json!({
                "is_published": listing.is_published,
                "published_at": listing.published_at,
            })),
            ip_address: None,
            user_agent: None,
        })
        .await;
    if let Err(e) = audit_result {
        tracing::warn!(
            error = %e,
            listing_id = %id,
            org_id = %org_id,
            "Failed to write audit log for global_unpublish"
        );
    }

    Ok(Json(listing))
}
