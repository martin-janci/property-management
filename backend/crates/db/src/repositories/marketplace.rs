//! Marketplace repository (Epic 68).
//!
//! Implements CRUD operations for service provider marketplace including:
//! - Service Provider Profiles (Story 68.1)
//! - Search & Discovery (Story 68.2)
//! - Request for Quote (Story 68.3)
//! - Provider Verification (Story 68.4)
//! - Reviews & Ratings (Story 68.5)

use crate::models::marketplace::{
    CategoryCount, CreateProviderQuote, CreateProviderReview, CreateProviderVerification,
    CreateRequestForQuote, CreateServiceProviderProfile, ExpiringVerification,
    ManagerMarketplaceDashboard, MarketplaceSearchQuery, MarketplaceStatistics, PendingAction,
    ProviderBadge, ProviderDashboard, ProviderProfileComplete, ProviderQuote, ProviderReview,
    ProviderReviewWithResponse, ProviderSearchResult, ProviderVerification, QuoteComparisonView,
    QuoteWithProvider, RatingBreakdown, RatingDistribution, RequestForQuote, ReviewQuery,
    ReviewStatistics, RfqInvitation, RfqQuery, RfqSummary, ServiceProviderProfile,
    UpdateProviderQuote, UpdateProviderReview, UpdateRequestForQuote, UpdateServiceProviderProfile,
    VerificationQuery, VerificationQueueItem,
};
use crate::DbPool;
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{Error as SqlxError, Executor, Postgres, Row};
use uuid::Uuid;

/// Repository for marketplace operations.
#[derive(Clone)]
pub struct MarketplaceRepository {
    pool: DbPool,
}

impl MarketplaceRepository {
    /// Create a new MarketplaceRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ==================== Story 68.1: Provider Profile Operations ====================

