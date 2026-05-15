-- ============================================================================
-- Phase 5.5: Tenant Lifecycle & Operability
-- Migration 00139 — soft-delete column on organizations.
-- ============================================================================
-- Defense for brainstorming leak #16 ("Restore-one rolls back everyone"):
-- delete is the default, hard-delete is a tooling path. Tenant rows do NOT need
-- to be marked one-by-one — RLS does the heavy lifting via the
-- get_current_org_not_deleted() helper introduced in 00140. Once an
-- organization is soft-deleted, no tenant query can see its data, but a
-- super-admin can still inspect (and restore) it.
-- ============================================================================

ALTER TABLE organizations
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ NULL;

COMMENT ON COLUMN organizations.deleted_at IS
    'Soft-delete tombstone (Phase 5.5). NULL = active. NOT NULL = soft-deleted; '
    'tenant queries hide the org and all its data via RLS, super-admin still sees it.';

-- Partial index — most rows are alive, so this stays small and is what every
-- tenant-context query effectively probes (via the RLS helper).
CREATE INDEX IF NOT EXISTS idx_organizations_alive
    ON organizations (id)
    WHERE deleted_at IS NULL;

-- A separate index for restore tooling that wants to enumerate the dead.
CREATE INDEX IF NOT EXISTS idx_organizations_deleted_at
    ON organizations (deleted_at)
    WHERE deleted_at IS NOT NULL;
