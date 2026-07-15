-- Migration: 00216_document_embeddings_super_admin_policy.sql
-- Follow-up to #2321 (post-merge review of PR #2312 / #2300).
--
-- Problem
-- -------
-- `document_embeddings` runs under `FORCE ROW LEVEL SECURITY` with a single
-- org-isolation policy (`document_embeddings_org_isolation`, last redefined in
-- 00179) whose `USING`/`WITH CHECK` clauses are purely `organization_id =
-- get_current_org_id()`. Unlike sibling tenant tables (`unit_residents`,
-- `delegations`, `invoices`, …) it has NO super-admin branch. Because FORCE RLS
-- also applies to the table owner, a non-superuser app role never sees rows
-- outside its current org on this table — even with `app.is_super_admin=true`.
--
-- The `POST /api/v1/ai/llm/rag/migrate` back-fill
-- (`migrate_embeddings_to_pgvector`) is documented and gated as a global
-- maintenance op that "converts legacy rows across every organization in one
-- pass" via the super-admin RLS bypass. Without a super-admin policy on this
-- table that bypass silently does not apply: the back-fill UPDATE (and its
-- before/after `COUNT(*)`) only ever touch the caller's own org, every other
-- organization's legacy rows stay unmigrated, and the reported `migrated`
-- count under-reports with no error — the classic FORCE-RLS silent-0-rows
-- pitfall.
--
-- Fix
-- ---
-- Add a permissive super-admin policy matching the established `*_super_admin`
-- pattern (00009/00010/00040/00162). PostgreSQL OR-combines permissive
-- policies for the same command, so a super-admin connection
-- (`is_super_admin()` reads the `app.is_super_admin` GUC set by
-- `set_request_context`) passes this policy for every row while ordinary org
-- users remain constrained to `document_embeddings_org_isolation`.

DROP POLICY IF EXISTS document_embeddings_super_admin ON document_embeddings;
CREATE POLICY document_embeddings_super_admin ON document_embeddings
    FOR ALL
    USING (is_super_admin())
    WITH CHECK (is_super_admin());

COMMENT ON POLICY document_embeddings_super_admin ON document_embeddings IS
    '#2321: permissive super-admin bypass so the cross-org /rag/migrate back-fill can see and convert legacy rows in every organization under FORCE RLS.';
