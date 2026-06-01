-- Migration: 00173_audit_action_oauth_client_update.sql
-- Findings #768 / #800: privileged OAuth client mutations must be audited.
-- The admin-web OAuth-clients screen requires an operator reason for scope
-- edits, but `update_client` previously wrote no audit_log row and silently
-- dropped the reason. Extend the `audit_action` Postgres ENUM with the
-- `oauth_client_update` variant so the update handler can write an audit row
-- (with the captured reason) like create/revoke/secret-regenerate already do.

ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'oauth_client_update';
