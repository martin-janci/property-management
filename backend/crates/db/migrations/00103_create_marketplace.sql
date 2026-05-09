-- Epic 68: Service Provider Marketplace
-- Creates tables for service provider profiles, RFQs, quotes, verifications, badges, and reviews.

-- ==================== Service Provider Profiles (Story 68.1) ====================

CREATE TABLE IF NOT EXISTS service_provider_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Company information
    company_name VARCHAR(255) NOT NULL,
    business_registration_number VARCHAR(100),
    tax_id VARCHAR(100),
    description TEXT,
    logo_url TEXT,
    website VARCHAR(500),

    -- Contact information
    contact_name VARCHAR(255) NOT NULL,
    contact_email VARCHAR(255) NOT NULL,
    contact_phone VARCHAR(50),
    address TEXT,
    city VARCHAR(100),
    postal_code VARCHAR(20),
    country VARCHAR(100),

    -- Services offered
    service_categories TEXT[] NOT NULL DEFAULT '{}',
    service_description TEXT,
    specializations TEXT[],

    -- Coverage area
    coverage_postal_codes TEXT[],
    coverage_radius_km INTEGER,
    coverage_regions TEXT[],

    -- Pricing structure
    pricing_type VARCHAR(50) NOT NULL DEFAULT 'hourly',
    hourly_rate_min DECIMAL(12, 2),
    hourly_rate_max DECIMAL(12, 2),
    currency VARCHAR(3) DEFAULT 'EUR',

    -- Certifications and licenses (JSON)
    certifications JSONB,
    licenses JSONB,

    -- Availability
    availability_calendar JSONB,
    response_time_hours INTEGER,
    emergency_available BOOLEAN DEFAULT FALSE,

    -- Portfolio
    portfolio_images TEXT[],
    portfolio_description TEXT,

    -- Status and metrics
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    is_featured BOOLEAN DEFAULT FALSE,
    average_rating DECIMAL(3, 2),
    total_reviews INTEGER DEFAULT 0,
    total_jobs_completed INTEGER DEFAULT 0,

    -- Verification
    is_verified BOOLEAN DEFAULT FALSE,
    verified_at TIMESTAMPTZ,
    badges TEXT[] DEFAULT '{}',

    -- Metadata
    metadata JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_active_at TIMESTAMPTZ DEFAULT NOW(),

    -- Constraints
    CONSTRAINT valid_pricing_type CHECK (pricing_type IN ('hourly', 'project', 'fixed', 'quote_required')),
    CONSTRAINT valid_status CHECK (status IN ('draft', 'pending_review', 'active', 'suspended', 'inactive')),
    CONSTRAINT valid_hourly_rate CHECK (hourly_rate_min IS NULL OR hourly_rate_max IS NULL OR hourly_rate_min <= hourly_rate_max),
    CONSTRAINT unique_user_profile UNIQUE (user_id)
);

CREATE INDEX IF NOT EXISTS idx_provider_profiles_user ON service_provider_profiles(user_id);
CREATE INDEX IF NOT EXISTS idx_provider_profiles_status ON service_provider_profiles(status);
CREATE INDEX IF NOT EXISTS idx_provider_profiles_categories ON service_provider_profiles USING GIN(service_categories);
CREATE INDEX IF NOT EXISTS idx_provider_profiles_city ON service_provider_profiles(city);
CREATE INDEX IF NOT EXISTS idx_provider_profiles_rating ON service_provider_profiles(average_rating DESC);
CREATE INDEX IF NOT EXISTS idx_provider_profiles_verified ON service_provider_profiles(is_verified) WHERE is_verified = TRUE;
CREATE INDEX IF NOT EXISTS idx_provider_profiles_emergency ON service_provider_profiles(emergency_available) WHERE emergency_available = TRUE;
CREATE INDEX IF NOT EXISTS idx_provider_profiles_featured ON service_provider_profiles(is_featured) WHERE is_featured = TRUE;

-- ==================== Request for Quote (Story 68.3) ====================

