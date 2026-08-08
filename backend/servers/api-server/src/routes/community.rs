//! Community & Social routes (Epic 37).
//!
//! Routes for community groups, posts, events, and marketplace.

use api_core::extractors::principal::RequestPrincipal;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use common::errors::ErrorResponse;
use db::models::{
    CommunityComment, CommunityEvent, CommunityEventRsvp, CommunityGroup,
    CommunityGroupWithMembership, CommunityPost, CreateCommunityComment, CreateCommunityEvent,
    CreateCommunityGroup, CreateCommunityPost, CreateMarketplaceInquiry, CreateMarketplaceItem,
    EventRsvpRequest, MarketplaceInquiry, MarketplaceItem,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::state::AppState;

// ============================================================================
// Helper Functions
// ============================================================================
//
// SECURITY: The previous `extract_tenant_context` helper deserialized the
// client-supplied `X-Tenant-Context` JSON header directly into a
// `TenantContext`. No JWT verification — any unauthenticated caller could
// forge tenancy. That helper has been deleted; every handler now goes
// through `RequestPrincipal` (verified bearer JWT + host-resolved tenant).
//
// P1-02 (dev-team review): handlers below take `building_id` from the
// URL path. Without an authorization check, any authenticated tenant of
// Org A could POST against `/api/v1/community/buildings/<orgB-building>/...`
// and either read or create rows under another tenant's building. The
// `verify_building_access` helper rejects that: it loads the building,
// confirms it exists, then confirms the principal has an active
// membership in the building's owning organization. Platform principals
// bypass the membership check.

/// Reject the request if the principal cannot access the given
/// `building_id`. Returns the building's organization_id on success so
/// the handler can use it without a second lookup.
async fn verify_building_access(
    state: &AppState,
    principal: &RequestPrincipal,
    building_id: Uuid,
) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    use db::models::PrincipalKind;
    use db::repositories::MembershipRepository;

    // We intentionally use the non-RLS lookup here: this IS the access
    // check, run before any RLS context is established for the request.
    #[allow(deprecated)]
    let building = state
        .building_repo
        .find_by_id(building_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Building lookup failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to verify building access",
                )),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new(
                    "BUILDING_NOT_FOUND",
                    "Building not found",
                )),
            )
        })?;

    if matches!(principal.kind, PrincipalKind::Platform) {
        return Ok(building.organization_id);
    }

    let has_access = MembershipRepository::new(state.db.clone())
        .is_active(principal.user_id, building.organization_id)
        .await
        .unwrap_or(false);
    if !has_access {
        return Err((
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new(
                "BUILDING_ACCESS_DENIED",
                "Caller is not a member of the building's organization",
            )),
        ));
    }
    Ok(building.organization_id)
}

// SECURITY (cross-tenant IDOR — code-review api-handlers/community): the five
// write handlers that take a *resource* id from the URL path (`group_id`,
// `post_id`, `event_id`, `item_id`) — `create_post`, `add_reaction`,
// `create_comment`, `rsvp_event`, `create_inquiry` — used to bind that id
// straight into the repository write with no tenant check. Any authenticated
// user of org A could POST into org B's group/post/event/item by guessing an
// id. The `verify_*_access` helpers below close that gap the same way the
// building-id routes use `verify_building_access` and the disputes IDOR fix
// used `*_for_org`: resolve the resource scoped to the caller's JWT-derived
// org and reject a miss with **404** (not 403) so the id space cannot be
// enumerated as an existence oracle.

/// Derive the caller's organization from the verified principal.
///
/// The scope MUST come from `RequestPrincipal::effective_org` (JWT + host
/// resolved, server-side) — never a path- or body-supplied org — so a write
/// cannot cross tenants. Absent org context (e.g. a platform principal on the
/// platform host) is a 403, matching the disputes IDOR fix.
fn require_org(principal: &RequestPrincipal) -> Result<Uuid, (StatusCode, Json<ErrorResponse>)> {
    principal.effective_org.ok_or_else(|| {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse::new("FORBIDDEN", "No organization context")),
        )
    })
}

