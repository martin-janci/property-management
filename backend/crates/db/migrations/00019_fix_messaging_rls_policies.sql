-- Migration: 00019_fix_messaging_rls_policies
-- Security Fix: Add organization_id verification to RLS policies (Critical 1.2)
--
-- This migration updates the RLS policies for messaging tables to include
-- organization-level isolation in addition to user-level checks.

-- ============================================================================
-- DROP EXISTING POLICIES
-- ============================================================================

DROP POLICY IF EXISTS message_threads_tenant_isolation ON message_threads;
DROP POLICY IF EXISTS messages_tenant_isolation ON messages;
DROP POLICY IF EXISTS user_blocks_owner_isolation ON user_blocks;

-- ============================================================================
-- CREATE IMPROVED RLS POLICIES
-- ============================================================================

-- Message threads: Users can only see threads they participate in AND within their org
-- The organization_id check provides defense-in-depth with the application layer
CREATE POLICY message_threads_tenant_isolation ON message_threads
    FOR ALL
    USING (
        current_setting('app.current_user_id', true)::UUID = ANY(participant_ids)
        AND organization_id = COALESCE(
            current_setting('app.current_org_id', true)::UUID,
            organization_id  -- Fallback if not set (backward compat)
        )
    );

-- Messages: Users can only see messages in threads they participate in within their org
CREATE POLICY messages_tenant_isolation ON messages
    FOR ALL
    USING (
        thread_id IN (
            SELECT id FROM message_threads
            WHERE current_setting('app.current_user_id', true)::UUID = ANY(participant_ids)
            AND organization_id = COALESCE(
                current_setting('app.current_org_id', true)::UUID,
                organization_id
            )
        )
    );

-- User blocks: Users can only see/manage their own blocks
-- Also verify they're operating within their organization context
CREATE POLICY user_blocks_owner_isolation ON user_blocks
    FOR ALL
    USING (
        blocker_id = current_setting('app.current_user_id', true)::UUID
    );

-- ============================================================================
-- ADD ORGANIZATION_ID TO USER_BLOCKS FOR FULL ISOLATION
-- ============================================================================

-- Add organization_id column to user_blocks if not exists
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'user_blocks' AND column_name = 'organization_id'
    ) THEN
        ALTER TABLE user_blocks
            ADD COLUMN IF NOT EXISTS organization_id UUID REFERENCES organizations(id) ON DELETE CASCADE;

        -- Update existing blocks with org from blocker (via organization_members)
        UPDATE user_blocks ub
        SET organization_id = om.organization_id
        FROM organization_members om
        WHERE ub.blocker_id = om.user_id AND ub.organization_id IS NULL;

        -- Make it required going forward
        ALTER TABLE user_blocks
            ALTER COLUMN organization_id SET NOT NULL;

        -- Add index
        CREATE INDEX IF NOT EXISTS idx_user_blocks_organization ON user_blocks(organization_id);
    END IF;
END $$;

-- Update user_blocks policy to include org check
DROP POLICY IF EXISTS user_blocks_owner_isolation ON user_blocks;

CREATE POLICY user_blocks_owner_isolation ON user_blocks
    FOR ALL
    USING (
        blocker_id = current_setting('app.current_user_id', true)::UUID
        AND organization_id = COALESCE(
            current_setting('app.current_org_id', true)::UUID,
            organization_id
        )
    );

COMMENT ON COLUMN user_blocks.organization_id IS 'Organization context for multi-tenant isolation (added in security fix)';