CREATE TABLE IF NOT EXISTS rfqs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    building_id UUID REFERENCES buildings(id) ON DELETE SET NULL,
    created_by UUID NOT NULL REFERENCES users(id),

    -- RFQ details
    title VARCHAR(500) NOT NULL,
    description TEXT NOT NULL,
    service_category VARCHAR(100) NOT NULL,
    scope_of_work TEXT,

    -- Timeline
    preferred_start_date DATE,
    preferred_end_date DATE,
    is_urgent BOOLEAN DEFAULT FALSE,

    -- Budget
    budget_min DECIMAL(14, 2),
    budget_max DECIMAL(14, 2),
    currency VARCHAR(3) DEFAULT 'EUR',

    -- Attachments
    attachments JSONB,
    images TEXT[],

    -- Status and dates
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    quote_deadline TIMESTAMPTZ,
    awarded_to UUID REFERENCES service_provider_profiles(id),
    awarded_quote_id UUID,
    awarded_at TIMESTAMPTZ,

    -- Contact preferences
    contact_preference VARCHAR(50),
    site_visit_required BOOLEAN DEFAULT FALSE,

    -- Metadata
    metadata JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,

    -- Constraints
    CONSTRAINT valid_rfq_status CHECK (status IN ('draft', 'sent', 'quotes_received', 'awarded', 'cancelled', 'expired')),
    CONSTRAINT valid_budget_range CHECK (budget_min IS NULL OR budget_max IS NULL OR budget_min <= budget_max)
);

CREATE INDEX IF NOT EXISTS idx_rfqs_organization ON rfqs(organization_id);
CREATE INDEX IF NOT EXISTS idx_rfqs_building ON rfqs(building_id);
CREATE INDEX IF NOT EXISTS idx_rfqs_created_by ON rfqs(created_by);
CREATE INDEX IF NOT EXISTS idx_rfqs_status ON rfqs(status);
CREATE INDEX IF NOT EXISTS idx_rfqs_service_category ON rfqs(service_category);
CREATE INDEX IF NOT EXISTS idx_rfqs_deadline ON rfqs(quote_deadline);
CREATE INDEX IF NOT EXISTS idx_rfqs_awarded_to ON rfqs(awarded_to);

-- Enable RLS on RFQs
ALTER TABLE rfqs ENABLE ROW LEVEL SECURITY;