/// 500 for a tenant-scoping lookup that errored at the DB.
fn tenant_lookup_error() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::new(
            "DATABASE_ERROR",
            "Failed to verify tenant access",
        )),
    )
}

/// 404 for a resource that is absent OR owned by another tenant. Uses the same
/// opaque body for both cases so the id space is not an existence oracle.
fn resource_not_found(what: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::new("NOT_FOUND", what)),
    )
}

/// Reject the request unless `group_id` belongs to the caller's org.
async fn verify_group_access(
    state: &AppState,
    principal: &RequestPrincipal,
    group_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org(principal)?;
    let exists = state
        .community_repo
        .group_exists_for_org(group_id, org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Group tenant lookup failed");
            tenant_lookup_error()
        })?;
    if !exists {
        return Err(resource_not_found("Group not found"));
    }
    Ok(())
}

/// Reject the request unless `post_id`'s group belongs to the caller's org.
async fn verify_post_access(
    state: &AppState,
    principal: &RequestPrincipal,
    post_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org(principal)?;
    let exists = state
        .community_repo
        .post_exists_for_org(post_id, org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Post tenant lookup failed");
            tenant_lookup_error()
        })?;
    if !exists {
        return Err(resource_not_found("Post not found"));
    }
    Ok(())
}

/// Reject the request unless `event_id`'s building belongs to the caller's org.
async fn verify_event_access(
    state: &AppState,
    principal: &RequestPrincipal,
    event_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org(principal)?;
    let exists = state
        .community_repo
        .event_exists_for_org(event_id, org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Event tenant lookup failed");
            tenant_lookup_error()
        })?;
    if !exists {
        return Err(resource_not_found("Event not found"));
    }
    Ok(())
}

/// Reject the request unless `item_id`'s building belongs to the caller's org.
async fn verify_item_access(
    state: &AppState,
    principal: &RequestPrincipal,
    item_id: Uuid,
) -> Result<(), (StatusCode, Json<ErrorResponse>)> {
    let org_id = require_org(principal)?;
    let exists = state
        .community_repo
        .item_exists_for_org(item_id, org_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Marketplace item tenant lookup failed");
            tenant_lookup_error()
        })?;
    if !exists {
        return Err(resource_not_found("Item not found"));
    }
    Ok(())
}

/// Create community router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Groups (Story 37.1)
        .route("/buildings/{building_id}/groups", get(list_groups))
        .route("/buildings/{building_id}/groups", post(create_group))
        .route("/groups/{id}", get(get_group))
        .route("/groups/{id}/join", post(join_group))
        .route("/groups/{id}/leave", post(leave_group))
        // Posts (Story 37.2)
        .route("/groups/{group_id}/posts", get(list_posts))
        .route("/groups/{group_id}/posts", post(create_post))
        .route("/posts/{id}/reactions", post(add_reaction))
        .route("/posts/{id}/comments", post(create_comment))
        // Events (Story 37.3)
        .route("/buildings/{building_id}/events", get(list_events))
        .route("/buildings/{building_id}/events", post(create_event))
        .route("/events/{id}/rsvp", post(rsvp_event))
        // Marketplace (Story 37.4)
        .route("/buildings/{building_id}/marketplace", get(list_items))
        .route("/buildings/{building_id}/marketplace", post(create_item))
        .route("/marketplace/{id}", get(get_item))
        .route("/marketplace/{id}/inquiries", post(create_inquiry))
}

// ==================== Types ====================

/// Building ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct BuildingIdPath {
    pub building_id: Uuid,
}

/// Group ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GroupIdPath {
    pub id: Uuid,
}

/// Group ID path for posts.
#[derive(Debug, Deserialize, IntoParams)]
pub struct GroupPostsPath {
    pub group_id: Uuid,
}

/// Post ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct PostIdPath {
    pub id: Uuid,
}

/// Item ID path parameter.
#[derive(Debug, Deserialize, IntoParams)]
pub struct ItemIdPath {
    pub id: Uuid,
}

/// Pagination query.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct PaginationQuery {
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_limit() -> i32 {
    20
}

