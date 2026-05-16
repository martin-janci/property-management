-- Phase 6 — C17: enforce principal_kind at OAuth token issuance.
--
-- Adds `allowed_principal_kinds` to `oauth_clients` so each client can
-- restrict which user kinds may authenticate through it:
--
--   'public'   — public portal users (formerly portal_users table)
--   'staff'    — agency staff (the historical PM users)
--   'platform' — platform / super-admin principals
--
-- Default is all three kinds to preserve backward compatibility for all
-- existing clients created before this migration.

ALTER TABLE oauth_clients
    ADD COLUMN IF NOT EXISTS allowed_principal_kinds TEXT[] NOT NULL
        DEFAULT ARRAY['public', 'staff', 'platform'];

COMMENT ON COLUMN oauth_clients.allowed_principal_kinds IS
    'Phase 6 C17: restricts which principal_kind values may obtain tokens via this client. Defaults to all kinds for backward compatibility.';