    /// Create a new service provider profile.
    pub async fn create_profile(
        &self,
        user_id: Uuid,
        data: CreateServiceProviderProfile,
    ) -> Result<ServiceProviderProfile, SqlxError> {
        let profile = sqlx::query_as::<_, ServiceProviderProfile>(
            r#"
            INSERT INTO service_provider_profiles (
                user_id, company_name, business_registration_number, tax_id, description,
                logo_url, website, contact_name, contact_email, contact_phone,
                address, city, postal_code, country, service_categories,
                service_description, specializations, coverage_postal_codes, coverage_radius_km,
                coverage_regions, pricing_type, hourly_rate_min, hourly_rate_max, currency,
                certifications, licenses, availability_calendar, response_time_hours,
                emergency_available, portfolio_images, portfolio_description, metadata
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, $20,
                $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32
            )
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&data.company_name)
        .bind(&data.business_registration_number)
        .bind(&data.tax_id)
        .bind(&data.description)
        .bind(&data.logo_url)
        .bind(&data.website)
        .bind(&data.contact_name)
        .bind(&data.contact_email)
        .bind(&data.contact_phone)
        .bind(&data.address)
        .bind(&data.city)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(&data.service_categories)
        .bind(&data.service_description)
        .bind(&data.specializations)
        .bind(&data.coverage_postal_codes)
        .bind(data.coverage_radius_km)
        .bind(&data.coverage_regions)
        .bind(data.pricing_type.as_deref().unwrap_or("hourly"))
        .bind(data.hourly_rate_min)
        .bind(data.hourly_rate_max)
        .bind(&data.currency)
        .bind(&data.certifications)
        .bind(&data.licenses)
        .bind(&data.availability_calendar)
        .bind(data.response_time_hours)
        .bind(data.emergency_available)
        .bind(&data.portfolio_images)
        .bind(&data.portfolio_description)
        .bind(&data.metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(profile)
    }

    /// Find provider profile by ID.
    pub async fn find_profile_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<ServiceProviderProfile>, SqlxError> {
        sqlx::query_as::<_, ServiceProviderProfile>(
            r#"SELECT * FROM service_provider_profiles WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Find provider profile by user ID.
    pub async fn find_profile_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Option<ServiceProviderProfile>, SqlxError> {
        sqlx::query_as::<_, ServiceProviderProfile>(
            r#"SELECT * FROM service_provider_profiles WHERE user_id = $1"#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Update provider profile.
    pub async fn update_profile(
        &self,
        id: Uuid,
        data: UpdateServiceProviderProfile,
    ) -> Result<Option<ServiceProviderProfile>, SqlxError> {
        // Dynamic update using COALESCE pattern
        let profile = sqlx::query_as::<_, ServiceProviderProfile>(
            r#"
            UPDATE service_provider_profiles
            SET
                company_name = COALESCE($2, company_name),
                business_registration_number = COALESCE($3, business_registration_number),
                tax_id = COALESCE($4, tax_id),
                description = COALESCE($5, description),
                logo_url = COALESCE($6, logo_url),
                website = COALESCE($7, website),
                contact_name = COALESCE($8, contact_name),
                contact_email = COALESCE($9, contact_email),
                contact_phone = COALESCE($10, contact_phone),
                address = COALESCE($11, address),
                city = COALESCE($12, city),
                postal_code = COALESCE($13, postal_code),
                country = COALESCE($14, country),
                service_categories = COALESCE($15, service_categories),
                service_description = COALESCE($16, service_description),
                specializations = COALESCE($17, specializations),
                coverage_postal_codes = COALESCE($18, coverage_postal_codes),
                coverage_radius_km = COALESCE($19, coverage_radius_km),
                coverage_regions = COALESCE($20, coverage_regions),
                pricing_type = COALESCE($21, pricing_type),
                hourly_rate_min = COALESCE($22, hourly_rate_min),
                hourly_rate_max = COALESCE($23, hourly_rate_max),
                currency = COALESCE($24, currency),
                certifications = COALESCE($25, certifications),
                licenses = COALESCE($26, licenses),
                availability_calendar = COALESCE($27, availability_calendar),
                response_time_hours = COALESCE($28, response_time_hours),
                emergency_available = COALESCE($29, emergency_available),
                portfolio_images = COALESCE($30, portfolio_images),
                portfolio_description = COALESCE($31, portfolio_description),
                status = COALESCE($32, status),
                metadata = COALESCE($33, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.company_name)
        .bind(&data.business_registration_number)
        .bind(&data.tax_id)
        .bind(&data.description)
        .bind(&data.logo_url)
        .bind(&data.website)
        .bind(&data.contact_name)
        .bind(&data.contact_email)
        .bind(&data.contact_phone)
        .bind(&data.address)
        .bind(&data.city)
        .bind(&data.postal_code)
        .bind(&data.country)
        .bind(&data.service_categories)
        .bind(&data.service_description)
        .bind(&data.specializations)
        .bind(&data.coverage_postal_codes)
        .bind(data.coverage_radius_km)
        .bind(&data.coverage_regions)
        .bind(&data.pricing_type)
        .bind(data.hourly_rate_min)
        .bind(data.hourly_rate_max)
        .bind(&data.currency)
        .bind(&data.certifications)
        .bind(&data.licenses)
        .bind(&data.availability_calendar)
        .bind(data.response_time_hours)
        .bind(data.emergency_available)
        .bind(&data.portfolio_images)
        .bind(&data.portfolio_description)
        .bind(&data.status)
        .bind(&data.metadata)
        .fetch_optional(&self.pool)
        .await?;

        Ok(profile)
    }

    // ==================== Story 68.2: Search & Discovery ====================

    /// Search for providers in the marketplace.
    pub async fn search_providers(
        &self,
        query: &MarketplaceSearchQuery,
    ) -> Result<Vec<ProviderSearchResult>, SqlxError> {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        let mut sql = String::from(
            r#"
            SELECT
                id, company_name, description, logo_url, service_categories,
                city, pricing_type, hourly_rate_min, hourly_rate_max, currency,
                average_rating, total_reviews, total_jobs_completed, is_verified,
                badges, emergency_available, response_time_hours
            FROM service_provider_profiles
            WHERE status = 'active'
            "#,
        );

        let mut conditions: Vec<String> = vec![];

        if query.category.is_some() {
            conditions.push("$1 = ANY(service_categories)".to_string());
        }

        if query.verified_only == Some(true) {
            conditions.push("is_verified = TRUE".to_string());
        }

        if query.emergency_only == Some(true) {
            conditions.push("emergency_available = TRUE".to_string());
        }

        if query.min_rating.is_some() {
            conditions.push("average_rating >= $2".to_string());
        }

        if query.max_hourly_rate.is_some() {
            conditions.push("hourly_rate_max <= $3".to_string());
        }

        if query.location.is_some() {
            conditions.push(
                "($4 = ANY(coverage_postal_codes) OR city ILIKE '%' || $4 || '%')".to_string(),
            );
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        // Sorting
        let sort_by = query.sort_by.as_deref().unwrap_or("rating");
        let sort_order = query.sort_order.as_deref().unwrap_or("desc");
        let order_dir = if sort_order == "asc" { "ASC" } else { "DESC" };

        sql.push_str(&format!(
            " ORDER BY {} {} LIMIT {} OFFSET {}",
            match sort_by {
                "reviews" => "total_reviews",
                "jobs_completed" => "total_jobs_completed",
                "hourly_rate" => "hourly_rate_min",
                _ => "average_rating",
            },
            order_dir,
            limit,
            offset
        ));

        // Build query with bindings
        let results = sqlx::query_as::<_, ProviderSearchResult>(sqlx::AssertSqlSafe(sql))
            .bind(&query.category)
            .bind(query.min_rating)
            .bind(query.max_hourly_rate)
            .bind(&query.location)
            .fetch_all(&self.pool)
            .await?;

        Ok(results)
    }

    /// Get marketplace statistics.
    pub async fn get_statistics(&self) -> Result<MarketplaceStatistics, SqlxError> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_providers,
                COUNT(*) FILTER (WHERE is_verified = TRUE) as verified_providers,
                COALESCE(AVG(average_rating), 0) as average_rating,
                COALESCE(SUM(total_reviews), 0) as total_reviews,
                COALESCE(SUM(total_jobs_completed), 0) as total_jobs_completed
            FROM service_provider_profiles
            WHERE status = 'active'
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        let providers_by_category = sqlx::query_as::<_, CategoryCount>(
            r#"
            SELECT unnest(service_categories) as category, COUNT(*) as count
            FROM service_provider_profiles
            WHERE status = 'active'
            GROUP BY category
            ORDER BY count DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(MarketplaceStatistics {
            total_providers: row.get::<i64, _>("total_providers"),
            verified_providers: row.get::<i64, _>("verified_providers"),
            providers_by_category,
            average_rating: row.get::<Decimal, _>("average_rating"),
            total_reviews: row.get::<i64, _>("total_reviews"),
            total_jobs_completed: row.get::<i64, _>("total_jobs_completed"),
        })
    }

    /// Get complete provider profile with all related data.
    pub async fn get_complete_profile(
        &self,
        id: Uuid,
    ) -> Result<Option<ProviderProfileComplete>, SqlxError> {
        let profile = match self.find_profile_by_id(id).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let verifications = self.list_verifications_by_provider(id).await?;
        let badges = self.list_provider_badges(id).await?;
        let rating_breakdown = self.get_rating_breakdown(id).await?;
        let recent_reviews = self.list_provider_reviews(id, 5).await?;

        let stats = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status IN ('sent', 'quotes_received')) as active_rfqs,
                COUNT(*) as pending_quotes
            FROM rfq_invitations ri
            JOIN rfqs r ON r.id = ri.rfq_id
            WHERE ri.provider_id = $1 AND ri.declined = FALSE
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(ProviderProfileComplete {
            profile,
            verifications,
            badges,
            rating_breakdown,
            recent_reviews,
            active_rfqs: stats.get::<i64, _>("active_rfqs"),
            pending_quotes: stats.get::<i64, _>("pending_quotes"),
        }))
    }

    /// Get provider dashboard.
    pub async fn get_provider_dashboard(
        &self,
        user_id: Uuid,
    ) -> Result<Option<ProviderDashboard>, SqlxError> {
        let profile = match self.find_profile_by_user_id(user_id).await? {
            Some(p) => p,
            None => return Ok(None),
        };

        let provider_id = profile.id;

        // RFQ summary
        let rfq_stats = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE ri.viewed_at IS NULL AND ri.declined = FALSE) as pending_invitations,
                COUNT(*) FILTER (WHERE pq.status = 'submitted') as active_quotes,
                COUNT(*) FILTER (WHERE pq.status = 'accepted') as won_quotes,
                COUNT(*) as total_quotes_submitted
            FROM rfq_invitations ri
            LEFT JOIN provider_quotes pq ON pq.rfq_id = ri.rfq_id AND pq.provider_id = ri.provider_id
            WHERE ri.provider_id = $1
            "#,
        )
        .bind(provider_id)
        .fetch_one(&self.pool)
        .await?;

        let pending_invitations = rfq_stats.get::<i64, _>("pending_invitations");
        let active_quotes = rfq_stats.get::<i64, _>("active_quotes");
        let won_quotes = rfq_stats.get::<i64, _>("won_quotes");
        let total_quotes_submitted = rfq_stats.get::<i64, _>("total_quotes_submitted");

        let win_rate = if total_quotes_submitted > 0 {
            Decimal::from(won_quotes * 100) / Decimal::from(total_quotes_submitted)
        } else {
            Decimal::ZERO
        };

        let rfq_summary = RfqSummary {
            pending_invitations,
            active_quotes,
            won_quotes,
            total_quotes_submitted,
            win_rate,
        };

        // Review stats
        let review_stats_row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_reviews,
                COALESCE(AVG(overall_rating), 0) as average_rating,
                COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '30 days') as reviews_this_month,
                COUNT(*) FILTER (WHERE provider_response IS NULL AND status = 'approved') as pending_responses
            FROM provider_reviews
            WHERE provider_id = $1 AND status = 'approved'
            "#,
        )
        .bind(provider_id)
        .fetch_one(&self.pool)
        .await?;

        let total_reviews = review_stats_row.get::<i64, _>("total_reviews");
        let avg_rating: Decimal = review_stats_row.get("average_rating");
        let reviews_this_month = review_stats_row.get::<i64, _>("reviews_this_month");
        let pending_responses = review_stats_row.get::<i64, _>("pending_responses");

        let responded_count = total_reviews - pending_responses;
        let response_rate = if total_reviews > 0 {
            Decimal::from(responded_count * 100) / Decimal::from(total_reviews)
        } else {
            Decimal::ZERO
        };

        let review_stats = ReviewStatistics {
            total_reviews,
            average_rating: avg_rating,
            reviews_this_month,
            rating_trend: Decimal::ZERO, // TODO: Calculate trend
            response_rate,
            pending_responses,
        };

        // Expiring verifications
        let expiring_verifications = self
            .get_expiring_verifications_for_provider(provider_id, 30)
            .await?;

        // Pending actions
        let pending_actions = self.get_pending_actions(provider_id).await?;

        Ok(Some(ProviderDashboard {
            profile,
            rfq_summary,
            review_stats,
            expiring_verifications,
            pending_actions,
        }))
    }

    async fn get_pending_actions(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<PendingAction>, SqlxError> {
        let mut actions = vec![];

        // Check for unanswered RFQ invitations
        let invitations = sqlx::query(
            r#"
            SELECT ri.id, r.title, r.quote_deadline
            FROM rfq_invitations ri
            JOIN rfqs r ON r.id = ri.rfq_id
            WHERE ri.provider_id = $1 AND ri.viewed_at IS NULL AND ri.declined = FALSE
            AND r.status = 'sent'
            ORDER BY r.quote_deadline ASC
            LIMIT 5
            "#,
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        for inv in invitations {
            actions.push(PendingAction {
                action_type: "rfq_invitation".to_string(),
                title: "New RFQ Invitation".to_string(),
                description: inv.get::<String, _>("title"),
                due_date: inv.get("quote_deadline"),
                reference_id: inv.get::<Uuid, _>("id"),
                priority: "medium".to_string(),
            });
        }

        // Check for reviews pending response
        let reviews = sqlx::query(
            r#"
            SELECT id, review_title, created_at
            FROM provider_reviews
            WHERE provider_id = $1 AND provider_response IS NULL AND status = 'approved'
            ORDER BY created_at DESC
            LIMIT 3
            "#,
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await?;

        for rev in reviews {
            actions.push(PendingAction {
                action_type: "review_response".to_string(),
                title: "Review Needs Response".to_string(),
                description: rev
                    .get::<Option<String>, _>("review_title")
                    .unwrap_or_default(),
                due_date: None,
                reference_id: rev.get::<Uuid, _>("id"),
                priority: "low".to_string(),
            });
        }

        Ok(actions)
    }

    // ==================== Story 68.3: RFQ Operations ====================

    /// Create a new RFQ.
    pub async fn create_rfq(
        &self,
        organization_id: Uuid,
        created_by: Uuid,
        data: CreateRequestForQuote,
    ) -> Result<RequestForQuote, SqlxError> {
        let rfq = sqlx::query_as::<_, RequestForQuote>(
            r#"
            INSERT INTO rfqs (
                organization_id, building_id, created_by, title, description,
                service_category, scope_of_work, preferred_start_date, preferred_end_date,
                is_urgent, budget_min, budget_max, currency, attachments, images,
                quote_deadline, contact_preference, site_visit_required, metadata, status
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19, 'draft'
            )
            RETURNING *
            "#,
        )
        .bind(organization_id)
        .bind(data.building_id)
        .bind(created_by)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.service_category)
        .bind(&data.scope_of_work)
        .bind(data.preferred_start_date)
        .bind(data.preferred_end_date)
        .bind(data.is_urgent)
        .bind(data.budget_min)
        .bind(data.budget_max)
        .bind(&data.currency)
        .bind(&data.attachments)
        .bind(&data.images)
        .bind(data.quote_deadline)
        .bind(&data.contact_preference)
        .bind(data.site_visit_required)
        .bind(&data.metadata)
        .fetch_one(&self.pool)
        .await?;

        // Create invitations for specified providers
        for provider_id in &data.provider_ids {
            sqlx::query(
                r#"
                INSERT INTO rfq_invitations (rfq_id, provider_id)
                VALUES ($1, $2)
                ON CONFLICT (rfq_id, provider_id) DO NOTHING
                "#,
            )
            .bind(rfq.id)
            .bind(provider_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(rfq)
    }

    /// Find RFQ by ID.
    pub async fn find_rfq_by_id(&self, id: Uuid) -> Result<Option<RequestForQuote>, SqlxError> {
        sqlx::query_as::<_, RequestForQuote>(r#"SELECT * FROM rfqs WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// Find RFQ by ID with RLS context.
    pub async fn find_rfq_by_id_rls<'e, E>(
        &self,
        executor: E,
        id: Uuid,
    ) -> Result<Option<RequestForQuote>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        sqlx::query_as::<_, RequestForQuote>(r#"SELECT * FROM rfqs WHERE id = $1"#)
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    /// List RFQs for an organization.
    pub async fn list_rfqs(
        &self,
        organization_id: Uuid,
        query: &RfqQuery,
    ) -> Result<Vec<RequestForQuote>, SqlxError> {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, RequestForQuote>(
            r#"
            SELECT * FROM rfqs
            WHERE organization_id = $1
            AND ($2::text IS NULL OR status = $2)
            AND ($3::text IS NULL OR service_category = $3)
            AND ($4::uuid IS NULL OR building_id = $4)
            AND ($5::boolean IS NULL OR is_urgent = $5)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(organization_id)
        .bind(&query.status)
        .bind(&query.service_category)
        .bind(query.building_id)
        .bind(query.is_urgent)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// List RFQs with RLS context.
    pub async fn list_rfqs_rls<'e, E>(
        &self,
        executor: E,
        organization_id: Uuid,
        query: &RfqQuery,
    ) -> Result<Vec<RequestForQuote>, SqlxError>
    where
        E: Executor<'e, Database = Postgres>,
    {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, RequestForQuote>(
            r#"
            SELECT * FROM rfqs
            WHERE organization_id = $1
            AND ($2::text IS NULL OR status = $2)
            AND ($3::text IS NULL OR service_category = $3)
            AND ($4::uuid IS NULL OR building_id = $4)
            AND ($5::boolean IS NULL OR is_urgent = $5)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(organization_id)
        .bind(&query.status)
        .bind(&query.service_category)
        .bind(query.building_id)
        .bind(query.is_urgent)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
    }

    /// Update an RFQ.
    pub async fn update_rfq(
        &self,
        id: Uuid,
        data: UpdateRequestForQuote,
    ) -> Result<Option<RequestForQuote>, SqlxError> {
        sqlx::query_as::<_, RequestForQuote>(
            r#"
            UPDATE rfqs
            SET
                title = COALESCE($2, title),
                description = COALESCE($3, description),
                scope_of_work = COALESCE($4, scope_of_work),
                preferred_start_date = COALESCE($5, preferred_start_date),
                preferred_end_date = COALESCE($6, preferred_end_date),
                is_urgent = COALESCE($7, is_urgent),
                budget_min = COALESCE($8, budget_min),
                budget_max = COALESCE($9, budget_max),
                attachments = COALESCE($10, attachments),
                images = COALESCE($11, images),
                quote_deadline = COALESCE($12, quote_deadline),
                contact_preference = COALESCE($13, contact_preference),
                site_visit_required = COALESCE($14, site_visit_required),
                status = COALESCE($15, status),
                metadata = COALESCE($16, metadata),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.title)
        .bind(&data.description)
        .bind(&data.scope_of_work)
        .bind(data.preferred_start_date)
        .bind(data.preferred_end_date)
        .bind(data.is_urgent)
        .bind(data.budget_min)
        .bind(data.budget_max)
        .bind(&data.attachments)
        .bind(&data.images)
        .bind(data.quote_deadline)
        .bind(&data.contact_preference)
        .bind(data.site_visit_required)
        .bind(&data.status)
        .bind(&data.metadata)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete an RFQ (soft delete by setting status to cancelled).
    pub async fn delete_rfq(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE rfqs SET status = 'cancelled', updated_at = NOW()
            WHERE id = $1 AND status = 'draft'
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Send RFQ to providers (update status).
    pub async fn send_rfq(&self, id: Uuid) -> Result<Option<RequestForQuote>, SqlxError> {
        sqlx::query_as::<_, RequestForQuote>(
            r#"
            UPDATE rfqs SET status = 'sent', updated_at = NOW()
            WHERE id = $1 AND status = 'draft'
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Award an RFQ to a quote.
    pub async fn award_rfq(
        &self,
        id: Uuid,
        quote_id: Uuid,
    ) -> Result<Option<RequestForQuote>, SqlxError> {
        // Get quote info. Defense-in-depth against IDOR: the awarded quote must
        // belong to THIS rfq. A quote from a different RFQ (possibly another
        // org) must never be awardable, even if a caller reaches this method
        // without the route-level ownership check. Return `Ok(None)` (surfaced
        // as 404) rather than awarding a foreign quote.
        let provider_id = match self.find_quote_by_id(quote_id).await? {
            Some(q) if q.rfq_id == id => Some(q.provider_id),
            _ => return Ok(None),
        };

        // Update RFQ
        let rfq = sqlx::query_as::<_, RequestForQuote>(
            r#"
            UPDATE rfqs SET
                status = 'awarded',
                awarded_to = $2,
                awarded_quote_id = $3,
                awarded_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(provider_id)
        .bind(quote_id)
        .fetch_optional(&self.pool)
        .await?;

        // Update the winning quote status
        if rfq.is_some() {
            sqlx::query(
                r#"
                UPDATE provider_quotes SET status = 'accepted', updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(quote_id)
            .execute(&self.pool)
            .await?;

            // Reject other quotes
            sqlx::query(
                r#"
                UPDATE provider_quotes SET status = 'rejected', updated_at = NOW()
                WHERE rfq_id = $1 AND id != $2 AND status = 'submitted'
                "#,
            )
            .bind(id)
            .bind(quote_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(rfq)
    }

    /// Cancel an RFQ.
    pub async fn cancel_rfq(&self, id: Uuid) -> Result<Option<RequestForQuote>, SqlxError> {
        sqlx::query_as::<_, RequestForQuote>(
            r#"
            UPDATE rfqs SET status = 'cancelled', updated_at = NOW()
            WHERE id = $1 AND status NOT IN ('awarded', 'cancelled')
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List quotes for an RFQ.
    pub async fn list_rfq_quotes(&self, rfq_id: Uuid) -> Result<Vec<ProviderQuote>, SqlxError> {
        sqlx::query_as::<_, ProviderQuote>(
            r#"SELECT * FROM provider_quotes WHERE rfq_id = $1 ORDER BY price ASC"#,
        )
        .bind(rfq_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Get quote comparison for an RFQ.
    pub async fn get_quote_comparison(
        &self,
        rfq_id: Uuid,
    ) -> Result<Option<QuoteComparisonView>, SqlxError> {
        let rfq = match self.find_rfq_by_id(rfq_id).await? {
            Some(r) => r,
            None => return Ok(None),
        };

        let quotes = self.list_rfq_quotes(rfq_id).await?;
        let mut quotes_with_providers = vec![];

        for quote in quotes {
            if let Some(provider) = self.get_provider_search_result(quote.provider_id).await? {
                quotes_with_providers.push(QuoteWithProvider { quote, provider });
            }
        }

        Ok(Some(QuoteComparisonView {
            rfq,
            quotes: quotes_with_providers,
        }))
    }

    async fn get_provider_search_result(
        &self,
        id: Uuid,
    ) -> Result<Option<ProviderSearchResult>, SqlxError> {
        sqlx::query_as::<_, ProviderSearchResult>(
            r#"
            SELECT
                id, company_name, description, logo_url, service_categories,
                city, pricing_type, hourly_rate_min, hourly_rate_max, currency,
                average_rating, total_reviews, total_jobs_completed, is_verified,
                badges, emergency_available, response_time_hours
            FROM service_provider_profiles
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    // ==================== Quote Operations ====================

    /// Submit a quote for an RFQ.
    pub async fn submit_quote(
        &self,
        provider_id: Uuid,
        data: CreateProviderQuote,
    ) -> Result<ProviderQuote, SqlxError> {
        sqlx::query_as::<_, ProviderQuote>(
            r#"
            INSERT INTO provider_quotes (
                rfq_id, provider_id, price, currency, price_breakdown,
                estimated_start_date, estimated_end_date, estimated_duration_days,
                terms_and_conditions, warranty_period_days, payment_terms,
                notes, attachments, valid_until, metadata, status, submitted_at
            )
            VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, 'submitted', NOW()
            )
            RETURNING *
            "#,
        )
        .bind(data.rfq_id)
        .bind(provider_id)
        .bind(data.price)
        .bind(data.currency.as_deref().unwrap_or("EUR"))
        .bind(&data.price_breakdown)
        .bind(data.estimated_start_date)
        .bind(data.estimated_end_date)
        .bind(data.estimated_duration_days)
        .bind(&data.terms_and_conditions)
        .bind(data.warranty_period_days)
        .bind(&data.payment_terms)
        .bind(&data.notes)
        .bind(&data.attachments)
        .bind(data.valid_until)
        .bind(&data.metadata)
        .fetch_one(&self.pool)
        .await
    }

    /// Find quote by ID.
    pub async fn find_quote_by_id(&self, id: Uuid) -> Result<Option<ProviderQuote>, SqlxError> {
        sqlx::query_as::<_, ProviderQuote>(r#"SELECT * FROM provider_quotes WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List quotes by provider.
    pub async fn list_provider_quotes(
        &self,
        provider_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<ProviderQuote>, SqlxError> {
        sqlx::query_as::<_, ProviderQuote>(
            r#"
            SELECT * FROM provider_quotes
            WHERE provider_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(provider_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a quote.
    pub async fn update_quote(
        &self,
        id: Uuid,
        data: UpdateProviderQuote,
    ) -> Result<Option<ProviderQuote>, SqlxError> {
        sqlx::query_as::<_, ProviderQuote>(
            r#"
            UPDATE provider_quotes
            SET
                price = COALESCE($2, price),
                price_breakdown = COALESCE($3, price_breakdown),
                estimated_start_date = COALESCE($4, estimated_start_date),
                estimated_end_date = COALESCE($5, estimated_end_date),
                estimated_duration_days = COALESCE($6, estimated_duration_days),
                terms_and_conditions = COALESCE($7, terms_and_conditions),
                warranty_period_days = COALESCE($8, warranty_period_days),
                payment_terms = COALESCE($9, payment_terms),
                notes = COALESCE($10, notes),
                attachments = COALESCE($11, attachments),
                valid_until = COALESCE($12, valid_until),
                metadata = COALESCE($13, metadata),
                updated_at = NOW()
            WHERE id = $1 AND status IN ('pending', 'submitted')
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.price)
        .bind(&data.price_breakdown)
        .bind(data.estimated_start_date)
        .bind(data.estimated_end_date)
        .bind(data.estimated_duration_days)
        .bind(&data.terms_and_conditions)
        .bind(data.warranty_period_days)
        .bind(&data.payment_terms)
        .bind(&data.notes)
        .bind(&data.attachments)
        .bind(data.valid_until)
        .bind(&data.metadata)
        .fetch_optional(&self.pool)
        .await
    }

    /// Withdraw a quote.
    pub async fn withdraw_quote(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE provider_quotes SET status = 'withdrawn', updated_at = NOW()
            WHERE id = $1 AND status IN ('pending', 'submitted')
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Invitation Operations ====================

    /// List invitations for a provider.
    pub async fn list_provider_invitations(
        &self,
        provider_id: Uuid,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<RfqInvitation>, SqlxError> {
        sqlx::query_as::<_, RfqInvitation>(
            r#"
            SELECT * FROM rfq_invitations
            WHERE provider_id = $1
            ORDER BY invited_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(provider_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Mark invitation as viewed.
    ///
    /// Keyed on `provider_id` as well as `id` so a provider can only act on its
    /// own invitations. Threading the caller's verified provider id closes the
    /// cross-provider IDOR (PAP-140): without it any authenticated provider
    /// could flip another provider's invitation by guessing its UUID.
    pub async fn mark_invitation_viewed(
        &self,
        id: Uuid,
        provider_id: Uuid,
    ) -> Result<Option<RfqInvitation>, SqlxError> {
        // Idempotent (#1301): key only on (id, provider_id) so a legitimate
        // repeat view returns 200 with the invitation, not 404. `COALESCE`
        // preserves the FIRST viewed_at timestamp on subsequent views. A None
        // result now means only "this provider has no such invitation" (real
        // 404), matching the sibling `decline_invitation` keying.
        sqlx::query_as::<_, RfqInvitation>(
            r#"
            UPDATE rfq_invitations SET viewed_at = COALESCE(viewed_at, NOW())
            WHERE id = $1 AND provider_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(provider_id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Decline an invitation.
    ///
    /// Keyed on `provider_id` as well as `id` (PAP-140) so a provider can only
    /// decline invitations addressed to it.
    pub async fn decline_invitation(
        &self,
        id: Uuid,
        provider_id: Uuid,
        reason: Option<String>,
    ) -> Result<Option<RfqInvitation>, SqlxError> {
        sqlx::query_as::<_, RfqInvitation>(
            r#"
            UPDATE rfq_invitations
            SET declined = TRUE, decline_reason = $3, responded_at = NOW()
            WHERE id = $1 AND provider_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(provider_id)
        .bind(reason)
        .fetch_optional(&self.pool)
        .await
    }

    // ==================== Story 68.4: Verification Operations ====================

    /// Submit a verification document.
    pub async fn submit_verification(
        &self,
        provider_id: Uuid,
        data: CreateProviderVerification,
    ) -> Result<ProviderVerification, SqlxError> {
        sqlx::query_as::<_, ProviderVerification>(
            r#"
            INSERT INTO provider_verifications (
                provider_id, verification_type, document_name, document_number,
                issuing_authority, issue_date, expiry_date, document_url, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(provider_id)
        .bind(&data.verification_type)
        .bind(&data.document_name)
        .bind(&data.document_number)
        .bind(&data.issuing_authority)
        .bind(data.issue_date)
        .bind(data.expiry_date)
        .bind(&data.document_url)
        .bind(&data.metadata)
        .fetch_one(&self.pool)
        .await
    }

    /// Find verification by ID.
    pub async fn find_verification_by_id(
        &self,
        id: Uuid,
    ) -> Result<Option<ProviderVerification>, SqlxError> {
        sqlx::query_as::<_, ProviderVerification>(
            r#"SELECT * FROM provider_verifications WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// List verifications by provider.
    pub async fn list_verifications_by_provider(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<ProviderVerification>, SqlxError> {
        sqlx::query_as::<_, ProviderVerification>(
            r#"
            SELECT * FROM provider_verifications
            WHERE provider_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await
    }

    /// List verifications with filters.
    pub async fn list_verifications(
        &self,
        query: &VerificationQuery,
    ) -> Result<Vec<ProviderVerification>, SqlxError> {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, ProviderVerification>(
            r#"
            SELECT * FROM provider_verifications
            WHERE ($1::uuid IS NULL OR provider_id = $1)
            AND ($2::text IS NULL OR verification_type = $2)
            AND ($3::text IS NULL OR status = $3)
            ORDER BY created_at DESC
            LIMIT $4 OFFSET $5
            "#,
        )
        .bind(query.provider_id)
        .bind(&query.verification_type)
        .bind(&query.status)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Get verification queue for admin review.
    pub async fn get_verification_queue(
        &self,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<VerificationQueueItem>, SqlxError> {
        sqlx::query_as::<_, VerificationQueueItem>(
            r#"
            SELECT
                pv.id,
                pv.provider_id,
                sp.company_name as provider_name,
                pv.verification_type,
                pv.document_name,
                pv.created_at as submitted_at,
                pv.expiry_date,
                EXTRACT(DAY FROM NOW() - pv.created_at)::integer as days_pending
            FROM provider_verifications pv
            JOIN service_provider_profiles sp ON sp.id = pv.provider_id
            WHERE pv.status = 'pending'
            ORDER BY pv.created_at ASC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Get expiring verifications.
    pub async fn get_expiring_verifications(
        &self,
        days: i32,
    ) -> Result<Vec<ExpiringVerification>, SqlxError> {
        sqlx::query_as::<_, ExpiringVerification>(
            r#"
            SELECT
                pv.id,
                pv.provider_id,
                sp.company_name as provider_name,
                pv.verification_type,
                pv.document_name,
                pv.expiry_date,
                (pv.expiry_date - CURRENT_DATE) as days_until_expiry
            FROM provider_verifications pv
            JOIN service_provider_profiles sp ON sp.id = pv.provider_id
            WHERE pv.status = 'verified'
            AND pv.expiry_date IS NOT NULL
            AND pv.expiry_date <= CURRENT_DATE + $1
            ORDER BY pv.expiry_date ASC
            "#,
        )
        .bind(days)
        .fetch_all(&self.pool)
        .await
    }

    async fn get_expiring_verifications_for_provider(
        &self,
        provider_id: Uuid,
        days: i32,
    ) -> Result<Vec<ExpiringVerification>, SqlxError> {
        sqlx::query_as::<_, ExpiringVerification>(
            r#"
            SELECT
                pv.id,
                pv.provider_id,
                sp.company_name as provider_name,
                pv.verification_type,
                pv.document_name,
                pv.expiry_date,
                (pv.expiry_date - CURRENT_DATE) as days_until_expiry
            FROM provider_verifications pv
            JOIN service_provider_profiles sp ON sp.id = pv.provider_id
            WHERE pv.provider_id = $1
            AND pv.status = 'verified'
            AND pv.expiry_date IS NOT NULL
            AND pv.expiry_date <= CURRENT_DATE + $2
            ORDER BY pv.expiry_date ASC
            "#,
        )
        .bind(provider_id)
        .bind(days)
        .fetch_all(&self.pool)
        .await
    }

    /// Review a verification (approve/reject).
    pub async fn review_verification(
        &self,
        id: Uuid,
        reviewed_by: Uuid,
        status: &str,
        rejection_reason: Option<String>,
        notes: Option<String>,
    ) -> Result<Option<ProviderVerification>, SqlxError> {
        let verification = sqlx::query_as::<_, ProviderVerification>(
            r#"
            UPDATE provider_verifications
            SET
                status = $2,
                reviewed_by = $3,
                reviewed_at = NOW(),
                rejection_reason = $4,
                notes = $5,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(reviewed_by)
        .bind(rejection_reason)
        .bind(notes)
        .fetch_optional(&self.pool)
        .await?;

        // If verified, update provider's verified status
        if let Some(ref v) = verification {
            if status == "verified" {
                sqlx::query(
                    r#"
                    UPDATE service_provider_profiles
                    SET is_verified = TRUE, verified_at = NOW(), updated_at = NOW()
                    WHERE id = $1
                    "#,
                )
                .bind(v.provider_id)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(verification)
    }

    // ==================== Badge Operations ====================

    /// List badges for a provider.
    pub async fn list_provider_badges(
        &self,
        provider_id: Uuid,
    ) -> Result<Vec<ProviderBadge>, SqlxError> {
        sqlx::query_as::<_, ProviderBadge>(
            r#"
            SELECT * FROM provider_badges
            WHERE provider_id = $1
            AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY awarded_at DESC
            "#,
        )
        .bind(provider_id)
        .fetch_all(&self.pool)
        .await
    }

    /// Award a badge to a provider.
    pub async fn award_badge(
        &self,
        provider_id: Uuid,
        awarded_by: Uuid,
        badge_type: &str,
        verification_id: Option<Uuid>,
        expires_at: Option<chrono::DateTime<Utc>>,
        notes: Option<String>,
    ) -> Result<ProviderBadge, SqlxError> {
        sqlx::query_as::<_, ProviderBadge>(
            r#"
            INSERT INTO provider_badges (
                provider_id, badge_type, awarded_by, verification_id, expires_at, notes
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (provider_id, badge_type) DO UPDATE SET
                awarded_by = $3,
                awarded_at = NOW(),
                verification_id = $4,
                expires_at = $5,
                notes = $6
            RETURNING *
            "#,
        )
        .bind(provider_id)
        .bind(badge_type)
        .bind(awarded_by)
        .bind(verification_id)
        .bind(expires_at)
        .bind(notes)
        .fetch_one(&self.pool)
        .await
    }

    /// Revoke a badge.
    pub async fn revoke_badge(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(r#"DELETE FROM provider_badges WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Story 68.5: Review Operations ====================

    /// Create a review.
    pub async fn create_review(
        &self,
        provider_id: Uuid,
        reviewer_id: Uuid,
        organization_id: Uuid,
        data: CreateProviderReview,
    ) -> Result<ProviderReview, SqlxError> {
        // Calculate overall rating
        let overall = (data.quality_rating
            + data.timeliness_rating
            + data.communication_rating
            + data.value_rating) as f64
            / 4.0;

        sqlx::query_as::<_, ProviderReview>(
            r#"
            INSERT INTO provider_reviews (
                provider_id, reviewer_id, organization_id, job_id, rfq_id,
                quality_rating, timeliness_rating, communication_rating, value_rating,
                overall_rating, review_title, review_text, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
            "#,
        )
        .bind(provider_id)
        .bind(reviewer_id)
        .bind(organization_id)
        .bind(data.job_id)
        .bind(data.rfq_id)
        .bind(data.quality_rating)
        .bind(data.timeliness_rating)
        .bind(data.communication_rating)
        .bind(data.value_rating)
        .bind(overall.round() as i32)
        .bind(&data.review_title)
        .bind(&data.review_text)
        .bind(&data.metadata)
        .fetch_one(&self.pool)
        .await
    }

    /// Find review by ID.
    pub async fn find_review_by_id(&self, id: Uuid) -> Result<Option<ProviderReview>, SqlxError> {
        sqlx::query_as::<_, ProviderReview>(r#"SELECT * FROM provider_reviews WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
    }

    /// List reviews for a provider.
    pub async fn list_provider_reviews(
        &self,
        provider_id: Uuid,
        limit: i32,
    ) -> Result<Vec<ProviderReviewWithResponse>, SqlxError> {
        let reviews = sqlx::query(
            r#"
            SELECT
                pr.*,
                u.name as reviewer_name,
                o.name as reviewer_organization
            FROM provider_reviews pr
            JOIN users u ON u.id = pr.reviewer_id
            JOIN organizations o ON o.id = pr.organization_id
            WHERE pr.provider_id = $1 AND pr.status = 'approved'
            ORDER BY pr.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(provider_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut result = vec![];
        for row in reviews {
            let review = ProviderReview {
                id: row.get("id"),
                provider_id: row.get("provider_id"),
                reviewer_id: row.get("reviewer_id"),
                organization_id: row.get("organization_id"),
                job_id: row.get("job_id"),
                rfq_id: row.get("rfq_id"),
                quality_rating: row.get("quality_rating"),
                timeliness_rating: row.get("timeliness_rating"),
                communication_rating: row.get("communication_rating"),
                value_rating: row.get("value_rating"),
                overall_rating: row.get("overall_rating"),
                review_title: row.get("review_title"),
                review_text: row.get("review_text"),
                status: row.get("status"),
                moderated_by: row.get("moderated_by"),
                moderated_at: row.get("moderated_at"),
                moderation_notes: row.get("moderation_notes"),
                provider_response: row.get("provider_response"),
                provider_responded_at: row.get("provider_responded_at"),
                helpful_count: row.get("helpful_count"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            };

            result.push(ProviderReviewWithResponse {
                review,
                reviewer_name: row.get("reviewer_name"),
                reviewer_organization: row.get("reviewer_organization"),
            });
        }

        Ok(result)
    }

    /// List reviews with filters.
    pub async fn list_reviews(
        &self,
        query: &ReviewQuery,
    ) -> Result<Vec<ProviderReview>, SqlxError> {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        sqlx::query_as::<_, ProviderReview>(
            r#"
            SELECT * FROM provider_reviews
            WHERE ($1::uuid IS NULL OR provider_id = $1)
            AND ($2::uuid IS NULL OR organization_id = $2)
            AND ($3::integer IS NULL OR overall_rating >= $3)
            AND ($4::text IS NULL OR status = $4)
            AND ($5::boolean IS NULL OR (provider_response IS NOT NULL) = $5)
            ORDER BY created_at DESC
            LIMIT $6 OFFSET $7
            "#,
        )
        .bind(query.provider_id)
        .bind(query.organization_id)
        .bind(query.min_rating)
        .bind(&query.status)
        .bind(query.has_response)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    /// Update a review.
    pub async fn update_review(
        &self,
        id: Uuid,
        reviewer_id: Uuid,
        data: UpdateProviderReview,
    ) -> Result<Option<ProviderReview>, SqlxError> {
        sqlx::query_as::<_, ProviderReview>(
            r#"
            UPDATE provider_reviews
            SET
                quality_rating = COALESCE($3, quality_rating),
                timeliness_rating = COALESCE($4, timeliness_rating),
                communication_rating = COALESCE($5, communication_rating),
                value_rating = COALESCE($6, value_rating),
                review_title = COALESCE($7, review_title),
                review_text = COALESCE($8, review_text),
                updated_at = NOW()
            WHERE id = $1 AND reviewer_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(reviewer_id)
        .bind(data.quality_rating)
        .bind(data.timeliness_rating)
        .bind(data.communication_rating)
        .bind(data.value_rating)
        .bind(&data.review_title)
        .bind(&data.review_text)
        .fetch_optional(&self.pool)
        .await
    }

    /// Delete a review (soft delete by setting status to removed).
    pub async fn delete_review(&self, id: Uuid, reviewer_id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE provider_reviews SET status = 'removed', updated_at = NOW()
            WHERE id = $1 AND reviewer_id = $2
            "#,
        )
        .bind(id)
        .bind(reviewer_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Respond to a review (by provider).
    pub async fn respond_to_review(
        &self,
        id: Uuid,
        provider_id: Uuid,
        response_text: &str,
    ) -> Result<Option<ProviderReview>, SqlxError> {
        sqlx::query_as::<_, ProviderReview>(
            r#"
            UPDATE provider_reviews
            SET provider_response = $3, provider_responded_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND provider_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(provider_id)
        .bind(response_text)
        .fetch_optional(&self.pool)
        .await
    }

    /// Moderate a review (admin only).
    pub async fn moderate_review(
        &self,
        id: Uuid,
        moderated_by: Uuid,
        status: &str,
        moderation_notes: Option<String>,
    ) -> Result<Option<ProviderReview>, SqlxError> {
        sqlx::query_as::<_, ProviderReview>(
            r#"
            UPDATE provider_reviews
            SET
                status = $2,
                moderated_by = $3,
                moderated_at = NOW(),
                moderation_notes = $4,
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status)
        .bind(moderated_by)
        .bind(moderation_notes)
        .fetch_optional(&self.pool)
        .await
    }

    /// Mark a review as helpful.
    pub async fn mark_review_helpful(
        &self,
        review_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<ProviderReview>, SqlxError> {
        // Insert vote (ignore if already exists)
        sqlx::query(
            r#"
            INSERT INTO review_helpful_votes (review_id, user_id)
            VALUES ($1, $2)
            ON CONFLICT (review_id, user_id) DO NOTHING
            "#,
        )
        .bind(review_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        // Return updated review
        self.find_review_by_id(review_id).await
    }

    /// Get rating breakdown for a provider.
    pub async fn get_rating_breakdown(
        &self,
        provider_id: Uuid,
    ) -> Result<RatingBreakdown, SqlxError> {
        let stats = sqlx::query(
            r#"
            SELECT
                COALESCE(AVG(overall_rating), 0) as average_overall,
                COALESCE(AVG(quality_rating), 0) as average_quality,
                COALESCE(AVG(timeliness_rating), 0) as average_timeliness,
                COALESCE(AVG(communication_rating), 0) as average_communication,
                COALESCE(AVG(value_rating), 0) as average_value,
                COUNT(*) as total_reviews,
                COUNT(*) FILTER (WHERE overall_rating = 5) as five_star,
                COUNT(*) FILTER (WHERE overall_rating = 4) as four_star,
                COUNT(*) FILTER (WHERE overall_rating = 3) as three_star,
                COUNT(*) FILTER (WHERE overall_rating = 2) as two_star,
                COUNT(*) FILTER (WHERE overall_rating = 1) as one_star
            FROM provider_reviews
            WHERE provider_id = $1 AND status = 'approved'
            "#,
        )
        .bind(provider_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(RatingBreakdown {
            average_overall: stats.get("average_overall"),
            average_quality: stats.get("average_quality"),
            average_timeliness: stats.get("average_timeliness"),
            average_communication: stats.get("average_communication"),
            average_value: stats.get("average_value"),
            total_reviews: stats.get::<i64, _>("total_reviews"),
            rating_distribution: RatingDistribution {
                five_star: stats.get::<i64, _>("five_star"),
                four_star: stats.get::<i64, _>("four_star"),
                three_star: stats.get::<i64, _>("three_star"),
                two_star: stats.get::<i64, _>("two_star"),
                one_star: stats.get::<i64, _>("one_star"),
            },
        })
    }

    // ==================== Manager Dashboard ====================

    /// Get manager marketplace dashboard.
    pub async fn get_manager_dashboard(
        &self,
        organization_id: Uuid,
    ) -> Result<ManagerMarketplaceDashboard, SqlxError> {
        // Active RFQs
        let active_rfqs = sqlx::query_as::<_, RequestForQuote>(
            r#"
            SELECT * FROM rfqs
            WHERE organization_id = $1
            AND status IN ('draft', 'sent', 'quotes_received')
            ORDER BY created_at DESC
            LIMIT 10
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;

        // Pending quotes count
        let pending_quotes: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM provider_quotes pq
            JOIN rfqs r ON r.id = pq.rfq_id
            WHERE r.organization_id = $1 AND pq.status = 'submitted'
            "#,
        )
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await?;

        // Recent completed jobs
        let recent_completed_jobs: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM rfqs
            WHERE organization_id = $1
            AND status = 'awarded'
            AND awarded_at >= NOW() - INTERVAL '30 days'
            "#,
        )
        .bind(organization_id)
        .fetch_one(&self.pool)
        .await?;

        // Favorite providers
        let favorite_providers = sqlx::query_as::<_, ProviderSearchResult>(
            r#"
            SELECT
                sp.id, sp.company_name, sp.description, sp.logo_url, sp.service_categories,
                sp.city, sp.pricing_type, sp.hourly_rate_min, sp.hourly_rate_max, sp.currency,
                sp.average_rating, sp.total_reviews, sp.total_jobs_completed, sp.is_verified,
                sp.badges, sp.emergency_available, sp.response_time_hours
            FROM favorite_providers fp
            JOIN service_provider_profiles sp ON sp.id = fp.provider_id
            WHERE fp.organization_id = $1
            ORDER BY fp.added_at DESC
            LIMIT 5
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;

        // Recommended providers (top-rated in categories the org has used)
        let recommended_providers = sqlx::query_as::<_, ProviderSearchResult>(
            r#"
            SELECT DISTINCT ON (sp.id)
                sp.id, sp.company_name, sp.description, sp.logo_url, sp.service_categories,
                sp.city, sp.pricing_type, sp.hourly_rate_min, sp.hourly_rate_max, sp.currency,
                sp.average_rating, sp.total_reviews, sp.total_jobs_completed, sp.is_verified,
                sp.badges, sp.emergency_available, sp.response_time_hours
            FROM service_provider_profiles sp
            WHERE sp.status = 'active'
            AND sp.is_verified = TRUE
            AND sp.id NOT IN (
                SELECT provider_id FROM favorite_providers WHERE organization_id = $1
            )
            ORDER BY sp.id, sp.average_rating DESC NULLS LAST
            LIMIT 5
            "#,
        )
        .bind(organization_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ManagerMarketplaceDashboard {
            active_rfqs,
            pending_quotes,
            recent_completed_jobs,
            favorite_providers,
            recommended_providers,
        })
    }
}
