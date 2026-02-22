-- Epic 150: API Ecosystem Expansion
-- Migration for Integration Marketplace, Connector Framework, Webhooks, and Developer Portal

-- ============================================
-- Story 150.1: Integration Marketplace
-- ============================================

-- Marketplace integrations catalog
CREATE TABLE marketplace_integrations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(100) NOT NULL UNIQUE,
    name VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    long_description TEXT,
    category VARCHAR(50) NOT NULL DEFAULT 'other',
    icon_url TEXT,
    banner_url TEXT,
    vendor_name VARCHAR(255) NOT NULL,
    vendor_url TEXT,
    documentation_url TEXT,
    support_url TEXT,
    version VARCHAR(50) NOT NULL DEFAULT '1.0.0',
    status VARCHAR(20) NOT NULL DEFAULT 'available',
    features JSONB,
    requirements JSONB,
    pricing_info JSONB,
    rating_average DECIMAL(3,2),
    rating_count INTEGER NOT NULL DEFAULT 0,
    install_count INTEGER NOT NULL DEFAULT 0,
    is_featured BOOLEAN NOT NULL DEFAULT FALSE,
    is_premium BOOLEAN NOT NULL DEFAULT FALSE,
    required_scopes TEXT[],
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_category CHECK (category IN ('accounting', 'crm', 'calendar', 'communication', 'payment', 'property_portal', 'iot', 'analytics', 'document_management', 'other')),
    CONSTRAINT valid_status CHECK (status IN ('available', 'coming_soon', 'deprecated', 'maintenance')),
    CONSTRAINT valid_rating CHECK (rating_average IS NULL OR (rating_average >= 0 AND rating_average <= 5))
);

-- Organization integration installations
CREATE TABLE organization_integrations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    integration_id UUID NOT NULL REFERENCES marketplace_integrations(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    configuration JSONB,
    credentials_encrypted TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    last_sync_at TIMESTAMPTZ,
    last_error TEXT,
    installed_by UUID NOT NULL REFERENCES users(id),
    installed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_installation_status CHECK (status IN ('installed', 'pending', 'failed', 'uninstalled')),
    UNIQUE(organization_id, integration_id)
);

-- Integration ratings and reviews
CREATE TABLE integration_ratings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    integration_id UUID NOT NULL REFERENCES marketplace_integrations(id) ON DELETE CASCADE,
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id),
    rating INTEGER NOT NULL,
    review TEXT,
    helpful_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_rating_value CHECK (rating >= 1 AND rating <= 5),
    -- A user can rate an integration once (per user, not per organization)
    UNIQUE(integration_id, user_id)
);

-- ============================================
-- Story 150.2: Pre-Built Connector Framework
-- ============================================

-- Connector definitions
CREATE TABLE connectors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    integration_id UUID NOT NULL REFERENCES marketplace_integrations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    auth_type VARCHAR(20) NOT NULL DEFAULT 'api_key',
    auth_config JSONB,
    base_url TEXT NOT NULL,
    rate_limit_requests INTEGER,
    rate_limit_window_seconds INTEGER,
    retry_max_attempts INTEGER NOT NULL DEFAULT 3,
    retry_initial_delay_ms INTEGER NOT NULL DEFAULT 1000,
    retry_max_delay_ms INTEGER NOT NULL DEFAULT 30000,
    timeout_ms INTEGER NOT NULL DEFAULT 30000,
    headers JSONB,
    supported_actions TEXT[] NOT NULL DEFAULT '{}',
    error_mapping JSONB,
    data_transformations JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_auth_type CHECK (auth_type IN ('oauth2', 'api_key', 'basic', 'bearer_token', 'custom'))
);

-- Connector actions/endpoints
CREATE TABLE connector_actions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    connector_id UUID NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    method VARCHAR(10) NOT NULL DEFAULT 'GET',
    path TEXT NOT NULL,
    input_schema JSONB,
    output_schema JSONB,
    headers JSONB,
    query_params JSONB,
    body_template JSONB,
    response_mapping JSONB,
    is_paginated BOOLEAN NOT NULL DEFAULT FALSE,
    pagination_config JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_method CHECK (method IN ('GET', 'POST', 'PUT', 'PATCH', 'DELETE')),
    UNIQUE(connector_id, name)
);

