-- Fix RLS policies on device_push_tokens (migration 00157).
-- 00157 mistakenly referenced `rls.user_id`; the project convention is
-- `app.current_user_id` (set by set_request_context()).  Drop and recreate.

DROP POLICY IF EXISTS device_push_tokens_user_policy ON device_push_tokens;
DROP POLICY IF EXISTS device_push_tokens_service_policy ON device_push_tokens;

-- Enforce RLS even for the table owner (matches all other RLS tables).
ALTER TABLE device_push_tokens FORCE ROW LEVEL SECURITY;

-- Users may only read/write their own tokens; super-admins bypass.
CREATE POLICY device_push_tokens_user_policy
    ON device_push_tokens
    FOR ALL
    USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    );
