-- ============================================================================
-- Phase 5.5 — integration fix: soft-delete on the 4-context listings policy.
-- ============================================================================
--
-- Background
-- ----------
-- 00136 (Phase 4) replaced the 2-context `listings_tenant_isolation` policy
-- with a 4-context `listings_four_context` policy that adds an OR-leg for
-- the unauthenticated PlatformHost reader (`is_published AND
-- is_global_read_context()`).
--
-- 00140 (Phase 5.5) was generated against a base that did NOT include 00136.
-- The generator introspected `pg_policies` for `listings`, found the legacy
-- `listings_tenant_isolation` policy (because it ran on a pre-Phase-4 schema),
-- and emitted a DROP/CREATE for that name only. Two regressions resulted on
-- the integration branch:
--
--   1. `listings_tenant_isolation` was reintroduced (a 2-context policy)
--      alongside the still-present `listings_four_context`. Postgres ORs
--      every PERMISSIVE policy on a table, so soft-deleted tenants'
--      published listings remained visible: `listings_four_context`
--      (without the soft-delete filter) supplies the OR-leg the global
--      portal reads through, and `listings_tenant_isolation` (a 2-context
--      copy that 00140 *did* AND with `get_current_org_not_deleted()`)
--      adds back the tenant-staff visibility for soft-deleted orgs because
--      Postgres ORs the two PERMISSIVE policies.
--
--   2. The 4-context `listings_four_context` policy was NEVER touched by
--      00140 (the generator didn't know its name), so its tenant-scoped
--      OR-leg still lacks the `get_current_org_not_deleted()` AND-leg.
--      Even after dropping the redundant `_tenant_isolation` copy, the
--      tenant-staff and agency-subdomain branches inside `_four_context`
--      would keep showing soft-deleted orgs' rows.
--
-- Net effect: defense for brainstorming leak #16 (soft-deleted tenant data
-- is hidden from tenant queries) is broken end-to-end for `listings` —
-- arguably the most user-visible table on the integration branch.
--
-- Fix
-- ---
-- Additive — we don't edit committed migration 00140. We:
--   * DROP the redundant 2-context `listings_tenant_isolation` policy
--     (00140 reintroduced it; 00136 had retired it on purpose).
--   * Rewrite `listings_four_context` to AND the soft-delete filter into
--     the tenant-scoped OR-leg. The super-admin branch stays untouched
--     (admins MUST see soft-deleted rows so they can restore them). The
--     PlatformHost branch matches the Phase 4 contract (read of any
--     `is_published` row); soft-deleted orgs' rows on the public portal are
--     handled at the application layer alongside the rest of the publish
--     lifecycle, not in this RLS policy.
--
-- After this migration `listings` carries exactly ONE policy
-- (`listings_four_context`) and the tenant-scoped OR-leg honors the
-- soft-delete flag.

DROP POLICY IF EXISTS listings_tenant_isolation ON listings;

DROP POLICY IF EXISTS listings_four_context ON listings;
CREATE POLICY listings_four_context ON listings
    FOR ALL
    USING (
        is_super_admin()
        OR (
            organization_id = get_current_org_id()
            AND get_current_org_not_deleted()
        )
        OR (is_published AND is_global_read_context())
    )
    WITH CHECK (
        is_super_admin()
        OR (
            organization_id = get_current_org_id()
            AND get_current_org_not_deleted()
        )
    );

COMMENT ON POLICY listings_four_context ON listings IS
    'Phase 4 + Phase 5.5: 4-context RLS policy with the soft-delete filter '
    'AND-ed into the tenant-scoped OR-leg. (1) super-admin bypass — sees '
    'soft-deleted rows so admins can restore them; (2) agency-staff and '
    '(3) agency-subdomain both gated by organization_id = get_current_org_id() '
    'AND get_current_org_not_deleted(); (4) PlatformHost public read of '
    'is_published rows. WITH CHECK omits the global branch — the global view '
    'is read-only.';
