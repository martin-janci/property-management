-- Reconcile portfolio_aggregated_metrics with the PortfolioAnalyticsRepository
-- projections (issue #3083; follow-up to #3078 / PR #3081, which reconciled the
-- sibling property_performance_metrics table in migration 00236).
--
-- `PortfolioAnalyticsRepository::{get,calculate}_portfolio_metrics` and
-- `get_portfolio_summary`, plus the `PortfolioAggregatedMetrics` model, project a
-- set of columns that migration 00091 never created on portfolio_aggregated_metrics
-- (it used different names: total_properties, avg_occupancy_rate,
-- total_gross_revenue, total_operating_expenses, total_property_value,
-- portfolio_revenue_per_unit, portfolio_expense_per_unit, revenue_trend, noi_trend).
-- The repository instead reads total_buildings, occupied_units,
-- portfolio_occupancy_rate, total_revenue, total_expenses, currency,
-- avg_rent_per_unit, avg_rent_per_sqm, avg_expense_per_unit, revenue_growth_pct,
-- expense_growth_pct, noi_growth_pct, estimated_portfolio_value, buildings_by_type
-- and revenue_by_region. Consequently:
--   * GET  /portfolio-analytics/summary                   -> 500
--   * GET  /portfolio-analytics/metrics                   -> 500
--   * POST /portfolio-analytics/metrics/calculate         -> 400
-- failed at runtime with `column "..." does not exist`. Earlier work tracked this
-- as "BIT-567", which is not a real issue in this repo; #3083 is the tracked bug.
-- (Adding total_revenue to property_performance_metrics in 00236 incidentally
-- unblocked only the `ppm.total_revenue` reference in calculate_portfolio_metrics;
-- the aggregated table's own columns remained unreconciled.)
--
-- Reconciliation strategy mirrors 00236: ADD the missing columns (rather than trim
-- the Rust projections) so the generated-client API shape is preserved. Columns the
-- struct reads as non-Option get a NOT NULL default so historical rows and partial
-- seeds still decode; the rest are nullable to mirror `Option<_>`.

ALTER TABLE portfolio_aggregated_metrics
    ADD COLUMN IF NOT EXISTS total_buildings           INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS occupied_units            INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS portfolio_occupancy_rate  DECIMAL(5, 2),
    ADD COLUMN IF NOT EXISTS total_revenue             DECIMAL(15, 2) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_expenses            DECIMAL(15, 2),
    ADD COLUMN IF NOT EXISTS currency                  VARCHAR(3) NOT NULL DEFAULT 'EUR',
    ADD COLUMN IF NOT EXISTS avg_rent_per_unit         DECIMAL(10, 2),
    ADD COLUMN IF NOT EXISTS avg_rent_per_sqm          DECIMAL(10, 2),
    ADD COLUMN IF NOT EXISTS avg_expense_per_unit      DECIMAL(10, 2),
    ADD COLUMN IF NOT EXISTS revenue_growth_pct        DECIMAL(5, 2),
    ADD COLUMN IF NOT EXISTS expense_growth_pct        DECIMAL(5, 2),
    ADD COLUMN IF NOT EXISTS noi_growth_pct            DECIMAL(5, 2),
    ADD COLUMN IF NOT EXISTS estimated_portfolio_value DECIMAL(15, 2),
    ADD COLUMN IF NOT EXISTS buildings_by_type         JSONB,
    ADD COLUMN IF NOT EXISTS revenue_by_region         JSONB;

-- `calculate_portfolio_metrics` keys on
-- `ON CONFLICT (organization_id, period_start, period_end, period_type)`, but 00091
-- only declared `unique_org_period UNIQUE (organization_id, period_type, period_start)`
-- (no period_end), so every upsert raised 42P10 ("no unique or exclusion constraint
-- matching the ON CONFLICT specification") at plan time — even before any row
-- actually conflicted (identical to the property_performance_metrics bug 00236 fixed).
-- Replace the constraint with one whose column set matches the upsert target so the
-- period key includes period_end.
ALTER TABLE portfolio_aggregated_metrics
    DROP CONSTRAINT IF EXISTS unique_org_period;
ALTER TABLE portfolio_aggregated_metrics
    ADD CONSTRAINT unique_org_period
    UNIQUE (organization_id, period_type, period_start, period_end);
