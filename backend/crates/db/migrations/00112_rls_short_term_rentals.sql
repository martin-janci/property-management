-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00051
-- (Epic 18: Short-Term Rental Integration).
--
-- All six tables are OWN-ORG tables: each has its own organization_id column.

-- ============================================
-- rental_platform_connections (own-org)
-- ============================================
ALTER TABLE rental_platform_connections ENABLE ROW LEVEL SECURITY;
ALTER TABLE rental_platform_connections FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rental_platform_connections_tenant_isolation ON rental_platform_connections;
CREATE POLICY rental_platform_connections_tenant_isolation ON rental_platform_connections
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- rental_bookings (own-org)
-- ============================================
ALTER TABLE rental_bookings ENABLE ROW LEVEL SECURITY;
ALTER TABLE rental_bookings FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rental_bookings_tenant_isolation ON rental_bookings;
CREATE POLICY rental_bookings_tenant_isolation ON rental_bookings
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- rental_guests (own-org)
-- ============================================
ALTER TABLE rental_guests ENABLE ROW LEVEL SECURITY;
ALTER TABLE rental_guests FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rental_guests_tenant_isolation ON rental_guests;
CREATE POLICY rental_guests_tenant_isolation ON rental_guests
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- rental_guest_reports (own-org)
-- ============================================
ALTER TABLE rental_guest_reports ENABLE ROW LEVEL SECURITY;
ALTER TABLE rental_guest_reports FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rental_guest_reports_tenant_isolation ON rental_guest_reports;
CREATE POLICY rental_guest_reports_tenant_isolation ON rental_guest_reports
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- rental_calendar_blocks (own-org)
-- ============================================
ALTER TABLE rental_calendar_blocks ENABLE ROW LEVEL SECURITY;
ALTER TABLE rental_calendar_blocks FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rental_calendar_blocks_tenant_isolation ON rental_calendar_blocks;
CREATE POLICY rental_calendar_blocks_tenant_isolation ON rental_calendar_blocks
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- rental_ical_feeds (own-org)
-- ============================================
ALTER TABLE rental_ical_feeds ENABLE ROW LEVEL SECURITY;
ALTER TABLE rental_ical_feeds FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS rental_ical_feeds_tenant_isolation ON rental_ical_feeds;
CREATE POLICY rental_ical_feeds_tenant_isolation ON rental_ical_feeds
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());
