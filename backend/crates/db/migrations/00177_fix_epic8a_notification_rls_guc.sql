-- Migration: 00177_fix_epic8a_notification_rls_guc
-- Security Fix (issue #755, findings #4 & #5): correct the RLS GUC names on the
-- Epic 8A notification tables and close the owner-bypass gap.
--
-- Background
-- ----------
-- 00021 (`notification_preferences`) and 00022 (`critical_notifications` /
-- `critical_notification_acknowledgments`) wrote their RLS policies against the
-- legacy `request.user_id` / `request.org_id` GUCs. No code path ever sets
-- those GUCs. The api-server's `RlsConnection` extractor sets
-- `app.current_user_id` / `app.current_org_id` (see 00159, 00171 for the
-- convention spelled out verbatim). The mismatch means:
--
--   * `notification_preferences` is reached through `RlsConnection` (the repo's
--     `*_rls` methods). With `request.user_id` unset, the policy collapses to
--     the all-zeros UUID and matches nothing — the RLS path is silently broken.
--   * `critical_notifications` is reached through a raw service-role pool, so
--     `ENABLE` (without `FORCE`) is owner-bypassed and the broken policies are
--     simply never exercised — defense-in-depth that does nothing.
--
-- This migration drops and recreates the policies against the correct
-- `app.*` GUCs (plus `is_super_admin()` bypass, matching every other RLS table)
-- and adds `FORCE ROW LEVEL SECURITY`. The critical-notification tables keep a
-- service-role allowance (GUC unset => permitted) so the existing raw-pool
-- repository keeps working, mirroring `device_push_tokens_service_policy`
-- (00159).

-- ============================================================================
-- notification_preferences (00021): request.user_id -> app.current_user_id
-- ============================================================================

ALTER TABLE notification_preferences FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS notification_preferences_user_access ON notification_preferences;

CREATE POLICY notification_preferences_user_access ON notification_preferences
    FOR ALL
    USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    )
    WITH CHECK (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    );

-- ============================================================================
-- critical_notifications (00022): request.* -> app.*
-- Raw service-role pool sets no GUC, so a service allowance keeps the existing
-- CriticalNotificationRepository working while FORCE closes owner-bypass.
-- ============================================================================

ALTER TABLE critical_notifications FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS critical_notifications_org_read ON critical_notifications;
DROP POLICY IF EXISTS critical_notifications_insert ON critical_notifications;

-- Org members read their org's critical notifications; service role (no user
-- GUC set) and super-admins may read for delivery fan-out / admin tooling.
CREATE POLICY critical_notifications_org_read ON critical_notifications
    FOR SELECT
    USING (
        organization_id = NULLIF(current_setting('app.current_org_id', TRUE), '')::UUID
        OR NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL
        OR is_super_admin()
    );

-- Inserts are creator-attributed; service role / super-admin may insert.
CREATE POLICY critical_notifications_insert ON critical_notifications
    FOR INSERT
    WITH CHECK (
        created_by = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL
        OR is_super_admin()
    );

-- ============================================================================
-- critical_notification_acknowledgments (00022): request.* -> app.*
-- ============================================================================

ALTER TABLE critical_notification_acknowledgments FORCE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS critical_acks_read ON critical_notification_acknowledgments;
DROP POLICY IF EXISTS critical_acks_insert ON critical_notification_acknowledgments;

CREATE POLICY critical_acks_read ON critical_notification_acknowledgments
    FOR SELECT
    USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL
        OR is_super_admin()
        OR EXISTS (
            SELECT 1 FROM critical_notifications cn
            WHERE cn.id = notification_id
              AND cn.organization_id
                  = NULLIF(current_setting('app.current_org_id', TRUE), '')::UUID
        )
    );

CREATE POLICY critical_acks_insert ON critical_notification_acknowledgments
    FOR INSERT
    WITH CHECK (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR NULLIF(current_setting('app.current_user_id', TRUE), '') IS NULL
        OR is_super_admin()
    );
