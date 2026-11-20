//! Agency dashboard data: stats, performance over time, agency-wide listings,
//! per-realtor stats, and member management (update role/active, remove).
//!
//! Registered inside the agencies router (`/api/v1/agencies/{id}/...`).
//! GH #1300: `PortalPrincipal`; every handler gates on active membership of
//! the agency (`check_agency_membership`), and member management additionally
//! requires an `owner`/`manager` role.
//!
//! An agency's listings are those created by its active members plus any
//! listing explicitly assigned to the agency in `realtor_listings`.

use crate::state::AppState;
use api_core::extractors::PortalPrincipal;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

type ApiError = (StatusCode, String);

/// Listings belonging to agency `$1`.
const AGENCY_LISTINGS_CTE: &str = r#"
    agency_listings AS (
        SELECT l.* FROM listings l
        WHERE l.created_by IN (
                SELECT m.user_id FROM reality_agency_members m
                WHERE m.agency_id = $1 AND m.is_active = TRUE
            )
           OR EXISTS (
                SELECT 1 FROM realtor_listings rl
                WHERE rl.listing_id = l.id AND rl.agency_id = $1
            )
    )
"#;

fn db_err(context: &str, e: sqlx::Error) -> ApiError {
    crate::util::errors::db_error(context, e)
}

async fn require_member(state: &AppState, agency_id: Uuid, user_id: Uuid) -> Result<(), ApiError> {
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_err("acquire db connection", e))?;
    super::agency_imports::check_agency_membership(&mut conn, agency_id, user_id).await
}

async fn require_manager(state: &AppState, agency_id: Uuid, user_id: Uuid) -> Result<(), ApiError> {
    require_member(state, agency_id, user_id).await?;
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_err("acquire db connection", e))?;
    let is_manager: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM reality_agency_members
            WHERE agency_id = $1 AND user_id = $2 AND is_active = TRUE
              AND role IN ('owner', 'manager')
        )
        "#,
    )
    .bind(agency_id)
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| db_err("check manager role", e))?;
    if is_manager {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            "Only agency owners and managers can manage members".to_string(),
        ))
    }
}

// ── Stats ────────────────────────────────────────────────────────────────────

/// Agency dashboard totals (camelCase, matches `@ppt/reality-api-client`).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgencyStatsResponse {
    pub total_listings: i64,
    pub active_listings: i64,
    pub sold_listings: i64,
    pub rented_listings: i64,
    pub total_views: i64,
    pub total_inquiries: i64,
    pub total_realtors: i64,
    /// Share of listings that ended sold or rented, 0..=1.
    pub conversion_rate: f64,
    /// Mean age in days of the currently active listings.
    pub average_days_on_market: f64,
    /// Mean asking price of the currently active listings.
    pub average_listing_price: f64,
}

/// Agency dashboard totals.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/stats",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID")),
    responses(
        (status = 200, description = "Agency stats", body = AgencyStatsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Not a member"),
        (status = 404, description = "Agency not found")
    )
)]
pub async fn get_agency_stats(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(agency_id): Path<Uuid>,
) -> Result<Json<AgencyStatsResponse>, ApiError> {
    require_member(&state, agency_id, principal.user_id).await?;
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_err("acquire db connection", e))?;

    let sql = format!(
        r#"
        WITH {AGENCY_LISTINGS_CTE}
        SELECT
            (SELECT COUNT(*) FROM agency_listings) AS total,
            (SELECT COUNT(*) FROM agency_listings WHERE status = 'active') AS active,
            (SELECT COUNT(*) FROM agency_listings WHERE status = 'sold') AS sold,
            (SELECT COUNT(*) FROM agency_listings WHERE status = 'rented') AS rented,
            (SELECT COALESCE(SUM(a.views), 0)::BIGINT FROM listing_analytics a
                WHERE a.listing_id IN (SELECT id FROM agency_listings)) AS views,
            (SELECT COUNT(*) FROM listing_inquiries i
                WHERE i.listing_id IN (SELECT id FROM agency_listings)) AS inquiries,
            (SELECT COUNT(*) FROM reality_agency_members
                WHERE agency_id = $1 AND is_active = TRUE) AS realtors,
            (SELECT AVG(EXTRACT(EPOCH FROM (NOW() - COALESCE(published_at, created_at))) / 86400.0)
                FROM agency_listings WHERE status = 'active')::FLOAT8 AS avg_days,
            (SELECT AVG(price) FROM agency_listings WHERE status = 'active') AS avg_price
        "#
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(agency_id)
        .fetch_one(&mut *conn)
        .await
        .map_err(|e| db_err("agency stats", e))?;

    let total: i64 = row.get("total");
    let sold: i64 = row.get("sold");
    let rented: i64 = row.get("rented");
    let avg_price: Option<rust_decimal::Decimal> = row.get("avg_price");
    Ok(Json(AgencyStatsResponse {
        total_listings: total,
        active_listings: row.get("active"),
        sold_listings: sold,
        rented_listings: rented,
        total_views: row.get("views"),
        total_inquiries: row.get("inquiries"),
        total_realtors: row.get("realtors"),
        conversion_rate: if total > 0 {
            (sold + rented) as f64 / total as f64
        } else {
            0.0
        },
        average_days_on_market: row.get::<Option<f64>, _>("avg_days").unwrap_or(0.0),
        average_listing_price: avg_price.and_then(|d| d.to_f64()).unwrap_or(0.0),
    }))
}

