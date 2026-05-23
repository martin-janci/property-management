-- Migration: 00154_audit_action_refresh_replay.sql
-- Dev-team review P1-01 (RFC 9700 refresh-token rotation): extend the
-- `audit_action` Postgres ENUM with `refresh_token_replay_detected`.
--
-- Emitted when the refresh-token endpoint sees a token whose hash matches
-- an already-revoked row; this is the OAuth 2.1 / RFC 9700 "stolen-token
-- replay" signal. On detection the api-server fans out a revocation
-- across every active refresh token for the user (we don't carry a
-- family_id column yet — this approximates family invalidation by
-- targeting all of the user's tokens) and writes one audit row using
-- this variant so SOC tooling can alert on it.

ALTER TYPE audit_action ADD VALUE IF NOT EXISTS 'refresh_token_replay_detected';