-- Organization connector instances
CREATE TABLE organization_connectors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    connector_id UUID NOT NULL REFERENCES connectors(id) ON DELETE CASCADE,
    org_integration_id UUID REFERENCES organization_integrations(id) ON DELETE CASCADE,
    status VARCHAR(20) NOT NULL DEFAULT 'inactive',
    credentials_encrypted TEXT,
    custom_config JSONB,
    last_used_at TIMESTAMPTZ,
    request_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_connector_status CHECK (status IN ('active', 'inactive', 'error', 'rate_limited')),
    UNIQUE(organization_id, connector_id)
);

-- Connector execution logs
CREATE TABLE connector_execution_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_connector_id UUID NOT NULL REFERENCES organization_connectors(id) ON DELETE CASCADE,
    action_id UUID REFERENCES connector_actions(id) ON DELETE SET NULL,
    action_name VARCHAR(100) NOT NULL,
    request_payload JSONB,
    response_payload JSONB,
    status_code INTEGER,
    duration_ms INTEGER,
    error_message TEXT,
    executed_by UUID REFERENCES users(id),
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Story 150.3: Custom Webhooks
-- ============================================

-- Webhook subscriptions
CREATE TABLE webhook_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    url TEXT NOT NULL,
    secret TEXT NOT NULL,
    events TEXT[] NOT NULL,
    headers JSONB,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    retry_config JSONB,
    failure_threshold INTEGER NOT NULL DEFAULT 5,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    last_triggered_at TIMESTAMPTZ,
    last_success_at TIMESTAMPTZ,
    last_failure_at TIMESTAMPTZ,
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Webhook delivery logs
CREATE TABLE webhook_deliveries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    subscription_id UUID NOT NULL REFERENCES webhook_subscriptions(id) ON DELETE CASCADE,
    event_type VARCHAR(100) NOT NULL,
    payload JSONB NOT NULL,
    response_status INTEGER,
    response_body TEXT,
    response_headers JSONB,
    duration_ms INTEGER,
    attempt_number INTEGER NOT NULL DEFAULT 1,
    next_retry_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    error_message TEXT,
    delivered_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_delivery_status CHECK (status IN ('pending', 'delivered', 'failed', 'retrying'))
);

-- Webhook event types registry
CREATE TABLE webhook_event_types (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL UNIQUE,
    display_name VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(50) NOT NULL,
    payload_schema JSONB,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- Story 150.4: Developer Portal
-- ============================================

-- Developer accounts
CREATE TABLE developer_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE SET NULL,
    email VARCHAR(255) NOT NULL,
    company_name VARCHAR(255),
    website TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    use_case TEXT,
    approved_at TIMESTAMPTZ,
    approved_by UUID REFERENCES users(id),
    terms_accepted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT valid_developer_status CHECK (status IN ('pending', 'approved', 'rejected', 'suspended')),
    UNIQUE(user_id)
);

-- API keys
CREATE TABLE developer_api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_id UUID NOT NULL REFERENCES developer_accounts(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    key_prefix VARCHAR(8) NOT NULL,
    key_hash TEXT NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    rate_limit_tier VARCHAR(20) NOT NULL DEFAULT 'standard',
    is_sandbox BOOLEAN NOT NULL DEFAULT FALSE,
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_used_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    revoked_by UUID REFERENCES users(id),

    CONSTRAINT valid_rate_limit_tier CHECK (rate_limit_tier IN ('free', 'standard', 'premium', 'enterprise')),
    CONSTRAINT valid_api_key_status CHECK (status IN ('active', 'inactive', 'revoked', 'expired'))
);

-- API key usage logs
CREATE TABLE api_key_usage_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    api_key_id UUID NOT NULL REFERENCES developer_api_keys(id) ON DELETE CASCADE,
    endpoint VARCHAR(255) NOT NULL,
    method VARCHAR(10) NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address INET,
    user_agent TEXT,
    error_code VARCHAR(50),
    logged_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- OAuth applications