// ── Performance ──────────────────────────────────────────────────────────────

/// Performance query: date range (inclusive, `YYYY-MM-DD`) and bucket size.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceQuery {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    /// `day`, `week` or `month` (default `month`).
    pub interval: Option<String>,
}

/// One bucket of agency activity.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgencyPerformancePoint {
    /// Bucket start date (`YYYY-MM-DD`).
    pub period: String,
    /// Listings created in the bucket.
    pub listings: i64,
    pub views: i64,
    pub inquiries: i64,
    /// Listings that reached `sold`/`rented` (by last update) in the bucket.
    pub conversions: i64,
}

/// Agency activity bucketed over time (default: last 12 months by month).
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/performance",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID"), PerformanceQuery),
    responses(
        (status = 200, description = "Performance series", body = [AgencyPerformancePoint]),
        (status = 400, description = "Invalid interval or range"),
        (status = 403, description = "Not a member")
    )
)]
pub async fn get_agency_performance(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(agency_id): Path<Uuid>,
    Query(q): Query<PerformanceQuery>,
) -> Result<Json<Vec<AgencyPerformancePoint>>, ApiError> {
    let interval = match q.interval.as_deref().unwrap_or("month") {
        "day" => "day",
        "week" => "week",
        "month" => "month",
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "interval must be one of: day, week, month".to_string(),
            ))
        }
    };
    let end = q.end_date.unwrap_or_else(|| Utc::now().date_naive());
    let start = q
        .start_date
        .unwrap_or_else(|| end - chrono::Duration::days(365));
    if start > end {
        return Err((
            StatusCode::BAD_REQUEST,
            "startDate must not be after endDate".to_string(),
        ));
    }
    // Keep the series bounded (≤ ~2 years of days).
    if (end - start).num_days() > 731 {
        return Err((
            StatusCode::BAD_REQUEST,
            "date range must not exceed two years".to_string(),
        ));
    }
    require_member(&state, agency_id, principal.user_id).await?;

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_err("acquire db connection", e))?;
    // `interval` is one of three literals above, never user text.
    let sql = format!(
        r#"
        WITH {AGENCY_LISTINGS_CTE},
        buckets AS (
            SELECT generate_series(
                date_trunc('{interval}', $2::date::timestamp),
                date_trunc('{interval}', $3::date::timestamp),
                '1 {interval}'::interval
            ) AS b
        )
        SELECT
            to_char(b, 'YYYY-MM-DD') AS period,
            (SELECT COUNT(*) FROM agency_listings l
                WHERE date_trunc('{interval}', l.created_at) = b) AS listings,
            (SELECT COALESCE(SUM(a.views), 0)::BIGINT FROM listing_analytics a
                WHERE a.listing_id IN (SELECT id FROM agency_listings)
                  AND date_trunc('{interval}', a.date::timestamp) = b) AS views,
            (SELECT COUNT(*) FROM listing_inquiries i
                WHERE i.listing_id IN (SELECT id FROM agency_listings)
                  AND date_trunc('{interval}', i.created_at) = b) AS inquiries,
            (SELECT COUNT(*) FROM agency_listings l
                WHERE l.status IN ('sold', 'rented')
                  AND date_trunc('{interval}', l.updated_at) = b) AS conversions
        FROM buckets
        ORDER BY b
        "#
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(agency_id)
        .bind(start)
        .bind(end)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| db_err("agency performance", e))?;

    Ok(Json(
        rows.iter()
            .map(|r| AgencyPerformancePoint {
                period: r.get("period"),
                listings: r.get("listings"),
                views: r.get("views"),
                inquiries: r.get("inquiries"),
                conversions: r.get("conversions"),
            })
            .collect(),
    ))
}

