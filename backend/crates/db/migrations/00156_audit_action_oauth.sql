-- Migration: 00156_audit_action_oauth.sql
-- Epic 10A: OAuth 2.0 Authorization Server
-- Extends the `audit_action` Postgres ENUM with the five OAuth provider
-- variants introduced in backend/crates/db/src/models/audit_log.rs.
-- These are written by the OAuth consent, grant-revoke, and admin-client
-- handlers; without them every audit INSERT in those paths fails silently
-- and the audit-trail integration tests assert-fail on the zero row count.

ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'oauth_authorize';
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'oauth_revoke';
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'oauth_client_create';
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'oauth_client_revoke';
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'oauth_client_secret_regenerate';