-- RLS policies for RFQs (organization members can access their org's RFQs)
CREATE POLICY rfqs_org_access ON rfqs
    FOR ALL
    USING (
        organization_id IN (
            SELECT organization_id FROM organization_members
            WHERE user_id = current_setting('app.user_id', true)::uuid
        )
    );

-- ==================== Provider Quotes (Story 68.3) ====================

CREATE TABLE IF NOT EXISTS provider_quotes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rfq_id UUID NOT NULL REFERENCES rfqs(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES service_provider_profiles(id) ON DELETE CASCADE,

    -- Quote details
    price DECIMAL(14, 2) NOT NULL,
    currency VARCHAR(3) NOT NULL DEFAULT 'EUR',
    price_breakdown JSONB,

    -- Timeline
    estimated_start_date DATE,
    estimated_end_date DATE,
    estimated_duration_days INTEGER,

    -- Terms
    terms_and_conditions TEXT,
    warranty_period_days INTEGER,
    payment_terms VARCHAR(255),

    -- Additional info
    notes TEXT,
    attachments JSONB,

    -- Status
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    valid_until TIMESTAMPTZ,

    -- Metadata
    metadata JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at TIMESTAMPTZ,

    -- Constraints
    CONSTRAINT valid_quote_status CHECK (status IN ('pending', 'submitted', 'accepted', 'rejected', 'withdrawn', 'expired')),
    CONSTRAINT unique_provider_per_rfq UNIQUE (rfq_id, provider_id)
);

CREATE INDEX IF NOT EXISTS idx_provider_quotes_rfq ON provider_quotes(rfq_id);
CREATE INDEX IF NOT EXISTS idx_provider_quotes_provider ON provider_quotes(provider_id);
CREATE INDEX IF NOT EXISTS idx_provider_quotes_status ON provider_quotes(status);
CREATE INDEX IF NOT EXISTS idx_provider_quotes_valid_until ON provider_quotes(valid_until);

-- Add foreign key for awarded_quote_id now that provider_quotes exists
ALTER TABLE rfqs ADD CONSTRAINT fk_awarded_quote
    FOREIGN KEY (awarded_quote_id) REFERENCES provider_quotes(id) ON DELETE SET NULL;

-- ==================== RFQ Invitations (Story 68.3) ====================

CREATE TABLE IF NOT EXISTS rfq_invitations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    rfq_id UUID NOT NULL REFERENCES rfqs(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES service_provider_profiles(id) ON DELETE CASCADE,

    -- Status tracking
    invited_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    viewed_at TIMESTAMPTZ,
    responded_at TIMESTAMPTZ,
    declined BOOLEAN DEFAULT FALSE,
    decline_reason TEXT,

    -- Constraints
    CONSTRAINT unique_invitation_per_rfq_provider UNIQUE (rfq_id, provider_id)
);

CREATE INDEX IF NOT EXISTS idx_rfq_invitations_rfq ON rfq_invitations(rfq_id);
CREATE INDEX IF NOT EXISTS idx_rfq_invitations_provider ON rfq_invitations(provider_id);
CREATE INDEX IF NOT EXISTS idx_rfq_invitations_declined ON rfq_invitations(declined) WHERE declined = FALSE;

-- ==================== Provider Verifications (Story 68.4) ====================

CREATE TABLE IF NOT EXISTS provider_verifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id UUID NOT NULL REFERENCES service_provider_profiles(id) ON DELETE CASCADE,

    -- Verification details
    verification_type VARCHAR(100) NOT NULL,
    document_name VARCHAR(255) NOT NULL,
    document_number VARCHAR(100),
    issuing_authority VARCHAR(255),
    issue_date DATE,
    expiry_date DATE,

    -- Document upload
    document_url TEXT,
    document_hash VARCHAR(128),

    -- Review info
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    reviewed_by UUID REFERENCES users(id),
    reviewed_at TIMESTAMPTZ,
    rejection_reason TEXT,
    notes TEXT,

    -- Metadata
    metadata JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT valid_verification_type CHECK (verification_type IN ('business_registration', 'insurance', 'certification', 'license', 'identity')),
    CONSTRAINT valid_verification_status CHECK (status IN ('pending', 'under_review', 'verified', 'rejected', 'expired'))
);

CREATE INDEX IF NOT EXISTS idx_provider_verifications_provider ON provider_verifications(provider_id);
CREATE INDEX IF NOT EXISTS idx_provider_verifications_type ON provider_verifications(verification_type);
CREATE INDEX IF NOT EXISTS idx_provider_verifications_status ON provider_verifications(status);
CREATE INDEX IF NOT EXISTS idx_provider_verifications_expiry ON provider_verifications(expiry_date);
CREATE INDEX IF NOT EXISTS idx_provider_verifications_pending ON provider_verifications(status, created_at) WHERE status = 'pending';

-- ==================== Provider Badges (Story 68.4) ====================

CREATE TABLE IF NOT EXISTS provider_badges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id UUID NOT NULL REFERENCES service_provider_profiles(id) ON DELETE CASCADE,
    badge_type VARCHAR(100) NOT NULL,

    -- Award info
    awarded_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    awarded_by UUID REFERENCES users(id),
    expires_at TIMESTAMPTZ,
    verification_id UUID REFERENCES provider_verifications(id),
    notes TEXT,

    -- Constraints
    CONSTRAINT valid_badge_type CHECK (badge_type IN ('verified_business', 'insured', 'certified', 'top_rated', 'fast_responder', 'preferred')),
    CONSTRAINT unique_badge_per_provider UNIQUE (provider_id, badge_type)
);

CREATE INDEX IF NOT EXISTS idx_provider_badges_provider ON provider_badges(provider_id);
CREATE INDEX IF NOT EXISTS idx_provider_badges_type ON provider_badges(badge_type);
CREATE INDEX IF NOT EXISTS idx_provider_badges_expiry ON provider_badges(expires_at) WHERE expires_at IS NOT NULL;

