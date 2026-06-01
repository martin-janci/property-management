-- Migration: 00171_force_messaging_rls
-- Security Fix (issue #898): FORCE row-level security on the messaging tables.
--
-- Background
-- ----------
-- 00017 created `message_threads`, `messages` and `user_blocks` with
-- `ENABLE ROW LEVEL SECURITY` only, and 00019 tightened the *policies* to add
-- organization-level isolation. But neither migration ever issued
-- `FORCE ROW LEVEL SECURITY`.
--
-- `ENABLE` (without `FORCE`) is bypassed for the table owner and for
-- superuser/`BYPASSRLS` roles. The api-server's `RlsConnection` extractor sets
-- `app.current_user_id` / `app.current_org_id` and treats the database RLS
-- policies as defense-in-depth behind the handler-layer participant/tenant
-- checks. That defense-in-depth is silently ineffective whenever the app
-- connects as the table owner — exactly the gap raised in the Epic 6 messaging
-- RLS spot-check (#898).
--
-- Every other security-sensitive RLS table in the schema is `FORCE`d
-- (organization_members, roles 00006; listings 00110; agencies 00111; rentals
-- 00112; reality-portal 00113; device_push_tokens 00159). 00159 spells out the
-- convention verbatim: "Enforce RLS even for the table owner (matches all other
-- RLS tables)." This migration brings the messaging tables in line.
--
-- Policies are unchanged — only owner-bypass is closed.

ALTER TABLE message_threads FORCE ROW LEVEL SECURITY;
ALTER TABLE messages FORCE ROW LEVEL SECURITY;
ALTER TABLE user_blocks FORCE ROW LEVEL SECURITY;
