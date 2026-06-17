-- BIT-85: enable safe org-level (unit-less) platform connections and eliminate
-- the cross-tenant token-binding hazard on rental_platform_connections.
--
-- Org-level OAuth/Supply-XML connections are stored with the nil-UUID sentinel
-- ('00000000-0000-0000-0000-000000000000') in unit_id because they belong to
-- the organisation as a whole rather than to one unit. Four schema changes make
-- that model safe:
--
--   1. Drop the FK on unit_id so the nil-UUID sentinel (which has no matching
--      units row) can be stored for org-wide connections.
--
--   2. Drop the global UNIQUE(unit_id, platform) constraint. With nil-unit_id
--      org-level rows, (nil, platform) is a single shared key across ALL orgs,
--      so only one organisation could ever hold a given platform connection.
--      A second org's INSERT either raised an unhandled unique violation or —
--      via ON CONFLICT (unit_id, platform) — rebound the first org's row to the
--      second org's tokens. This was the cross-tenant hazard.
--
--   3. Add a partial unique index on (organization_id, platform) for nil-unit_id
--      rows: one org-level connection per platform per org. This is the arbiter
--      for the org-scoped ON CONFLICT used by the OAuth/Supply-XML upserts.
--
--   4. Add a partial unique index on (unit_id, platform) for real (non-nil)
--      unit_id rows: preserves the original one-connection-per-platform-per-unit
--      invariant for unit-level connections (the arbiter for the unit-level
--      ON CONFLICT (unit_id, platform) WHERE unit_id <> nil path).

ALTER TABLE rental_platform_connections
    DROP CONSTRAINT IF EXISTS rental_platform_connections_unit_id_fkey;

ALTER TABLE rental_platform_connections
    DROP CONSTRAINT IF EXISTS rental_platform_connections_unit_id_platform_key;

CREATE UNIQUE INDEX IF NOT EXISTS idx_rental_platform_connections_org_platform_nil
    ON rental_platform_connections (organization_id, platform)
    WHERE unit_id = '00000000-0000-0000-0000-000000000000';

CREATE UNIQUE INDEX IF NOT EXISTS idx_rental_platform_connections_unit_platform
    ON rental_platform_connections (unit_id, platform)
    WHERE unit_id <> '00000000-0000-0000-0000-000000000000';