/// List groups query.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListGroupsQuery {
    pub user_id: Option<Uuid>,
}

/// Marketplace filter query.
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct MarketplaceQuery {
    pub category: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

/// Add reaction request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddReactionRequest {
    pub reaction_type: String,
}

// ==================== Groups (Story 37.1) ====================

/// List community groups for a building.
#[utoipa::path(
    get,
    path = "/api/v1/community/buildings/{building_id}/groups",
    params(BuildingIdPath, ListGroupsQuery),
    responses(
        (status = 200, description = "Groups retrieved", body = Vec<CommunityGroupWithMembership>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn list_groups(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<BuildingIdPath>,
    Query(query): Query<ListGroupsQuery>,
) -> Result<Json<Vec<CommunityGroupWithMembership>>, (StatusCode, Json<ErrorResponse>)> {
    verify_building_access(&state, &principal, path.building_id).await?;
    // P1-02: query.user_id was previously honored verbatim from the
    // query string, which let a caller infer another user's group
    // membership. Use the verified principal's user_id instead.
    let _ = query.user_id; // intentionally unused
    let groups = state
        .community_repo
        .list_groups(path.building_id, Some(principal.user_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list groups");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list groups",
                )),
            )
        })?;

    Ok(Json(groups))
}

/// Create a community group.
#[utoipa::path(
    post,
    path = "/api/v1/community/buildings/{building_id}/groups",
    params(BuildingIdPath),
    request_body = CreateCommunityGroup,
    responses(
        (status = 201, description = "Group created", body = CommunityGroup),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn create_group(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<BuildingIdPath>,
    Json(data): Json<CreateCommunityGroup>,
) -> Result<(StatusCode, Json<CommunityGroup>), (StatusCode, Json<ErrorResponse>)> {
    verify_building_access(&state, &principal, path.building_id).await?;
    let group = state
        .community_repo
        .create_group(path.building_id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create group");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create group",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(group)))
}

/// Get a community group by ID.
#[utoipa::path(
    get,
    path = "/api/v1/community/groups/{id}",
    params(GroupIdPath),
    responses(
        (status = 200, description = "Group retrieved", body = CommunityGroup),
        (status = 404, description = "Group not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn get_group(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<GroupIdPath>,
) -> Result<Json<CommunityGroup>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: without RequestPrincipal this read ran unauthenticated and
    // leaked any tenant's group by UUID. Require a verified principal and
    // reject a cross-tenant id with 404 (via verify_group_access).
    verify_group_access(&state, &principal, path.id).await?;
    let group = state.community_repo.get_group(path.id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get group");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get group")),
        )
    })?;

    match group {
        Some(g) => Ok(Json(g)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Group not found")),
        )),
    }
}

/// Join a community group.
#[utoipa::path(
    post,
    path = "/api/v1/community/groups/{id}/join",
    params(GroupIdPath),
    responses(
        (status = 200, description = "Joined group"),
        (status = 404, description = "Group not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn join_group(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<GroupIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    verify_group_access(&state, &principal, path.id).await?;

    state
        .community_repo
        .join_group(path.id, principal.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to join group");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to join group")),
            )
        })?;

    Ok(StatusCode::OK)
}

/// Leave a community group.
#[utoipa::path(
    post,
    path = "/api/v1/community/groups/{id}/leave",
    params(GroupIdPath),
    responses(
        (status = 200, description = "Left group"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn leave_group(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<GroupIdPath>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    verify_group_access(&state, &principal, path.id).await?;

    state
        .community_repo
        .leave_group(path.id, principal.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to leave group");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to leave group",
                )),
            )
        })?;

    Ok(StatusCode::OK)
}

// ==================== Posts (Story 37.2) ====================

