-- Phase 4: Publishing & Global Portal — 4th RLS context for the platform host.
--
-- This migration replaces the existing tenant-isolation policy on `listings`
-- (from 00110) with a 4-context policy:
--
--   1. Super-admin             — bypass via `is_super_admin()` (existing).
--   2. Agency-staff            — `organization_id = get_current_org_id()`
--                                (existing — also used by Path #3 below at
--                                the RLS layer).
--   3. Agency-subdomain        — same RLS predicate as #2; the differentiator
--      (public read of own org)  lives at the application layer (whether the
--                                request is authenticated agency-staff vs.
--                                unauthenticated public visitor on the
--                                agency.example.com subdomain). Both paths
--                                set `app.current_org_id` from the resolved
--                                tenant; what they can WRITE is gated by
--                                app-layer role (Phase 5 territory).
--   4. Global-read              — `is_published = TRUE` AND
--      (platform host, public)   `is_global_read_context()`. Reads the union
--                                of published rows across all orgs. Cannot
--                                write (WITH CHECK has no global-read clause).
--
-- Invariant I-D (a listing exists once) is enforced by construction: the
-- global view is one OR-clause in this policy, not a sync pipeline. There is
-- no second copy of a listing anywhere in the schema.

-- ============================================
-- is_global_read_context() — the 4th-context flag.
-- ============================================
--
-- Mirrors the shape of the existing `is_super_admin()` / `get_current_org_id()`
-- helpers (see 00004): reads a session-local setting set by the API extractor,
-- swallows the "setting not yet defined" error, defaults to FALSE.
--
-- The setting is `app.global_read`. The value is parsed as a BOOLEAN; the
-- exception handler ensures any malformed value (or a never-set setting on a
-- fresh connection) is treated as FALSE — fail-closed default. The application
-- code (`HostRlsConnection`) sets it explicitly to `'on'` for PlatformHost
-- requests and explicitly to `'off'` for every other request, so the
-- defensive-default branch only fires before the extractor has run on a
-- connection (e.g. during tests that intentionally omit the bootstrap).
CREATE OR REPLACE FUNCTION is_global_read_context()
RETURNS BOOLEAN
LANGUAGE plpgsql
STABLE
AS $$
BEGIN
    RETURN COALESCE(current_setting('app.global_read', TRUE)::BOOLEAN, FALSE);
EXCEPTION
    WHEN OTHERS THEN
        RETURN FALSE;
END;
$$;

COMMENT ON FUNCTION is_global_read_context() IS
    'Returns TRUE when the connection is in the 4th RLS context (PlatformHost): '
    'an unauthenticated public reader on the platform-wide reality portal. '
    'Used by the listings policy to grant cross-org SELECT of published rows.';

-- ============================================
-- listings: replace tenant-isolation with 4-context policy.
-- ============================================
DROP POLICY IF EXISTS listings_tenant_isolation ON listings;
DROP POLICY IF EXISTS listings_four_context ON listings;

CREATE POLICY listings_four_context ON listings
    FOR ALL
    USING (
        is_super_admin()
        OR organization_id = get_current_org_id()
        OR (is_published AND is_global_read_context())
    )
    WITH CHECK (
        -- WITH CHECK deliberately omits the global-read branch: the platform
        -- host is read-only for listings. Any INSERT/UPDATE/DELETE attempted
        -- from the PlatformHost context (where `app.current_org_id` is unset
        -- and `app.global_read = on`) will fail because:
        --   * `is_super_admin()` is FALSE,
        --   * `get_current_org_id()` returns NULL, so the equality is NULL,
        --   * the OR collapses to NULL/FALSE → the CHECK denies the row.
        is_super_admin()
        OR organization_id = get_current_org_id()
    );

COMMENT ON POLICY listings_four_context ON listings IS
    'Phase 4: 4-context RLS policy. (1) super-admin bypass; (2) agency-staff '
    'and (3) agency-subdomain both gated by organization_id = get_current_org_id(); '
    '(4) PlatformHost public read of is_published rows across all orgs. '
    'WITH CHECK omits the global branch — the global view is read-only.';

-- ============================================
-- Bootstrap helpers for the PlatformHost path.
-- ============================================
--
-- The Phase 1 `set_request_context(p_org_id, p_user_id, p_is_super_admin)`
-- helper (00006) bundles the three legacy session settings. The PlatformHost
-- path needs to additionally set `app.global_read`. Rather than extending the
-- legacy function (which would need every caller updated), we add a thin
-- companion that sets the flag on its own. The HostRlsConnection extractor
-- calls both: first `set_request_context(NULL, NULL, FALSE)` to clear the
-- tenant/user/admin trio, then `set_global_read_context(TRUE)` to opt into
-- the 4th context. `clear_request_context()` is extended below to also reset
-- `app.global_read`, so connection cleanup remains a one-call story.

CREATE OR REPLACE FUNCTION set_global_read_context(p_enabled BOOLEAN)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    -- FALSE = session-scoped, matching set_request_context's discipline.
    PERFORM set_config('app.global_read', COALESCE(p_enabled::TEXT, 'false'), FALSE);
END;
$$;

COMMENT ON FUNCTION set_global_read_context(BOOLEAN) IS
    'Phase 4: opt the current connection into (or out of) the 4th RLS context '
    '(PlatformHost / global-read). Call AFTER set_request_context so the legacy '
    'tenant/user/admin trio is in a known state.';

-- Replace clear_request_context() so connection cleanup also drops the
-- global-read flag. Without this extension, a PlatformHost request that
-- finishes and hands its connection back to the pool would leave
-- `app.global_read = on`, and the next request to acquire that connection
-- (which might be a tenant-scoped agency request) would silently get
-- cross-org SELECT visibility on listings. Defense for leak #2 (context
-- bleeding between requests).
CREATE OR REPLACE FUNCTION clear_request_context()
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM set_config('app.current_org_id', '', FALSE);
    PERFORM set_config('app.current_user_id', '', FALSE);
    PERFORM set_config('app.is_super_admin', 'false', FALSE);
    PERFORM set_config('app.global_read', 'false', FALSE);
END;
$$;

COMMENT ON FUNCTION clear_request_context() IS
    'Reset every RLS-relevant session setting to its safe default. Phase 4: '
    'extended to also clear app.global_read so PlatformHost context cannot '
    'bleed onto a pooled connection acquired by a tenant-scoped request.';