// ── Agency listings ──────────────────────────────────────────────────────────

/// Agency listings query.
#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct AgencyListingsQuery {
    pub status: Option<String>,
    /// Filter to one member (user id of the realtor).
    pub realtor_id: Option<Uuid>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

/// One agency listing row (camelCase, matches `AgencyListing`).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AgencyListingRow {
    pub id: Uuid,
    pub title: String,
    pub property_type: String,
    pub transaction_type: String,
    pub price: f64,
    pub currency: String,
    pub status: String,
    pub realtor_id: Uuid,
    pub realtor_name: String,
    pub views: i64,
    pub inquiries: i64,
    pub primary_photo_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Agency listings page.
#[derive(Debug, Serialize, ToSchema)]
pub struct AgencyListingsResponse {
    pub listings: Vec<AgencyListingRow>,
    pub total: i64,
}

const LISTING_STATUSES: &[&str] = &["draft", "active", "paused", "sold", "rented", "archived"];

/// Listings of all active agency members (newest first).
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/listings",
    tag = "Agencies",
    params(("id" = Uuid, Path, description = "Agency ID"), AgencyListingsQuery),
    responses(
        (status = 200, description = "Agency listings", body = AgencyListingsResponse),
        (status = 400, description = "Invalid status filter"),
        (status = 403, description = "Not a member")
    )
)]
pub async fn list_agency_listings(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path(agency_id): Path<Uuid>,
    Query(q): Query<AgencyListingsQuery>,
) -> Result<Json<AgencyListingsResponse>, ApiError> {
    let status = match q.status.as_deref().map(str::trim) {
        None | Some("") | Some("all") => None,
        Some(s) if LISTING_STATUSES.contains(&s) => Some(s.to_string()),
        Some(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!(
                    "Invalid status filter. Allowed values: {}",
                    LISTING_STATUSES.join(", ")
                ),
            ))
        }
    };
    require_member(&state, agency_id, principal.user_id).await?;

    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1).saturating_mul(limit).min(10_000);

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_err("acquire db connection", e))?;
    let filter = "($2::text IS NULL OR l.status = $2) AND ($3::uuid IS NULL OR l.created_by = $3)";
    let sql = format!(
        r#"
        WITH {AGENCY_LISTINGS_CTE}
        SELECT
            l.id, l.title, l.property_type, l.transaction_type, l.price, l.currency, l.status,
            l.created_by AS realtor_id, COALESCE(u.name, '') AS realtor_name,
            (SELECT COALESCE(SUM(a.views), 0)::BIGINT FROM listing_analytics a
                WHERE a.listing_id = l.id) AS views,
            (SELECT COUNT(*) FROM listing_inquiries i WHERE i.listing_id = l.id) AS inquiries,
            (SELECT p.url FROM listing_photos p WHERE p.listing_id = l.id
                ORDER BY p.display_order LIMIT 1) AS primary_photo_url,
            l.created_at, l.updated_at,
            COUNT(*) OVER () AS total
        FROM agency_listings l
        LEFT JOIN users u ON u.id = l.created_by
        WHERE {filter}
        ORDER BY l.created_at DESC
        LIMIT $4 OFFSET $5
        "#
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(sql))
        .bind(agency_id)
        .bind(&status)
        .bind(q.realtor_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&mut *conn)
        .await
        .map_err(|e| db_err("agency listings", e))?;

    let total = match rows.first() {
        Some(r) => r.get::<i64, _>("total"),
        None => {
            // Past the last page: report the real total, not 0.
            let count_sql = format!(
                "WITH {AGENCY_LISTINGS_CTE} SELECT COUNT(*) FROM agency_listings l WHERE {filter}"
            );
            sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(count_sql))
                .bind(agency_id)
                .bind(&status)
                .bind(q.realtor_id)
                .fetch_one(&mut *conn)
                .await
                .map_err(|e| db_err("count agency listings", e))?
        }
    };

    let listings = rows
        .iter()
        .map(|r| AgencyListingRow {
            id: r.get("id"),
            title: r.get("title"),
            property_type: r.get("property_type"),
            transaction_type: r.get("transaction_type"),
            price: r
                .get::<rust_decimal::Decimal, _>("price")
                .to_f64()
                .unwrap_or(0.0),
            currency: r.get("currency"),
            status: r.get("status"),
            realtor_id: r.get("realtor_id"),
            realtor_name: r.get("realtor_name"),
            views: r.get("views"),
            inquiries: r.get("inquiries"),
            primary_photo_url: r.get("primary_photo_url"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    Ok(Json(AgencyListingsResponse { listings, total }))
}

// ── Per-realtor stats ────────────────────────────────────────────────────────

/// Per-realtor totals (camelCase, matches `RealtorStats`).
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RealtorStatsResponse {
    pub total_listings: i64,
    pub active_listings: i64,
    pub closed_deals: i64,
    pub total_views: i64,
    pub total_inquiries: i64,
    pub conversion_rate: f64,
    /// Mean days from publication to the closing update of sold/rented listings.
    pub average_days_to_close: f64,
    /// Sum of asking prices of sold/rented listings.
    pub total_revenue: f64,
}

/// Stats for one agency member; `member_id` is the membership row id.
#[utoipa::path(
    get,
    path = "/api/v1/agencies/{id}/members/{member_id}/stats",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("member_id" = Uuid, Path, description = "Membership ID")
    ),
    responses(
        (status = 200, description = "Realtor stats", body = RealtorStatsResponse),
        (status = 403, description = "Not a member"),
        (status = 404, description = "Member not found")
    )
)]
pub async fn get_member_stats(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path((agency_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RealtorStatsResponse>, ApiError> {
    require_member(&state, agency_id, principal.user_id).await?;
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_err("acquire db connection", e))?;

    let user_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT user_id FROM reality_agency_members WHERE id = $1 AND agency_id = $2",
    )
    .bind(member_id)
    .bind(agency_id)
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| db_err("find member", e))?;
    let user_id = user_id.ok_or_else(|| (StatusCode::NOT_FOUND, "Member not found".to_string()))?;

    let row = sqlx::query(
        r#"
        WITH mine AS (SELECT * FROM listings WHERE created_by = $1)
        SELECT
            (SELECT COUNT(*) FROM mine) AS total,
            (SELECT COUNT(*) FROM mine WHERE status = 'active') AS active,
            (SELECT COUNT(*) FROM mine WHERE status IN ('sold', 'rented')) AS closed,
            (SELECT COALESCE(SUM(a.views), 0)::BIGINT FROM listing_analytics a
                WHERE a.listing_id IN (SELECT id FROM mine)) AS views,
            (SELECT COUNT(*) FROM listing_inquiries i
                WHERE i.listing_id IN (SELECT id FROM mine)) AS inquiries,
            (SELECT AVG(EXTRACT(EPOCH FROM (updated_at - COALESCE(published_at, created_at))) / 86400.0)
                FROM mine WHERE status IN ('sold', 'rented'))::FLOAT8 AS days_to_close,
            (SELECT SUM(price) FROM mine WHERE status IN ('sold', 'rented')) AS revenue
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await
    .map_err(|e| db_err("realtor stats", e))?;

    let total: i64 = row.get("total");
    let closed: i64 = row.get("closed");
    let revenue: Option<rust_decimal::Decimal> = row.get("revenue");
    Ok(Json(RealtorStatsResponse {
        total_listings: total,
        active_listings: row.get("active"),
        closed_deals: closed,
        total_views: row.get("views"),
        total_inquiries: row.get("inquiries"),
        conversion_rate: if total > 0 {
            closed as f64 / total as f64
        } else {
            0.0
        },
        average_days_to_close: row.get::<Option<f64>, _>("days_to_close").unwrap_or(0.0),
        total_revenue: revenue.and_then(|d| d.to_f64()).unwrap_or(0.0),
    }))
}

// ── Member management ────────────────────────────────────────────────────────

/// Update an agency member. Both fields optional.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMemberRequest {
    /// `owner`, `manager` or `realtor`.
    pub role: Option<String>,
    /// `false` suspends the member without removing them.
    pub is_active: Option<bool>,
}