/// List posts in a group.
#[utoipa::path(
    get,
    path = "/api/v1/community/groups/{group_id}/posts",
    params(GroupPostsPath, PaginationQuery),
    responses(
        (status = 200, description = "Posts retrieved", body = Vec<CommunityPost>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn list_posts(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<GroupPostsPath>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<CommunityPost>>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: without RequestPrincipal this read ran unauthenticated and
    // leaked another tenant's posts by guessing a group UUID. Verify the
    // group belongs to the caller's org (404 on a cross-tenant miss) before
    // returning any of its posts.
    verify_group_access(&state, &principal, path.group_id).await?;
    let posts = state
        .community_repo
        .get_group_posts(path.group_id, query.limit, query.offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list posts");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to list posts")),
            )
        })?;

    Ok(Json(posts))
}

/// Create a post in a group.
#[utoipa::path(
    post,
    path = "/api/v1/community/groups/{group_id}/posts",
    params(GroupPostsPath),
    request_body = CreateCommunityPost,
    responses(
        (status = 201, description = "Post created", body = CommunityPost),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn create_post(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<GroupPostsPath>,
    Json(data): Json<CreateCommunityPost>,
) -> Result<(StatusCode, Json<CommunityPost>), (StatusCode, Json<ErrorResponse>)> {
    verify_group_access(&state, &principal, path.group_id).await?;
    let post = state
        .community_repo
        .create_post(path.group_id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create post");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create post",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(post)))
}

/// Add reaction to a post.
#[utoipa::path(
    post,
    path = "/api/v1/community/posts/{id}/reactions",
    params(PostIdPath),
    request_body = AddReactionRequest,
    responses(
        (status = 200, description = "Reaction added"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn add_reaction(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<PostIdPath>,
    Json(data): Json<AddReactionRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    verify_post_access(&state, &principal, path.id).await?;
    state
        .community_repo
        .add_post_reaction(path.id, principal.user_id, &data.reaction_type)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to add reaction");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to add reaction",
                )),
            )
        })?;

    Ok(StatusCode::OK)
}

/// Create a comment on a post.
#[utoipa::path(
    post,
    path = "/api/v1/community/posts/{id}/comments",
    params(PostIdPath),
    request_body = CreateCommunityComment,
    responses(
        (status = 201, description = "Comment created", body = CommunityComment),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn create_comment(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<PostIdPath>,
    Json(data): Json<CreateCommunityComment>,
) -> Result<(StatusCode, Json<CommunityComment>), (StatusCode, Json<ErrorResponse>)> {
    verify_post_access(&state, &principal, path.id).await?;
    let comment = state
        .community_repo
        .create_comment(path.id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create comment");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create comment",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(comment)))
}

// ==================== Events (Story 37.3) ====================

/// List upcoming events for a building.
#[utoipa::path(
    get,
    path = "/api/v1/community/buildings/{building_id}/events",
    params(BuildingIdPath, PaginationQuery),
    responses(
        (status = 200, description = "Events retrieved", body = Vec<CommunityEvent>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn list_events(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<BuildingIdPath>,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<Vec<CommunityEvent>>, (StatusCode, Json<ErrorResponse>)> {
    verify_building_access(&state, &principal, path.building_id).await?;
    let events = state
        .community_repo
        .get_upcoming_events(path.building_id, query.limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list events");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to list events",
                )),
            )
        })?;

    Ok(Json(events))
}

/// Create a community event.
#[utoipa::path(
    post,
    path = "/api/v1/community/buildings/{building_id}/events",
    params(BuildingIdPath),
    request_body = CreateCommunityEvent,
    responses(
        (status = 201, description = "Event created", body = CommunityEvent),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn create_event(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<BuildingIdPath>,
    Json(data): Json<CreateCommunityEvent>,
) -> Result<(StatusCode, Json<CommunityEvent>), (StatusCode, Json<ErrorResponse>)> {
    verify_building_access(&state, &principal, path.building_id).await?;
    let event = state
        .community_repo
        .create_event(path.building_id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create event");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create event",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(event)))
}

