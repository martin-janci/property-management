-- Allow the service-role pool to DELETE device_push_tokens rows.
--
-- Migration 00159_fix_device_push_tokens_rls.sql created
-- `device_push_tokens_service_policy` FOR SELECT only.  With FORCE ROW LEVEL
-- SECURITY the service pool (no app.current_user_id set) cannot issue DELETE,
-- so `delete_stale_token` silently removed 0 rows and stale FCM/APNs tokens
-- were never evicted (Epic 8A-3 blocker).
--
-- We add a dedicated FOR DELETE policy with the same USING predicate so the
-- service role can evict tokens whose gateway has signalled NOT_REGISTERED.

CREATE POLICY device_push_tokens_service_delete_policy
    ON device_push_tokens
    FOR DELETE
    USING (NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL);