const MEMBER_ROLES: &[&str] = &["owner", "manager", "realtor"];

/// Change a member's role or active flag (owners/managers only).
#[utoipa::path(
    put,
    path = "/api/v1/agencies/{id}/members/{member_id}",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("member_id" = Uuid, Path, description = "Membership ID")
    ),
    request_body = UpdateMemberRequest,
    responses(
        (status = 200, description = "Updated member", body = db::models::RealityAgencyMember),
        (status = 400, description = "Invalid role or last owner"),
        (status = 403, description = "Not an owner/manager"),
        (status = 404, description = "Member not found")
    )
)]
pub async fn update_member(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path((agency_id, member_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateMemberRequest>,
) -> Result<Json<db::models::RealityAgencyMember>, ApiError> {
    if let Some(role) = req.role.as_deref() {
        if !MEMBER_ROLES.contains(&role) {
            return Err((
                StatusCode::BAD_REQUEST,
                "role must be one of: owner, manager, realtor".to_string(),
            ));
        }
    }
    require_manager(&state, agency_id, principal.user_id).await?;

    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_err("acquire db connection", e))?;
    let mut tx = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| db_err("begin", e))?;

    let was_owner = member_is_active_owner(&mut tx, agency_id, member_id).await?;
    let updated = sqlx::query_as::<_, db::models::RealityAgencyMember>(
        r#"
        UPDATE reality_agency_members SET
            role = COALESCE($3, role),
            is_active = COALESCE($4, is_active),
            left_at = CASE
                WHEN $4 = FALSE THEN COALESCE(left_at, NOW())
                WHEN $4 = TRUE THEN NULL
                ELSE left_at
            END
        WHERE id = $1 AND agency_id = $2
        RETURNING *
        "#,
    )
    .bind(member_id)
    .bind(agency_id)
    .bind(&req.role)
    .bind(req.is_active)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| db_err("update member", e))?
    .ok_or_else(|| (StatusCode::NOT_FOUND, "Member not found".to_string()))?;

    if was_owner {
        ensure_active_owner_remains(&mut tx, agency_id).await?;
    }
    tx.commit().await.map_err(|e| db_err("commit", e))?;
    Ok(Json(updated))
}