-- ==================== Provider Reviews (Story 68.5) ====================

CREATE TABLE IF NOT EXISTS provider_reviews (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_id UUID NOT NULL REFERENCES service_provider_profiles(id) ON DELETE CASCADE,
    reviewer_id UUID NOT NULL REFERENCES users(id),
    organization_id UUID NOT NULL REFERENCES organizations(id),
    job_id UUID,
    rfq_id UUID REFERENCES rfqs(id),

    -- Multi-dimension ratings (1-5)
    quality_rating INTEGER NOT NULL CHECK (quality_rating BETWEEN 1 AND 5),
    timeliness_rating INTEGER NOT NULL CHECK (timeliness_rating BETWEEN 1 AND 5),
    communication_rating INTEGER NOT NULL CHECK (communication_rating BETWEEN 1 AND 5),
    value_rating INTEGER NOT NULL CHECK (value_rating BETWEEN 1 AND 5),
    overall_rating INTEGER NOT NULL CHECK (overall_rating BETWEEN 1 AND 5),

    -- Review content
    review_title VARCHAR(255),
    review_text TEXT,

    -- Moderation
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    moderated_by UUID REFERENCES users(id),
    moderated_at TIMESTAMPTZ,
    moderation_notes TEXT,

    -- Provider response
    provider_response TEXT,
    provider_responded_at TIMESTAMPTZ,

    -- Helpful votes
    helpful_count INTEGER DEFAULT 0,

    -- Metadata
    metadata JSONB,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT valid_review_status CHECK (status IN ('pending', 'approved', 'flagged', 'removed')),
    CONSTRAINT unique_review_per_rfq UNIQUE (provider_id, reviewer_id, rfq_id)
);

CREATE INDEX IF NOT EXISTS idx_provider_reviews_provider ON provider_reviews(provider_id);
CREATE INDEX IF NOT EXISTS idx_provider_reviews_reviewer ON provider_reviews(reviewer_id);
CREATE INDEX IF NOT EXISTS idx_provider_reviews_organization ON provider_reviews(organization_id);
CREATE INDEX IF NOT EXISTS idx_provider_reviews_rfq ON provider_reviews(rfq_id);
CREATE INDEX IF NOT EXISTS idx_provider_reviews_status ON provider_reviews(status);
CREATE INDEX IF NOT EXISTS idx_provider_reviews_rating ON provider_reviews(overall_rating DESC);
CREATE INDEX IF NOT EXISTS idx_provider_reviews_created ON provider_reviews(created_at DESC);

-- Enable RLS on provider_reviews
ALTER TABLE provider_reviews ENABLE ROW LEVEL SECURITY;

-- RLS policies for reviews
CREATE POLICY reviews_read_all ON provider_reviews
    FOR SELECT
    USING (status = 'approved' OR reviewer_id = current_setting('app.user_id', true)::uuid);

CREATE POLICY reviews_insert ON provider_reviews
    FOR INSERT
    WITH CHECK (
        organization_id IN (
            SELECT organization_id FROM organization_members
            WHERE user_id = current_setting('app.user_id', true)::uuid
        )
    );

CREATE POLICY reviews_update_own ON provider_reviews
    FOR UPDATE
    USING (reviewer_id = current_setting('app.user_id', true)::uuid);

CREATE POLICY reviews_delete_own ON provider_reviews
    FOR DELETE
    USING (reviewer_id = current_setting('app.user_id', true)::uuid);

-- ==================== Review Helpful Votes (Story 68.5) ====================

