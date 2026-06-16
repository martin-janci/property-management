-- GH #1405: close the cross-tenant token-binding hazard in the Airbnb/Booking
-- OAuth upserts. When unit_id is unknown at token-exchange time the Rust code
-- falls back to Uuid::nil() as a placeholder. The existing UNIQUE(unit_id,
-- platform) constraint is shared across all organizations, so two orgs using
-- the nil placeholder would conflict and the DO UPDATE would write one org's
-- tokens under the other org's row.
--
-- This partial unique index scopes the nil-placeholder conflict to
-- (organization_id, platform), ensuring each organization's nil-unit_id
-- connection is a distinct row that can only be upserted by the same org.
-- The upsert in rental.rs uses this as the ON CONFLICT target for nil rows.
CREATE UNIQUE INDEX IF NOT EXISTS idx_rental_platform_org_nil_platform
    ON rental_platform_connections (organization_id, platform)
    WHERE unit_id = '00000000-0000-0000-0000-000000000000';