/// RSVP to an event.
#[utoipa::path(
    post,
    path = "/api/v1/community/events/{id}/rsvp",
    params(("id" = Uuid, Path, description = "Event ID")),
    request_body = EventRsvpRequest,
    responses(
        (status = 200, description = "RSVP recorded", body = CommunityEventRsvp),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn rsvp_event(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(id): Path<Uuid>,
    Json(data): Json<EventRsvpRequest>,
) -> Result<Json<CommunityEventRsvp>, (StatusCode, Json<ErrorResponse>)> {
    verify_event_access(&state, &principal, id).await?;
    let rsvp = state
        .community_repo
        .rsvp_event(id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to RSVP to event");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to RSVP to event",
                )),
            )
        })?;

    Ok(Json(rsvp))
}

// ==================== Marketplace (Story 37.4) ====================

/// List marketplace items for a building.
#[utoipa::path(
    get,
    path = "/api/v1/community/buildings/{building_id}/marketplace",
    params(BuildingIdPath, MarketplaceQuery),
    responses(
        (status = 200, description = "Items retrieved", body = Vec<MarketplaceItem>),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn list_items(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<BuildingIdPath>,
    Query(query): Query<MarketplaceQuery>,
) -> Result<Json<Vec<MarketplaceItem>>, (StatusCode, Json<ErrorResponse>)> {
    verify_building_access(&state, &principal, path.building_id).await?;
    let items = state
        .community_repo
        .list_items(path.building_id, query.category, query.limit, query.offset)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to list items");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new("DATABASE_ERROR", "Failed to list items")),
            )
        })?;

    Ok(Json(items))
}

/// Create a marketplace item.
#[utoipa::path(
    post,
    path = "/api/v1/community/buildings/{building_id}/marketplace",
    params(BuildingIdPath),
    request_body = CreateMarketplaceItem,
    responses(
        (status = 201, description = "Item created", body = MarketplaceItem),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn create_item(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<BuildingIdPath>,
    Json(data): Json<CreateMarketplaceItem>,
) -> Result<(StatusCode, Json<MarketplaceItem>), (StatusCode, Json<ErrorResponse>)> {
    verify_building_access(&state, &principal, path.building_id).await?;
    let item = state
        .community_repo
        .create_item(path.building_id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create item");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create item",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(item)))
}

/// Get a marketplace item by ID.
#[utoipa::path(
    get,
    path = "/api/v1/community/marketplace/{id}",
    params(ItemIdPath),
    responses(
        (status = 200, description = "Item retrieved", body = MarketplaceItem),
        (status = 404, description = "Item not found"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn get_item(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<ItemIdPath>,
) -> Result<Json<MarketplaceItem>, (StatusCode, Json<ErrorResponse>)> {
    // SECURITY: without RequestPrincipal this read ran unauthenticated and
    // leaked any tenant's marketplace item by UUID. Require a verified
    // principal and reject a cross-tenant id with 404 (via verify_item_access).
    verify_item_access(&state, &principal, path.id).await?;
    let item = state.community_repo.get_item(path.id).await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get item");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse::new("DATABASE_ERROR", "Failed to get item")),
        )
    })?;

    match item {
        Some(i) => Ok(Json(i)),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse::new("NOT_FOUND", "Item not found")),
        )),
    }
}

/// Create an inquiry on a marketplace item.
#[utoipa::path(
    post,
    path = "/api/v1/community/marketplace/{id}/inquiries",
    params(ItemIdPath),
    request_body = CreateMarketplaceInquiry,
    responses(
        (status = 201, description = "Inquiry created", body = MarketplaceInquiry),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    security(("bearer_auth" = [])),
    tag = "Community"
)]
pub async fn create_inquiry(
    State(state): State<AppState>,
    principal: RequestPrincipal,
    Path(path): Path<ItemIdPath>,
    Json(data): Json<CreateMarketplaceInquiry>,
) -> Result<(StatusCode, Json<MarketplaceInquiry>), (StatusCode, Json<ErrorResponse>)> {
    verify_item_access(&state, &principal, path.id).await?;
    let inquiry = state
        .community_repo
        .create_inquiry(path.id, principal.user_id, data)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to create inquiry");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::new(
                    "DATABASE_ERROR",
                    "Failed to create inquiry",
                )),
            )
        })?;

    Ok((StatusCode::CREATED, Json(inquiry)))
}
