-- Epic 10B, Story 10B.6: User Onboarding Progress — RLS Policies
--
-- Adds Row-Level Security to user_onboarding_progress so each user
-- can only read and modify their own progress rows.
-- onboarding_tours is a global config table (no user-scoped RLS needed).

-- Enable RLS on user_onboarding_progress
ALTER TABLE user_onboarding_progress ENABLE ROW LEVEL SECURITY;
-- Force RLS for the table owner (superusers always bypass RLS regardless;
-- this applies to any non-superuser role that owns the table)
ALTER TABLE user_onboarding_progress FORCE ROW LEVEL SECURITY;

-- Policy: users can only SELECT their own rows; super-admins bypass
CREATE POLICY user_onboarding_progress_select ON user_onboarding_progress
    FOR SELECT
    USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    );

-- Policy: users can only INSERT rows for themselves; super-admins bypass
CREATE POLICY user_onboarding_progress_insert ON user_onboarding_progress
    FOR INSERT
    WITH CHECK (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    );

-- Policy: users can only UPDATE their own rows; super-admins bypass
CREATE POLICY user_onboarding_progress_update ON user_onboarding_progress
    FOR UPDATE
    USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    );

-- Policy: users can only DELETE their own rows; super-admins bypass
CREATE POLICY user_onboarding_progress_delete ON user_onboarding_progress
    FOR DELETE
    USING (
        user_id = NULLIF(current_setting('app.current_user_id', TRUE), '')::UUID
        OR is_super_admin()
    );

COMMENT ON TABLE user_onboarding_progress IS 'Tracks user progress through onboarding tours (Epic 10B, Story 10B.6) — RLS enforced per user_id';
