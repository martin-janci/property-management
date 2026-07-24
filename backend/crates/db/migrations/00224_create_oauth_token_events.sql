-- Epic 10A (data audit): OAuth token-usage analytics.
-- Records issuance / refresh / revocation events per OAuth client so the
-- platform-admin ecosystem-health dashboard can report first-party OAuth
-- token usage. System-scoped like the other oauth_* provider tables
-- (see 00028_create_oauth_provider.sql): no org_id / no tenant RLS — these
-- rows are platform-level and read only by super admins.

CREATE TABLE IF NOT EXISTS oauth_token_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Lifecycle event: 'issued' | 'refreshed' | 'revoked'
    event_type VARCHAR(16) NOT NULL CHECK (event_type IN ('issued', 'refreshed', 'revoked')),
    client_id VARCHAR(64) NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    -- Nullable: client-level revocations are not tied to a single user.
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    -- Which token the event concerns: 'access' | 'refresh'
    token_type VARCHAR(16) NOT NULL DEFAULT 'access' CHECK (token_type IN ('access', 'refresh')),
    -- OAuth grant_type for issuance/refresh events (authorization_code, refresh_token, …)
    grant_type VARCHAR(32),
    scopes JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dashboard queries roll up per client, over a recent time window.
CREATE INDEX IF NOT EXISTS idx_oauth_token_events_client_id ON oauth_token_events(client_id);
CREATE INDEX IF NOT EXISTS idx_oauth_token_events_created_at ON oauth_token_events(created_at);
CREATE INDEX IF NOT EXISTS idx_oauth_token_events_client_created
    ON oauth_token_events(client_id, created_at);
CREATE INDEX IF NOT EXISTS idx_oauth_token_events_event_type ON oauth_token_events(event_type);

-- Note: like the other oauth_* provider tables, RLS is intentionally omitted.
-- These are platform-level analytics rows written by the OAuth service and
-- read only through super-admin dashboards.
