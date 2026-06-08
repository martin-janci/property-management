-- Gap 83-1: Airbnb OAuth token rotation.
--
-- Adds two dedicated encrypted-token columns to rental_platform_connections so
-- that the Airbnb access token and refresh token each have a clearly-named home
-- that cannot be confused with Booking.com's (username, password) pair which also
-- occupies the generic (access_token, refresh_token) pair.
--
-- Migration strategy:
--   1. Add columns as nullable.
--   2. Backfill from the generic columns for Airbnb rows (data unchanged, same
--      ciphertext — no re-encryption required).
--   The columns intentionally remain nullable: they are NULL for non-Airbnb
--   rows (Booking.com and future platforms do not use these).
--
-- After this migration the application writes both columns for Airbnb
-- connections: the generic columns are kept for backward-compat until a
-- future migration drops them, but the canonical source of truth for
-- Airbnb OAuth tokens is encrypted_token / encrypted_refresh_token.

ALTER TABLE rental_platform_connections
    ADD COLUMN IF NOT EXISTS encrypted_token         TEXT,
    ADD COLUMN IF NOT EXISTS encrypted_refresh_token TEXT;

-- Backfill from generic columns for existing Airbnb connections.
UPDATE rental_platform_connections
SET
    encrypted_token         = access_token,
    encrypted_refresh_token = refresh_token
WHERE
    platform = 'airbnb'
    AND access_token IS NOT NULL;

-- Index to efficiently find Airbnb connections that are near-expiry so the
-- background refresh scheduler can query them without a full table scan.
-- (An equivalent index on token_expires_at already exists through
--  idx_rental_platform_connections_active, but it is a partial index on
--  is_active only. This index filters on platform + expiry directly.)
CREATE INDEX IF NOT EXISTS idx_airbnb_connections_token_expires
    ON rental_platform_connections (token_expires_at)
    WHERE platform = 'airbnb'
      AND is_active = true
      AND encrypted_refresh_token IS NOT NULL;

COMMENT ON COLUMN rental_platform_connections.encrypted_token IS
    'AES-256-GCM encrypted Airbnb OAuth access token (enc: prefix). '
    'NULL for non-Airbnb connections. Canonical after gap-83-1.';

COMMENT ON COLUMN rental_platform_connections.encrypted_refresh_token IS
    'AES-256-GCM encrypted Airbnb OAuth refresh token (enc: prefix). '
    'NULL when no refresh token was issued (rare) or for non-Airbnb connections.';
