-- Epic 8A-3: Mobile OS push notification registration
-- Creates the device_push_tokens table to store FCM/APNs device tokens
-- so the notification pipeline can deliver push notifications to real devices.

CREATE TABLE device_push_tokens (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- FCM registration token (Android) or APNs device token (iOS)
    token       TEXT        NOT NULL,
    -- 'fcm' | 'apns'
    platform    TEXT        NOT NULL CHECK (platform IN ('fcm', 'apns')),
    -- Optional: app bundle ID / package name for multi-app environments
    app_id      TEXT,
    -- Optional: human-readable device name for diagnostics
    device_name TEXT,
    -- Track when a token was last seen active (refresh on every app open)
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- One token string is globally unique: if a token is re-registered by
    -- another user (device transfer), upsert replaces the old row.
    CONSTRAINT device_push_tokens_token_unique UNIQUE (token)
);

-- Fast lookup of all tokens for a given user (used by FcmPushAdapter)
CREATE INDEX device_push_tokens_user_id_idx ON device_push_tokens (user_id);

-- RLS: users can only see / manage their own tokens.
ALTER TABLE device_push_tokens ENABLE ROW LEVEL SECURITY;
-- Force RLS even for table owner (consistent with other tenant-scoped tables).
ALTER TABLE device_push_tokens FORCE ROW LEVEL SECURITY;

-- Policy: each user can only access their own push tokens.
CREATE POLICY device_push_tokens_user_policy
    ON device_push_tokens
    FOR ALL
    USING (user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID);

-- Service-role bypass: allow SELECT when no user session is active
-- (background push-delivery task looks up tokens for all users).
CREATE POLICY device_push_tokens_service_policy
    ON device_push_tokens
    FOR SELECT
    USING (NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL);
