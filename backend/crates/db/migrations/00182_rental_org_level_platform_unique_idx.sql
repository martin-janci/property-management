-- BIT-85: two schema changes to enable safe org-level (unit-less) platform
-- connections and eliminate the cross-tenant token-binding hazard.
--
-- 1. Drop the FK on unit_id so the nil-UUID sentinel
--    ('00000000-0000-0000-0000-000000000000') can be stored for connections
--    that are org-wide rather than tied to a specific unit.
--
-- 2. Add a partial unique index scoped to (organization_id, platform) for
--    nil-unit_id rows, replacing the shared (unit_id, platform) target.
--    Without this, two orgs writing nil-unit_id rows for the same platform
--    share a single ON CONFLICT slot — org B's DO UPDATE overwrites org A's
--    row (cross-tenant rebind hazard).

ALTER TABLE rental_platform_connections
    DROP CONSTRAINT IF EXISTS rental_platform_connections_unit_id_fkey;

CREATE UNIQUE INDEX IF NOT EXISTS idx_rental_platform_connections_org_platform_nil
    ON rental_platform_connections (organization_id, platform)
    WHERE unit_id = '00000000-0000-0000-0000-000000000000';
