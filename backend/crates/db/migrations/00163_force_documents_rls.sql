-- Migration: 00163_force_documents_rls
-- Security Fix (issue #754): FORCE row-level security on the document tables and
-- repair their tenant-isolation policies so the table owner can't bypass them.
--
-- Background
-- ----------
-- 00020 created `documents` and `document_folders` with `ENABLE ROW LEVEL
-- SECURITY` only (never `FORCE`). `ENABLE` (without `FORCE`) is bypassed for
-- the table owner and for superuser/`BYPASSRLS` roles. The production
-- api-server connects as the table OWNER (`ppt`), so the documents RLS policy
-- was silently ineffective: `get_document` / `find_by_id_with_details_rls`
-- did NOT isolate by org at runtime -> a real cross-tenant IDOR (#754).
--
-- Two problems are fixed here:
--
-- 1. The policies introduced by 00020 (and re-stated by 00140) filter on
--    `current_setting('app.tenant_id', true)`. That GUC is NEVER set anywhere
--    in the codebase -- `set_request_context` / `set_tenant_context` set
--    `app.current_org_id` (see 00004/00006). So for any non-owner role the
--    policy already evaluated `organization_id = NULL` -> deny-all, and for
--    the owner role it was bypassed entirely. We rewrite the policies to use
--    the standard `get_current_org_id()` / `is_super_admin()` helpers and the
--    `get_current_org_not_deleted()` soft-delete guard, matching every other
--    org-scoped table (00140).
--
-- 2. `documents` and `document_folders` get `FORCE ROW LEVEL SECURITY` so the
--    owner role is subject to the policy, closing the IDOR. This mirrors the
--    messaging fix (#898 / 00171) and device_push_tokens (00159), which spells
--    out the convention: "Enforce RLS even for the table owner (matches all
--    other RLS tables)."
--
-- Scope note: `document_shares` and `document_share_access_log` are
-- deliberately NOT forced here. Their public share-token endpoints
-- (`/api/v1/documents/shared/{token}`) read them via the raw pool with no org
-- context (the share token is the authorization), so forcing them would break
-- public sharing. They are tracked as a follow-up (route the public path
-- through an org-scoped connection first, then force). The raw-pool reads of
-- `documents` itself on the public + version-restore paths are migrated to
-- org-scoped connections in this PR so FORCE on `documents` is safe.

-- ============================================================================
-- Repair tenant-isolation policies (app.tenant_id -> get_current_org_id())
-- ============================================================================

DROP POLICY IF EXISTS folder_tenant_isolation ON document_folders;
CREATE POLICY folder_tenant_isolation ON document_folders
    FOR ALL
    USING ((is_super_admin() OR (organization_id = get_current_org_id())) AND get_current_org_not_deleted())
    WITH CHECK ((is_super_admin() OR (organization_id = get_current_org_id())) AND get_current_org_not_deleted());

DROP POLICY IF EXISTS document_tenant_isolation ON documents;
CREATE POLICY document_tenant_isolation ON documents
    FOR ALL
    USING ((is_super_admin() OR (organization_id = get_current_org_id())) AND get_current_org_not_deleted())
    WITH CHECK ((is_super_admin() OR (organization_id = get_current_org_id())) AND get_current_org_not_deleted());

-- ============================================================================
-- FORCE RLS so the table owner (production api-server role) is bound by the
-- policy above instead of bypassing it.
-- ============================================================================

ALTER TABLE document_folders FORCE ROW LEVEL SECURITY;
ALTER TABLE documents FORCE ROW LEVEL SECURITY;
