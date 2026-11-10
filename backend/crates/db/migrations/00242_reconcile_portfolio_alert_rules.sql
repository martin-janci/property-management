-- Reconcile portfolio_alert_rules / portfolio_alerts with the
-- PortfolioAnalyticsRepository queries and the `PortfolioAlertRule` /
-- `PortfolioAlert` models (same strategy as 00236-00238). `PortfolioAlert`
-- decodes the legacy nullable columns (rule_id, metric_name, threshold_value,
-- message) as Option, since rule_id is nulled when its rule is deleted.
--
-- Migration 00091 created these tables with a different design
-- (condition_type, threshold_operator, notification_channels, recipients,
-- status, ...) than the one the repository and the generated API clients use.
-- Consequently:
--   * POST  /portfolio-analytics/alert-rules -> 500 `column "category" ... does not exist`
--   * GET   /portfolio-analytics/alert-rules -> 500 (same projection)
--   * GET   /portfolio-analytics/alerts      -> 500 `column "current_value" ... does not exist`
--
-- The model is the API contract (OpenAPI + generated clients), so the schema is
-- aligned to it: ADD the columns the repository reads and writes. Columns the
-- structs decode as non-Option get NOT NULL defaults. The legacy 00091 columns
-- are kept for backwards compatibility; `condition_type` was NOT NULL with no
-- default, so it gets a default the repository insert can rely on.

ALTER TABLE portfolio_alert_rules
    ADD COLUMN IF NOT EXISTS category               benchmark_category NOT NULL DEFAULT 'occupancy',
    ADD COLUMN IF NOT EXISTS operator               VARCHAR(20) NOT NULL DEFAULT '<',
    ADD COLUMN IF NOT EXISTS scope                  comparison_scope NOT NULL DEFAULT 'portfolio',
    ADD COLUMN IF NOT EXISTS building_id            UUID REFERENCES buildings(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS notify_roles           TEXT[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS notify_users           UUID[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS notification_frequency VARCHAR(50) DEFAULT 'immediate',
    ADD COLUMN IF NOT EXISTS trigger_count          INTEGER NOT NULL DEFAULT 0;

ALTER TABLE portfolio_alert_rules ALTER COLUMN condition_type SET DEFAULT 'threshold';

-- Legacy rows: carry the old operator over and give threshold_value a value so
-- the non-Option `threshold_value: Decimal` field decodes.
-- Legacy operators were stored as words (00091: 'gt', 'lt', 'gte', 'lte',
-- 'eq'); the model uses the symbols.
UPDATE portfolio_alert_rules
   SET operator = CASE lower(threshold_operator)
                      WHEN 'gt'  THEN '>'
                      WHEN 'lt'  THEN '<'
                      WHEN 'gte' THEN '>='
                      WHEN 'lte' THEN '<='
                      WHEN 'eq'  THEN '='
                      ELSE threshold_operator
                  END
 WHERE threshold_operator IS NOT NULL AND operator = '<';
UPDATE portfolio_alert_rules SET threshold_value = 0 WHERE threshold_value IS NULL;
ALTER TABLE portfolio_alert_rules ALTER COLUMN threshold_value SET DEFAULT 0;
ALTER TABLE portfolio_alert_rules ALTER COLUMN threshold_value SET NOT NULL;

CREATE INDEX IF NOT EXISTS idx_portfolio_alert_rules_org ON portfolio_alert_rules (organization_id);
CREATE INDEX IF NOT EXISTS idx_portfolio_alert_rules_building ON portfolio_alert_rules (building_id);

-- `alert_type` (00091) is NOT NULL but the model has no such field; default
-- it so alerts can be raised from the model's columns.
ALTER TABLE portfolio_alerts ALTER COLUMN alert_type SET DEFAULT 'threshold';

ALTER TABLE portfolio_alerts
    ADD COLUMN IF NOT EXISTS current_value      DECIMAL(15, 4) NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS deviation_pct      DECIMAL(10, 4),
    ADD COLUMN IF NOT EXISTS is_read            BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS is_resolved        BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS resolution_notes   TEXT,
    ADD COLUMN IF NOT EXISTS notifications_sent JSONB;

UPDATE portfolio_alerts SET is_resolved = TRUE WHERE status = 'resolved' AND NOT is_resolved;
UPDATE portfolio_alerts SET is_read = TRUE
 WHERE status IN ('acknowledged', 'resolved', 'dismissed') AND NOT is_read;

CREATE INDEX IF NOT EXISTS idx_portfolio_alerts_org_created
    ON portfolio_alerts (organization_id, created_at DESC);
