-- Migration: 00178_force_document_shares_rls
-- Security Fix (PAP-21, follow-up to #754): FORCE row-level security on the
-- document *share* tables and repair their tenant-isolation policies so the
-- table owner can no longer bypass them.
--
-- Background
-- ----------
-- 00172 forced RLS on `documents` / `document_folders` but DELIBERATELY left
-- `document_shares` and `document_share_access_log` un-forced, with this note:
--
--   "Their public share-token endpoints read them via the raw pool with no org
--    context (the share token is the authorization), so forcing them would
--    break public sharing. They are tracked as a follow-up (route the public
--    path through an org-scoped connection first, then force)."
--
-- That follow-up is PAP-21. The public share handlers
-- (`/api/v1/documents/shared/{token}`) now read these tables through a
-- dedicated connection that sets super-admin RLS context for the single lookup
-- and clears it afterwards (mirroring `find_by_id_for_share`), so forcing RLS
-- here no longer breaks public sharing.
--
-- Two problems are fixed, exactly as 00172 did for `documents`:
--
-- 1. The 00020/00140 policies filter on `current_setting('app.tenant_id')`.
--    That GUC is NEVER set anywhere -- `set_request_context` sets
--    `app.current_org_id` / `app.is_super_admin` (00004/00006). So for any
--    non-owner role the policy evaluated `... = NULL` -> deny-all, and for the
--    owner role (the production api-server connects as the table owner `ppt`)
--    RLS was bypassed entirely -> a real cross-tenant IDOR on the share tables.
--    We rewrite the policies to the standard `is_super_admin()` /
--    `get_current_org_id()` helpers + the `get_current_org_not_deleted()`
--    soft-delete guard, matching `documents` (00172) and every other
--    org-scoped table (00140).
--
-- 2. `document_shares` and `document_share_access_log` get
--    `FORCE ROW LEVEL SECURITY` so the owner role is bound by the policy
--    instead of bypassing it, closing the IDOR.
--
-- Both tables key off the owning document's `organization_id`, so isolation is
-- expressed by joining through `documents`. The public path supplies
-- super-admin context (the validated share token is the authorization grant),
-- which satisfies the `is_super_admin()` OR-leg; authenticated share
-- management runs under the caller's org context via `RlsConnection`.

-- ============================================================================
-- Repair tenant-isolation policies (app.tenant_id -> get_current_org_id())
-- ============================================================================

DROP POLICY IF EXISTS share_tenant_isolation ON document_shares;
CREATE POLICY share_tenant_isolation ON document_shares
    FOR ALL
    USING (
        (is_super_admin() OR (document_id IN (
            SELECT id FROM documents WHERE organization_id = get_current_org_id()
        ))) AND get_current_org_not_deleted()
    )
    WITH CHECK (
        (is_super_admin() OR (document_id IN (
            SELECT id FROM documents WHERE organization_id = get_current_org_id()
        ))) AND get_current_org_not_deleted()
    );

DROP POLICY IF EXISTS access_log_tenant_isolation ON document_share_access_log;
CREATE POLICY access_log_tenant_isolation ON document_share_access_log
    FOR ALL
    USING (
        (is_super_admin() OR (share_id IN (
            SELECT ds.id FROM document_shares ds
            JOIN documents d ON d.id = ds.document_id
            WHERE d.organization_id = get_current_org_id()
        ))) AND get_current_org_not_deleted()
    )
    WITH CHECK (
        (is_super_admin() OR (share_id IN (
            SELECT ds.id FROM document_shares ds
            JOIN documents d ON d.id = ds.document_id
            WHERE d.organization_id = get_current_org_id()
        ))) AND get_current_org_not_deleted()
    );

-- ============================================================================
-- FORCE RLS so the table owner (production api-server role) is bound by the
-- policies above instead of bypassing them.
-- ============================================================================

ALTER TABLE document_shares FORCE ROW LEVEL SECURITY;
ALTER TABLE document_share_access_log FORCE ROW LEVEL SECURITY;
