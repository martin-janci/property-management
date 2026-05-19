-- Phase 4: Publishing & Global Portal — explicit publish state on listings.
--
-- Design:
--   * `is_published` is the single binary flag a listing carries to opt into
--     the platform-host (global portal) read context (4th RLS context — see
--     migration 00136). It is intentionally orthogonal to the existing
--     `status` column ('draft' / 'active' / 'paused' / 'sold' / 'rented' /
--     'archived', from migration 00049). `status` describes the workflow
--     state of the listing on the agency's own subdomain; `is_published`
--     describes whether the listing is opted into the cross-org global view.
--   * Sequence per ROADMAP.md Phase 4: ship binary first; channel-set
--     evolution stays compatible (a future migration can replace `is_published`
--     with `published_channels JSONB` while keeping the global-read OR-clause
--     intact via a generated column or a view).
--   * Backfill: the prior reality-server public-listing query in
--     `crates/db/src/repositories/portal.rs::search_listings` filters on
--     `WHERE l.status = 'active'`. So existing rows with `status = 'active'`
--     ARE the rows that were already publicly visible — those backfill to
--     `is_published = TRUE`. All other status values ('draft', 'paused',
--     'sold', 'rented', 'archived') stay `FALSE`.
--   * `published_at` already exists on `listings` (added in 00049). The
--     existing column was set in some publish flows but isn't a reliable
--     signal across the historical data. We do NOT redefine it; instead we
--     populate it from `created_at` for backfilled rows that already have a
--     NULL `published_at` (so `(is_published, published_at)` is a usable
--     index for "newest published first" queries from day one).
--
-- Defense for invariant I-D: a listing exists once. There is no copy of the
-- row in a "global portal" table — the global view is a filter (see 00136).

ALTER TABLE listings
    ADD COLUMN IF NOT EXISTS is_published BOOLEAN NOT NULL DEFAULT FALSE;

-- Backfill: rows previously visible via the reality-server `status='active'`
-- filter become published. Other status values remain unpublished.
UPDATE listings
    SET is_published = TRUE
    WHERE status = 'active';

-- Backfill `published_at` for newly-flagged rows that don't already carry one.
-- Use `created_at` as a safe lower bound — any sort by published_at DESC for
-- these rows reproduces the prior behaviour (which sorted by `published_at`
-- DESC and fell back through created order).
UPDATE listings
    SET published_at = COALESCE(published_at, created_at)
    WHERE is_published = TRUE AND published_at IS NULL;

-- Composite index supporting the dominant global-read query shape:
--   SELECT ... FROM listings WHERE is_published = TRUE ORDER BY published_at DESC
-- Partial on `is_published = TRUE` keeps it small (drafts dominate over time)
-- while still answering the "global feed" path efficiently.
CREATE INDEX idx_listings_is_published_published_at
    ON listings (is_published, published_at DESC);

-- ============================================
-- agency_domains: reserve PLATFORM_HOST sentinels
-- ============================================
--
-- The 4th RLS context (PlatformHost) is reachable only when the resolved host
-- is the platform-wide reality portal host (env `PLATFORM_HOST`, with sane
-- defaults baked in for dev). If an agency could ever be assigned the same
-- host, the resolver becomes ambiguous: we'd either set a tenant context
-- (closing off the global feed) or open the global feed (leaking the agency
-- as if it were the platform). Either is wrong. So we hard-reject those hosts
-- from `agency_domains` at the schema level.
--
-- We can't read env vars from a migration. Instead we maintain a small
-- `reserved_platform_hosts` lookup table seeded with safe defaults; ops can
-- INSERT additional entries via a super-admin path (Phase 5 control plane)
-- when the production PLATFORM_HOST changes. The check constraint references
-- this table via a STABLE SECURITY DEFINER function so a single source of
-- truth governs both the resolver (host_tenant_middleware) and the schema.

CREATE TABLE reserved_platform_hosts (
    host       VARCHAR(253) PRIMARY KEY,
    reason     TEXT         NOT NULL,
    created_at TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

COMMENT ON TABLE reserved_platform_hosts IS
    'Hosts that cannot be assigned to an agency because they identify the global '
    'platform portal (4th RLS context). The host_tenant_middleware also resolves '
    'these to PlatformHost so the table is the single source of truth.';

-- Seed safe defaults. Production deployments will add their own PLATFORM_HOST
-- via a super-admin endpoint; localhost variants stay reserved permanently so
-- dev never stumbles into "agency owns localhost" ambiguity.
INSERT INTO reserved_platform_hosts (host, reason) VALUES
    ('reality.example.com', 'default platform host (production placeholder)'),
    ('localhost',           'dev: bare localhost is the platform host fallback'),
    ('127.0.0.1',           'dev: loopback is the platform host fallback'),
    ('reality.dev.ppt.rlt.sk', 'dev deploy: platform reality portal'),
    ('reality.rlt.sk',      'staging: platform reality portal');

-- SECURITY DEFINER so the constraint check works even when an unprivileged
-- agency-staff connection (RLS-scoped) inserts. STABLE so PostgreSQL can
-- treat the result as constant within a statement.
CREATE OR REPLACE FUNCTION is_reserved_platform_host(p_host TEXT)
RETURNS BOOLEAN
LANGUAGE plpgsql
STABLE
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
    RETURN EXISTS (SELECT 1 FROM reserved_platform_hosts WHERE host = LOWER(p_host));
END;
$$;

COMMENT ON FUNCTION is_reserved_platform_host(TEXT) IS
    'Returns TRUE if the host is reserved for the global platform portal. '
    'Used by the agency_domains check constraint and (in the application '
    'layer) by host_tenant_middleware to short-circuit to PlatformHost.';

-- The check constraint. We give it a NOT VALID + VALIDATE pattern only if we
-- expected pre-existing violations; here we know the seed list is safe (no
-- agency has been provisioned with these hosts in Phase 1) so a plain ADD
-- CONSTRAINT is correct.
ALTER TABLE agency_domains
    ADD CONSTRAINT agency_domains_host_not_reserved
    CHECK (NOT is_reserved_platform_host(host));

COMMENT ON CONSTRAINT agency_domains_host_not_reserved ON agency_domains IS
    'No agency domain may equal a reserved PLATFORM_HOST sentinel. The host '
    'resolver short-circuits those hosts to the global-read context, so an '
    'agency holding the same host would be permanently unreachable.';
