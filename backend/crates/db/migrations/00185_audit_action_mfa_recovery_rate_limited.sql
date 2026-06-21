-- Migration: 00185_audit_action_mfa_recovery_rate_limited.sql
-- GH #1583: the MFA recovery-code verify rate-limit DENIAL (PR #1580) was
-- logged with `mfa_backup_code_used` — a SUCCESS action ("a backup code was
-- used") — disambiguated only via `resource_type`/`details`. That pollutes any
-- query/alert that counts `mfa_backup_code_used` as a successful MFA bypass and
-- makes the throttle event invisible to action-level filtering.
--
-- Add a dedicated denial variant so the rate-limit event is action-filterable,
-- following the `refresh_token_replay_detected` precedent (00154).
ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'mfa_recovery_rate_limited';