CREATE TABLE IF NOT EXISTS review_helpful_votes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    review_id UUID NOT NULL REFERENCES provider_reviews(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    voted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT unique_vote_per_user_review UNIQUE (review_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_review_helpful_votes_review ON review_helpful_votes(review_id);
CREATE INDEX IF NOT EXISTS idx_review_helpful_votes_user ON review_helpful_votes(user_id);

-- ==================== Favorite Providers (Story 68.2) ====================

CREATE TABLE IF NOT EXISTS favorite_providers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    provider_id UUID NOT NULL REFERENCES service_provider_profiles(id) ON DELETE CASCADE,
    added_by UUID NOT NULL REFERENCES users(id),
    added_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    notes TEXT,

    CONSTRAINT unique_favorite_per_org_provider UNIQUE (organization_id, provider_id)
);

CREATE INDEX IF NOT EXISTS idx_favorite_providers_org ON favorite_providers(organization_id);
CREATE INDEX IF NOT EXISTS idx_favorite_providers_provider ON favorite_providers(provider_id);

-- Enable RLS
ALTER TABLE favorite_providers ENABLE ROW LEVEL SECURITY;

CREATE POLICY favorites_org_access ON favorite_providers
    FOR ALL
    USING (
        organization_id IN (
            SELECT organization_id FROM organization_members
            WHERE user_id = current_setting('app.user_id', true)::uuid
        )
    );

-- ==================== Triggers ====================

-- Update updated_at timestamp
CREATE OR REPLACE FUNCTION update_marketplace_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_service_provider_profiles_updated_at
    BEFORE UPDATE ON service_provider_profiles
    FOR EACH ROW EXECUTE FUNCTION update_marketplace_updated_at();

CREATE TRIGGER update_rfqs_updated_at
    BEFORE UPDATE ON rfqs
    FOR EACH ROW EXECUTE FUNCTION update_marketplace_updated_at();

CREATE TRIGGER update_provider_quotes_updated_at
    BEFORE UPDATE ON provider_quotes
    FOR EACH ROW EXECUTE FUNCTION update_marketplace_updated_at();

CREATE TRIGGER update_provider_verifications_updated_at
    BEFORE UPDATE ON provider_verifications
    FOR EACH ROW EXECUTE FUNCTION update_marketplace_updated_at();

CREATE TRIGGER update_provider_reviews_updated_at
    BEFORE UPDATE ON provider_reviews
    FOR EACH ROW EXECUTE FUNCTION update_marketplace_updated_at();

-- Update provider rating when reviews change
CREATE OR REPLACE FUNCTION update_provider_rating()
RETURNS TRIGGER AS $$
BEGIN
    -- Only count approved reviews
    UPDATE service_provider_profiles
    SET
        average_rating = (
            SELECT ROUND(AVG(overall_rating)::numeric, 2)
            FROM provider_reviews
            WHERE provider_id = COALESCE(NEW.provider_id, OLD.provider_id)
            AND status = 'approved'
        ),
        total_reviews = (
            SELECT COUNT(*)
            FROM provider_reviews
            WHERE provider_id = COALESCE(NEW.provider_id, OLD.provider_id)
            AND status = 'approved'
        ),
        updated_at = NOW()
    WHERE id = COALESCE(NEW.provider_id, OLD.provider_id);

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_provider_rating_on_review
    AFTER INSERT OR UPDATE OR DELETE ON provider_reviews
    FOR EACH ROW EXECUTE FUNCTION update_provider_rating();

-- Update helpful count when votes change
CREATE OR REPLACE FUNCTION update_review_helpful_count()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE provider_reviews
    SET helpful_count = (
        SELECT COUNT(*)
        FROM review_helpful_votes
        WHERE review_id = COALESCE(NEW.review_id, OLD.review_id)
    )
    WHERE id = COALESCE(NEW.review_id, OLD.review_id);

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_helpful_count_on_vote
    AFTER INSERT OR DELETE ON review_helpful_votes
    FOR EACH ROW EXECUTE FUNCTION update_review_helpful_count();

-- Update provider badges array when badges change
CREATE OR REPLACE FUNCTION sync_provider_badges()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE service_provider_profiles
    SET badges = (
        SELECT ARRAY_AGG(badge_type)
        FROM provider_badges
        WHERE provider_id = COALESCE(NEW.provider_id, OLD.provider_id)
        AND (expires_at IS NULL OR expires_at > NOW())
    ),
    updated_at = NOW()
    WHERE id = COALESCE(NEW.provider_id, OLD.provider_id);

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER sync_badges_on_change
    AFTER INSERT OR UPDATE OR DELETE ON provider_badges
    FOR EACH ROW EXECUTE FUNCTION sync_provider_badges();
