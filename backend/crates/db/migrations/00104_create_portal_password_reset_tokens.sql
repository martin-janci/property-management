-- Reality Portal: portal_users password reset (UC-44.3).
--
-- The api-server already has `password_reset_tokens` for PM users; the
-- reality-server has its own `portal_users` table, so we add a parallel
-- reset table that references portal_users(id). Same column shape and
-- cleanup strategy as 00003 to keep the repository implementations
-- mirrored.

CREATE TABLE IF NOT EXISTS portal_password_reset_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    portal_user_id UUID NOT NULL REFERENCES portal_users(id) ON DELETE CASCADE,
    token_hash VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Lookup by user (invalidate when a new token is issued / password changes).
CREATE INDEX IF NOT EXISTS idx_portal_password_reset_tokens_user
    ON portal_password_reset_tokens(portal_user_id);

-- Lookup by expiry for the cleanup job.
CREATE INDEX IF NOT EXISTS idx_portal_password_reset_tokens_expires
    ON portal_password_reset_tokens(expires_at)
    WHERE used_at IS NULL;
