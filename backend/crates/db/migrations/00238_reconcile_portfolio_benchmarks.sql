-- Reconcile portfolio_benchmarks with PortfolioAnalyticsRepository benchmark
-- queries and the PortfolioBenchmark model (issue #3098; follow-up to #3083 /
-- PR #3087, which reconciled portfolio_aggregated_metrics in 00237).
--
-- Migration 00091 created portfolio_benchmarks with comparison_scope,
-- warning_threshold, critical_threshold, is_higher_better, created_by. The
-- repository instead SELECT/INSERTs min_acceptable, max_acceptable, scope,
-- property_type, region, is_industry_standard, source_name and source_year, so
-- GET/POST/PUT/DELETE /portfolio-analytics/benchmarks[/{id}] returned 500
-- (`column "..." does not exist`). The Rust enums also carry variants the
-- Postgres enums lack (BenchmarkCategory::{Revenue,Expense,Maintenance},
-- ComparisonScope::{Building,UnitType}), so inserting those would fail to encode.
--
-- Strategy mirrors 00236/00237: ADD the missing columns/values (preserving the
-- generated-client API shape). Option<_> fields are nullable; non-Option fields
-- get NOT NULL defaults.

ALTER TYPE benchmark_category ADD VALUE IF NOT EXISTS 'revenue';
ALTER TYPE benchmark_category ADD VALUE IF NOT EXISTS 'expense';
ALTER TYPE benchmark_category ADD VALUE IF NOT EXISTS 'maintenance';
ALTER TYPE comparison_scope   ADD VALUE IF NOT EXISTS 'building';
ALTER TYPE comparison_scope   ADD VALUE IF NOT EXISTS 'unit_type';

ALTER TABLE portfolio_benchmarks
    ADD COLUMN IF NOT EXISTS min_acceptable       DECIMAL(15, 4),
    ADD COLUMN IF NOT EXISTS max_acceptable       DECIMAL(15, 4),
    ADD COLUMN IF NOT EXISTS scope                comparison_scope NOT NULL DEFAULT 'portfolio',
    ADD COLUMN IF NOT EXISTS property_type        VARCHAR(100),
    ADD COLUMN IF NOT EXISTS region               VARCHAR(100),
    ADD COLUMN IF NOT EXISTS is_industry_standard BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS source_name          VARCHAR(200),
    ADD COLUMN IF NOT EXISTS source_year          INTEGER;

-- Keep the new `scope` in sync with the legacy comparison_scope column for
-- pre-existing rows.
UPDATE portfolio_benchmarks SET scope = comparison_scope WHERE scope IS DISTINCT FROM comparison_scope;

-- The model reads target_value as a non-Option Decimal.
UPDATE portfolio_benchmarks SET target_value = 0 WHERE target_value IS NULL;
ALTER TABLE portfolio_benchmarks
    ALTER COLUMN target_value SET DEFAULT 0,
    ALTER COLUMN target_value SET NOT NULL;