CREATE TABLE developer_oauth_apps (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_id UUID NOT NULL REFERENCES developer_accounts(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    client_id VARCHAR(100) NOT NULL UNIQUE,
    client_secret_hash TEXT NOT NULL,
    redirect_uris TEXT[] NOT NULL,
    scopes TEXT[] NOT NULL DEFAULT '{}',
    logo_url TEXT,
    homepage_url TEXT,
    privacy_policy_url TEXT,
    terms_of_service_url TEXT,
    is_confidential BOOLEAN NOT NULL DEFAULT TRUE,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- OAuth authorization grants
CREATE TABLE developer_oauth_grants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    oauth_app_id UUID NOT NULL REFERENCES developer_oauth_apps(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE,
    scopes TEXT[] NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,

    UNIQUE(oauth_app_id, user_id, organization_id)
);

-- Sandbox environments
CREATE TABLE developer_sandboxes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    developer_id UUID NOT NULL REFERENCES developer_accounts(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL DEFAULT 'Default Sandbox',
    seed_data_config JSONB,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    reset_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ
);

-- API documentation versions
CREATE TABLE api_documentation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version VARCHAR(20) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    openapi_spec JSONB NOT NULL,
    changelog TEXT,
    is_published BOOLEAN NOT NULL DEFAULT FALSE,
    published_at TIMESTAMPTZ,
    deprecated_at TIMESTAMPTZ,
    sunset_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(version)
);

-- Code samples
CREATE TABLE api_code_samples (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    endpoint VARCHAR(255) NOT NULL,
    method VARCHAR(10) NOT NULL,
    language VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    code TEXT NOT NULL,
    dependencies JSONB,
    is_verified BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE(endpoint, method, language)
);

-- ============================================
-- Indexes
-- ============================================

-- Marketplace integrations
CREATE INDEX idx_marketplace_integrations_category ON marketplace_integrations(category);
CREATE INDEX idx_marketplace_integrations_status ON marketplace_integrations(status);
CREATE INDEX idx_marketplace_integrations_featured ON marketplace_integrations(is_featured) WHERE is_featured = TRUE;
CREATE INDEX idx_marketplace_integrations_search ON marketplace_integrations USING gin(to_tsvector('english', name || ' ' || description));

-- Organization integrations
CREATE INDEX idx_org_integrations_org ON organization_integrations(organization_id);
CREATE INDEX idx_org_integrations_integration ON organization_integrations(integration_id);
CREATE INDEX idx_org_integrations_status ON organization_integrations(status);

-- Integration ratings
CREATE INDEX idx_integration_ratings_integration ON integration_ratings(integration_id);
CREATE INDEX idx_integration_ratings_user ON integration_ratings(user_id);

-- Connectors
CREATE INDEX idx_connectors_integration ON connectors(integration_id);

-- Connector actions
CREATE INDEX idx_connector_actions_connector ON connector_actions(connector_id);

-- Organization connectors
CREATE INDEX idx_org_connectors_org ON organization_connectors(organization_id);
CREATE INDEX idx_org_connectors_connector ON organization_connectors(connector_id);

-- Connector execution logs
CREATE INDEX idx_connector_logs_org_connector ON connector_execution_logs(org_connector_id);
CREATE INDEX idx_connector_logs_executed_at ON connector_execution_logs(executed_at);

-- Webhook subscriptions
CREATE INDEX idx_webhook_subscriptions_org ON webhook_subscriptions(organization_id);
CREATE INDEX idx_webhook_subscriptions_active ON webhook_subscriptions(is_active) WHERE is_active = TRUE;

-- Webhook deliveries
CREATE INDEX idx_webhook_deliveries_subscription ON webhook_deliveries(subscription_id);
CREATE INDEX idx_webhook_deliveries_status ON webhook_deliveries(status);
CREATE INDEX idx_webhook_deliveries_retry ON webhook_deliveries(next_retry_at) WHERE status = 'retrying';

-- Developer accounts
CREATE INDEX idx_developer_accounts_user ON developer_accounts(user_id);
CREATE INDEX idx_developer_accounts_status ON developer_accounts(status);

-- API keys
CREATE INDEX idx_api_keys_developer ON developer_api_keys(developer_id);
CREATE INDEX idx_api_keys_prefix ON developer_api_keys(key_prefix);
CREATE INDEX idx_api_keys_active ON developer_api_keys(is_active) WHERE is_active = TRUE;

-- API key usage logs
CREATE INDEX idx_api_key_usage_key ON api_key_usage_logs(api_key_id);
CREATE INDEX idx_api_key_usage_logged_at ON api_key_usage_logs(logged_at);

-- OAuth apps
CREATE INDEX idx_oauth_apps_developer ON developer_oauth_apps(developer_id);
CREATE INDEX idx_oauth_apps_client_id ON developer_oauth_apps(client_id);

-- OAuth grants
CREATE INDEX idx_oauth_grants_app ON developer_oauth_grants(oauth_app_id);
CREATE INDEX idx_oauth_grants_user ON developer_oauth_grants(user_id);

-- Sandboxes
CREATE INDEX idx_sandboxes_developer ON developer_sandboxes(developer_id);

-- ============================================
-- Row Level Security
-- ============================================

-- Enable RLS on all tables
ALTER TABLE marketplace_integrations ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_integrations ENABLE ROW LEVEL SECURITY;
ALTER TABLE integration_ratings ENABLE ROW LEVEL SECURITY;
ALTER TABLE connectors ENABLE ROW LEVEL SECURITY;
ALTER TABLE connector_actions ENABLE ROW LEVEL SECURITY;
ALTER TABLE organization_connectors ENABLE ROW LEVEL SECURITY;
ALTER TABLE connector_execution_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE webhook_subscriptions ENABLE ROW LEVEL SECURITY;
ALTER TABLE webhook_deliveries ENABLE ROW LEVEL SECURITY;
ALTER TABLE webhook_event_types ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_accounts ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_api_keys ENABLE ROW LEVEL SECURITY;
ALTER TABLE api_key_usage_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_oauth_apps ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_oauth_grants ENABLE ROW LEVEL SECURITY;
ALTER TABLE developer_sandboxes ENABLE ROW LEVEL SECURITY;
ALTER TABLE api_documentation ENABLE ROW LEVEL SECURITY;
ALTER TABLE api_code_samples ENABLE ROW LEVEL SECURITY;

-- Marketplace integrations: Public read, admin write
CREATE POLICY marketplace_integrations_read ON marketplace_integrations
    FOR SELECT USING (TRUE);

CREATE POLICY marketplace_integrations_write ON marketplace_integrations
    FOR ALL USING (
        COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- Organization integrations: Organization members only
CREATE POLICY org_integrations_policy ON organization_integrations
    FOR ALL USING (
        organization_id = NULLIF(current_setting('app.current_organization_id', TRUE), '')::UUID
    );

-- Integration ratings: Public read, authenticated write for own org
CREATE POLICY integration_ratings_read ON integration_ratings
    FOR SELECT USING (TRUE);

CREATE POLICY integration_ratings_write ON integration_ratings
    FOR ALL USING (
        organization_id = NULLIF(current_setting('app.current_organization_id', TRUE), '')::UUID
    );

-- Connectors: Public read (part of integration catalog)
CREATE POLICY connectors_read ON connectors
    FOR SELECT USING (TRUE);

CREATE POLICY connectors_write ON connectors
    FOR ALL USING (
        COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- Connector actions: Public read
CREATE POLICY connector_actions_read ON connector_actions
    FOR SELECT USING (TRUE);

CREATE POLICY connector_actions_write ON connector_actions
    FOR ALL USING (
        COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- Organization connectors: Organization members only
CREATE POLICY org_connectors_policy ON organization_connectors
    FOR ALL USING (
        organization_id = NULLIF(current_setting('app.current_organization_id', TRUE), '')::UUID
    );

-- Connector execution logs: Organization members only (via org_connector)
CREATE POLICY connector_logs_policy ON connector_execution_logs
    FOR ALL USING (
        org_connector_id IN (
            SELECT id FROM organization_connectors
            WHERE organization_id = NULLIF(current_setting('app.current_organization_id', TRUE), '')::UUID
        )
    );

-- Webhook subscriptions: Organization members only
CREATE POLICY webhook_subscriptions_policy ON webhook_subscriptions
    FOR ALL USING (
        organization_id = NULLIF(current_setting('app.current_organization_id', TRUE), '')::UUID
    );

-- Webhook deliveries: Organization members only (via subscription)
CREATE POLICY webhook_deliveries_policy ON webhook_deliveries
    FOR ALL USING (
        subscription_id IN (
            SELECT id FROM webhook_subscriptions
            WHERE organization_id = NULLIF(current_setting('app.current_organization_id', TRUE), '')::UUID
        )
    );

-- Webhook event types: Public read, admin write
CREATE POLICY webhook_event_types_read ON webhook_event_types
    FOR SELECT USING (TRUE);

CREATE POLICY webhook_event_types_write ON webhook_event_types
    FOR ALL USING (
        COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- Developer accounts: Owner or admin
CREATE POLICY developer_accounts_policy ON developer_accounts
    FOR ALL USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- API keys: Developer owner or admin
CREATE POLICY api_keys_policy ON developer_api_keys
    FOR ALL USING (
        developer_id IN (
            SELECT id FROM developer_accounts
            WHERE user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- API key usage: Developer owner or admin
CREATE POLICY api_key_usage_policy ON api_key_usage_logs
    FOR ALL USING (
        api_key_id IN (
            SELECT dk.id FROM developer_api_keys dk
            JOIN developer_accounts da ON dk.developer_id = da.id
            WHERE da.user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- OAuth apps: Developer owner or admin
CREATE POLICY oauth_apps_policy ON developer_oauth_apps
    FOR ALL USING (
        developer_id IN (
            SELECT id FROM developer_accounts
            WHERE user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- OAuth grants: User or admin
CREATE POLICY oauth_grants_policy ON developer_oauth_grants
    FOR ALL USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- Sandboxes: Developer owner or admin
CREATE POLICY sandboxes_policy ON developer_sandboxes
    FOR ALL USING (
        developer_id IN (
            SELECT id FROM developer_accounts
            WHERE user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        )
        OR COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- API documentation: Public read, admin write
CREATE POLICY api_documentation_read ON api_documentation
    FOR SELECT USING (is_published = TRUE OR COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE);

CREATE POLICY api_documentation_write ON api_documentation
    FOR ALL USING (
        COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- Code samples: Public read, admin write
CREATE POLICY code_samples_read ON api_code_samples
    FOR SELECT USING (TRUE);

CREATE POLICY code_samples_write ON api_code_samples
    FOR ALL USING (
        COALESCE(current_setting('app.is_super_admin', TRUE)::BOOLEAN, FALSE) = TRUE
    );

-- ============================================
-- Seed webhook event types
-- ============================================

INSERT INTO webhook_event_types (name, display_name, description, category, payload_schema) VALUES
    ('building.created', 'Building Created', 'Triggered when a new building is created', 'buildings', '{"type": "object", "properties": {"building_id": {"type": "string"}, "name": {"type": "string"}}}'),
    ('building.updated', 'Building Updated', 'Triggered when a building is updated', 'buildings', '{"type": "object", "properties": {"building_id": {"type": "string"}, "changes": {"type": "object"}}}'),
    ('unit.created', 'Unit Created', 'Triggered when a new unit is created', 'units', '{"type": "object", "properties": {"unit_id": {"type": "string"}, "building_id": {"type": "string"}}}'),
    ('unit.updated', 'Unit Updated', 'Triggered when a unit is updated', 'units', '{"type": "object", "properties": {"unit_id": {"type": "string"}, "changes": {"type": "object"}}}'),
    ('fault.created', 'Fault Reported', 'Triggered when a new fault is reported', 'faults', '{"type": "object", "properties": {"fault_id": {"type": "string"}, "title": {"type": "string"}, "priority": {"type": "string"}}}'),
    ('fault.status_changed', 'Fault Status Changed', 'Triggered when a fault status changes', 'faults', '{"type": "object", "properties": {"fault_id": {"type": "string"}, "old_status": {"type": "string"}, "new_status": {"type": "string"}}}'),
    ('fault.resolved', 'Fault Resolved', 'Triggered when a fault is resolved', 'faults', '{"type": "object", "properties": {"fault_id": {"type": "string"}, "resolved_by": {"type": "string"}}}'),
    ('vote.created', 'Vote Created', 'Triggered when a new vote is created', 'voting', '{"type": "object", "properties": {"vote_id": {"type": "string"}, "title": {"type": "string"}}}'),
    ('vote.completed', 'Vote Completed', 'Triggered when voting ends', 'voting', '{"type": "object", "properties": {"vote_id": {"type": "string"}, "results": {"type": "object"}}}'),
    ('invoice.created', 'Invoice Created', 'Triggered when a new invoice is created', 'financial', '{"type": "object", "properties": {"invoice_id": {"type": "string"}, "amount": {"type": "number"}}}'),
    ('invoice.paid', 'Invoice Paid', 'Triggered when an invoice is paid', 'financial', '{"type": "object", "properties": {"invoice_id": {"type": "string"}, "payment_id": {"type": "string"}}}'),
    ('lease.created', 'Lease Created', 'Triggered when a new lease is created', 'leases', '{"type": "object", "properties": {"lease_id": {"type": "string"}, "unit_id": {"type": "string"}}}'),
    ('lease.expiring', 'Lease Expiring', 'Triggered when a lease is about to expire', 'leases', '{"type": "object", "properties": {"lease_id": {"type": "string"}, "expires_at": {"type": "string"}}}'),
    ('work_order.created', 'Work Order Created', 'Triggered when a work order is created', 'maintenance', '{"type": "object", "properties": {"work_order_id": {"type": "string"}, "type": {"type": "string"}}}'),
    ('work_order.completed', 'Work Order Completed', 'Triggered when a work order is completed', 'maintenance', '{"type": "object", "properties": {"work_order_id": {"type": "string"}, "completed_by": {"type": "string"}}}');

-- ============================================
-- Triggers for updated_at
-- ============================================

-- Use CREATE OR REPLACE so this is idempotent across migrations
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_marketplace_integrations_updated_at
    BEFORE UPDATE ON marketplace_integrations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_integrations_updated_at
    BEFORE UPDATE ON organization_integrations
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_integration_ratings_updated_at
    BEFORE UPDATE ON integration_ratings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_connectors_updated_at
    BEFORE UPDATE ON connectors
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_organization_connectors_updated_at
    BEFORE UPDATE ON organization_connectors
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_webhook_subscriptions_updated_at
    BEFORE UPDATE ON webhook_subscriptions
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_developer_accounts_updated_at
    BEFORE UPDATE ON developer_accounts
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_developer_oauth_apps_updated_at
    BEFORE UPDATE ON developer_oauth_apps
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_api_documentation_updated_at
    BEFORE UPDATE ON api_documentation
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_api_code_samples_updated_at
    BEFORE UPDATE ON api_code_samples
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- ============================================
-- Trigger for rating aggregation
-- ============================================

CREATE OR REPLACE FUNCTION update_integration_rating_stats()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE marketplace_integrations
    SET
        rating_average = (
            SELECT AVG(rating)::DECIMAL(3,2)
            FROM integration_ratings
            WHERE integration_id = COALESCE(NEW.integration_id, OLD.integration_id)
        ),
        rating_count = (
            SELECT COUNT(*)
            FROM integration_ratings
            WHERE integration_id = COALESCE(NEW.integration_id, OLD.integration_id)
        ),
        updated_at = NOW()
    WHERE id = COALESCE(NEW.integration_id, OLD.integration_id);
    -- Return NEW for INSERT/UPDATE, OLD for DELETE
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_rating_stats_on_insert
    AFTER INSERT ON integration_ratings
    FOR EACH ROW EXECUTE FUNCTION update_integration_rating_stats();

CREATE TRIGGER update_rating_stats_on_update
    AFTER UPDATE ON integration_ratings
    FOR EACH ROW EXECUTE FUNCTION update_integration_rating_stats();

CREATE TRIGGER update_rating_stats_on_delete
    AFTER DELETE ON integration_ratings
    FOR EACH ROW EXECUTE FUNCTION update_integration_rating_stats();
