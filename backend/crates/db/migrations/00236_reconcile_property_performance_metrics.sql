-- Reconcile property_performance_metrics with the PortfolioAnalyticsRepository
-- projections (issue #3078; follow-up to the #2946 IDOR fix in PR #3003).
--
-- `PortfolioAnalyticsRepository::{upsert,get,list}_property_metrics` and the
-- `PropertyPerformanceMetrics` model project a set of columns that migration
-- 00091 never created (gross_rental_income, other_income, total_revenue,
-- average_lease_term_months, expense_ratio, collection_rate,
-- tenant_satisfaction_score, complaints_count, estimated_value, currency).
-- Consequently:
--   * GET  /portfolio-analytics/properties/{building_id}/metrics  -> 500
--   * GET  /portfolio-analytics/properties/metrics                -> 500
--   * POST /portfolio-analytics/properties/metrics                -> 400
-- failed at runtime with `column "..." does not exist`, regardless of the
-- tenant-scoping (IDOR) fix. Earlier work tracked this as "BIT-567", which is
-- not a real issue in this repo; #3078 is the tracked bug.
--
-- Reconciliation strategy: ADD the missing columns (rather than trim the Rust
-- projections), preserving the existing generated-client API shape. Columns the
-- struct reads as non-Option get a NOT NULL default so historical rows and
-- partial seeds still decode; the rest are nullable to mirror `Option<_>`.

ALTER TABLE property_performance_metrics
    ADD COLUMN IF NOT EXISTS gross_rental_income       DECIMAL(15, 2) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS other_income              DECIMAL(15, 2),
    ADD COLUMN IF NOT EXISTS total_revenue             DECIMAL(15, 2),
    ADD COLUMN IF NOT EXISTS average_lease_term_months DECIMAL(10, 2),
    ADD COLUMN IF NOT EXISTS expense_ratio             DECIMAL(10, 2),
    ADD COLUMN IF NOT EXISTS collection_rate           DECIMAL(5, 2),
    ADD COLUMN IF NOT EXISTS tenant_satisfaction_score DECIMAL(5, 2),
    ADD COLUMN IF NOT EXISTS complaints_count          INTEGER,
    ADD COLUMN IF NOT EXISTS estimated_value           DECIMAL(15, 2),
    ADD COLUMN IF NOT EXISTS currency                  VARCHAR(3) NOT NULL DEFAULT 'EUR';

-- `upsert_property_metrics` keys on
-- `ON CONFLICT (building_id, period_start, period_end, period_type)`, but 00091
-- only declared `UNIQUE (building_id, period_type, period_start)` (no
-- period_end), so every write raised 42P10 ("no unique or exclusion constraint
-- matching the ON CONFLICT specification") at plan time — even before any row
-- actually conflicted. Replace the constraint with one whose column set matches
-- the upsert target so the period key includes period_end.
ALTER TABLE property_performance_metrics
    DROP CONSTRAINT IF EXISTS unique_property_period;
ALTER TABLE property_performance_metrics
    ADD CONSTRAINT unique_property_period
    UNIQUE (building_id, period_type, period_start, period_end);
