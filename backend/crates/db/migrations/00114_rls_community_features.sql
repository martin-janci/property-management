-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00064
-- (Epic 37: Community & Social Features, Epic 38: Workflow Automation).
--
-- Org-reach chains traced from 00064_community_features.sql:
--   community_groups            -> buildings.organization_id (fk building_id)
--   community_group_members     -> community_groups -> buildings
--   community_join_requests     -> community_groups -> buildings
--   community_posts             -> community_groups -> buildings
--   community_post_reactions    -> community_posts -> community_groups -> buildings
--   community_comments          -> community_posts -> community_groups -> buildings
--   community_comment_reactions -> community_comments -> community_posts -> community_groups -> buildings
--   community_poll_votes        -> community_posts -> community_groups -> buildings
--   community_events            -> buildings.organization_id (fk building_id; group_id is nullable)
--   community_event_rsvps       -> community_events -> buildings
--   community_event_comments    -> community_events -> buildings
--   marketplace_items           -> buildings.organization_id (fk building_id)
--   marketplace_inquiries       -> marketplace_items -> buildings
--   marketplace_favorites       -> marketplace_items -> buildings
--   workflow_automation_rules   -> own organization_id
--   workflow_automation_logs    -> workflow_automation_rules.organization_id (fk rule_id)
--
-- Since intermediate parents (community_groups, community_posts,
-- community_comments, community_events, marketplace_items) have no
-- organization_id of their own, the EXISTS clauses chain JOINs all the way
-- down to buildings.organization_id.

-- ============================================
-- workflow_automation_rules (own-org)
-- ============================================
ALTER TABLE workflow_automation_rules ENABLE ROW LEVEL SECURITY;
ALTER TABLE workflow_automation_rules FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS workflow_automation_rules_tenant_isolation ON workflow_automation_rules;
CREATE POLICY workflow_automation_rules_tenant_isolation ON workflow_automation_rules
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- workflow_automation_logs (child of workflow_automation_rules, fk rule_id)
-- ============================================
ALTER TABLE workflow_automation_logs ENABLE ROW LEVEL SECURITY;
ALTER TABLE workflow_automation_logs FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS workflow_automation_logs_tenant_isolation ON workflow_automation_logs;
CREATE POLICY workflow_automation_logs_tenant_isolation ON workflow_automation_logs
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM workflow_automation_rules p
        WHERE p.id = workflow_automation_logs.rule_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM workflow_automation_rules p
        WHERE p.id = workflow_automation_logs.rule_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- community_groups (child of buildings, fk building_id)
-- ============================================
ALTER TABLE community_groups ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_groups FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_groups_tenant_isolation ON community_groups;
CREATE POLICY community_groups_tenant_isolation ON community_groups
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM buildings b
        WHERE b.id = community_groups.building_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM buildings b
        WHERE b.id = community_groups.building_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_group_members (child of community_groups, fk group_id)
-- ============================================
ALTER TABLE community_group_members ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_group_members FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_group_members_tenant_isolation ON community_group_members;
CREATE POLICY community_group_members_tenant_isolation ON community_group_members
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_groups g
        JOIN buildings b ON b.id = g.building_id
        WHERE g.id = community_group_members.group_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_groups g
        JOIN buildings b ON b.id = g.building_id
        WHERE g.id = community_group_members.group_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_join_requests (child of community_groups, fk group_id)
-- ============================================
ALTER TABLE community_join_requests ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_join_requests FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_join_requests_tenant_isolation ON community_join_requests;
CREATE POLICY community_join_requests_tenant_isolation ON community_join_requests
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_groups g
        JOIN buildings b ON b.id = g.building_id
        WHERE g.id = community_join_requests.group_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_groups g
        JOIN buildings b ON b.id = g.building_id
        WHERE g.id = community_join_requests.group_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_posts (child of community_groups, fk group_id)
-- ============================================
ALTER TABLE community_posts ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_posts FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_posts_tenant_isolation ON community_posts;
CREATE POLICY community_posts_tenant_isolation ON community_posts
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_groups g
        JOIN buildings b ON b.id = g.building_id
        WHERE g.id = community_posts.group_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_groups g
        JOIN buildings b ON b.id = g.building_id
        WHERE g.id = community_posts.group_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_post_reactions (child of community_posts, fk post_id)
-- ============================================
ALTER TABLE community_post_reactions ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_post_reactions FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_post_reactions_tenant_isolation ON community_post_reactions;
CREATE POLICY community_post_reactions_tenant_isolation ON community_post_reactions
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_posts po
        JOIN community_groups g ON g.id = po.group_id
        JOIN buildings b ON b.id = g.building_id
        WHERE po.id = community_post_reactions.post_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_posts po
        JOIN community_groups g ON g.id = po.group_id
        JOIN buildings b ON b.id = g.building_id
        WHERE po.id = community_post_reactions.post_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_comments (child of community_posts, fk post_id)
