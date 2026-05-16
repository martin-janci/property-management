-- Migration: 00150_audit_action_oauth_token_denied.sql
-- Phase 6 (C17 / R5 follow-up): extend the `audit_action` Postgres ENUM with
-- the new `oauth_token_denied_principal_kind` variant introduced in
-- backend/crates/db/src/models/audit_log.rs.
--
-- Without this, the very first audit_log row written when an OAuth client
-- rejects a token issuance for a principal_kind mismatch would fail with
-- `invalid input value for enum audit_action: "oauth_token_denied_principal_kind"`,
-- silently dropping the rejection event we explicitly want to surface in
-- the unified admin audit viewer (leak #21 defense).

ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'oauth_token_denied_principal_kind';