/// Remove a member from the agency (owners/managers only). The membership row
/// is kept as inactive with `left_at` set so history stays intact.
#[utoipa::path(
    delete,
    path = "/api/v1/agencies/{id}/members/{member_id}",
    tag = "Agencies",
    params(
        ("id" = Uuid, Path, description = "Agency ID"),
        ("member_id" = Uuid, Path, description = "Membership ID")
    ),
    responses(
        (status = 204, description = "Member removed"),
        (status = 400, description = "Would remove the last owner"),
        (status = 403, description = "Not an owner/manager"),
        (status = 404, description = "Member not found")
    )
)]
pub async fn remove_member(
    State(state): State<AppState>,
    principal: PortalPrincipal,
    Path((agency_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    require_manager(&state, agency_id, principal.user_id).await?;
    let mut conn = state
        .acquire_public_conn()
        .await
        .map_err(|e| db_err("acquire db connection", e))?;
    let mut tx = sqlx::Connection::begin(&mut *conn)
        .await
        .map_err(|e| db_err("begin", e))?;

    let was_owner = member_is_active_owner(&mut tx, agency_id, member_id).await?;
    let removed: Option<Uuid> = sqlx::query_scalar(
        r#"
        UPDATE reality_agency_members
        SET is_active = FALSE, left_at = COALESCE(left_at, NOW())
        WHERE id = $1 AND agency_id = $2
        RETURNING id
        "#,
    )
    .bind(member_id)
    .bind(agency_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| db_err("remove member", e))?;
    if removed.is_none() {
        return Err((StatusCode::NOT_FOUND, "Member not found".to_string()));
    }

    if was_owner {
        ensure_active_owner_remains(&mut tx, agency_id).await?;
    }
    tx.commit().await.map_err(|e| db_err("commit", e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn member_is_active_owner(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    agency_id: Uuid,
    member_id: Uuid,
) -> Result<bool, ApiError> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM reality_agency_members
         WHERE id = $1 AND agency_id = $2 AND role = 'owner' AND is_active = TRUE)",
    )
    .bind(member_id)
    .bind(agency_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| db_err("check member role", e))
}

/// An agency must keep at least one active owner; otherwise nobody could
/// manage it any more. Only enforced when the change touches an active owner. Called inside the mutating transaction (rolled back on
/// error by drop).
async fn ensure_active_owner_remains(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    agency_id: Uuid,
) -> Result<(), ApiError> {
    let owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reality_agency_members
         WHERE agency_id = $1 AND role = 'owner' AND is_active = TRUE",
    )
    .bind(agency_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| db_err("count owners", e))?;
    if owners == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            "An agency must keep at least one active owner".to_string(),
        ));
    }
    Ok(())
}