-- ============================================
ALTER TABLE community_comments ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_comments FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_comments_tenant_isolation ON community_comments;
CREATE POLICY community_comments_tenant_isolation ON community_comments
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_posts po
        JOIN community_groups g ON g.id = po.group_id
        JOIN buildings b ON b.id = g.building_id
        WHERE po.id = community_comments.post_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_posts po
        JOIN community_groups g ON g.id = po.group_id
        JOIN buildings b ON b.id = g.building_id
        WHERE po.id = community_comments.post_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_comment_reactions (child of community_comments, fk comment_id)
-- ============================================
ALTER TABLE community_comment_reactions ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_comment_reactions FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_comment_reactions_tenant_isolation ON community_comment_reactions;
CREATE POLICY community_comment_reactions_tenant_isolation ON community_comment_reactions
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_comments c
        JOIN community_posts po ON po.id = c.post_id
        JOIN community_groups g ON g.id = po.group_id
        JOIN buildings b ON b.id = g.building_id
        WHERE c.id = community_comment_reactions.comment_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_comments c
        JOIN community_posts po ON po.id = c.post_id
        JOIN community_groups g ON g.id = po.group_id
        JOIN buildings b ON b.id = g.building_id
        WHERE c.id = community_comment_reactions.comment_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_poll_votes (child of community_posts, fk post_id)
-- ============================================
ALTER TABLE community_poll_votes ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_poll_votes FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_poll_votes_tenant_isolation ON community_poll_votes;
CREATE POLICY community_poll_votes_tenant_isolation ON community_poll_votes
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_posts po
        JOIN community_groups g ON g.id = po.group_id
        JOIN buildings b ON b.id = g.building_id
        WHERE po.id = community_poll_votes.post_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_posts po
        JOIN community_groups g ON g.id = po.group_id
        JOIN buildings b ON b.id = g.building_id
        WHERE po.id = community_poll_votes.post_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_events (child of buildings, fk building_id; group_id is nullable)
-- ============================================
ALTER TABLE community_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_events FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_events_tenant_isolation ON community_events;
CREATE POLICY community_events_tenant_isolation ON community_events
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM buildings b
        WHERE b.id = community_events.building_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM buildings b
        WHERE b.id = community_events.building_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_event_rsvps (child of community_events, fk event_id)
-- ============================================
ALTER TABLE community_event_rsvps ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_event_rsvps FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_event_rsvps_tenant_isolation ON community_event_rsvps;
CREATE POLICY community_event_rsvps_tenant_isolation ON community_event_rsvps
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_events e
        JOIN buildings b ON b.id = e.building_id
        WHERE e.id = community_event_rsvps.event_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_events e
        JOIN buildings b ON b.id = e.building_id
        WHERE e.id = community_event_rsvps.event_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- community_event_comments (child of community_events, fk event_id)
-- ============================================
ALTER TABLE community_event_comments ENABLE ROW LEVEL SECURITY;
ALTER TABLE community_event_comments FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS community_event_comments_tenant_isolation ON community_event_comments;
CREATE POLICY community_event_comments_tenant_isolation ON community_event_comments
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_events e
        JOIN buildings b ON b.id = e.building_id
        WHERE e.id = community_event_comments.event_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM community_events e
        JOIN buildings b ON b.id = e.building_id
        WHERE e.id = community_event_comments.event_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- marketplace_items (child of buildings, fk building_id)
-- ============================================
ALTER TABLE marketplace_items ENABLE ROW LEVEL SECURITY;
ALTER TABLE marketplace_items FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS marketplace_items_tenant_isolation ON marketplace_items;
CREATE POLICY marketplace_items_tenant_isolation ON marketplace_items
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM buildings b
        WHERE b.id = marketplace_items.building_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM buildings b
        WHERE b.id = marketplace_items.building_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- marketplace_inquiries (child of marketplace_items, fk item_id)
-- ============================================
ALTER TABLE marketplace_inquiries ENABLE ROW LEVEL SECURITY;
ALTER TABLE marketplace_inquiries FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS marketplace_inquiries_tenant_isolation ON marketplace_inquiries;
CREATE POLICY marketplace_inquiries_tenant_isolation ON marketplace_inquiries
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM marketplace_items i
        JOIN buildings b ON b.id = i.building_id
        WHERE i.id = marketplace_inquiries.item_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM marketplace_items i
        JOIN buildings b ON b.id = i.building_id
        WHERE i.id = marketplace_inquiries.item_id AND b.organization_id = get_current_org_id()));

-- ============================================
-- marketplace_favorites (child of marketplace_items, fk item_id)
-- ============================================
ALTER TABLE marketplace_favorites ENABLE ROW LEVEL SECURITY;
ALTER TABLE marketplace_favorites FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS marketplace_favorites_tenant_isolation ON marketplace_favorites;
CREATE POLICY marketplace_favorites_tenant_isolation ON marketplace_favorites
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM marketplace_items i
        JOIN buildings b ON b.id = i.building_id
        WHERE i.id = marketplace_favorites.item_id AND b.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM marketplace_items i
        JOIN buildings b ON b.id = i.building_id
        WHERE i.id = marketplace_favorites.item_id AND b.organization_id = get_current_org_id()));
