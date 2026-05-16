-- Phase 0: RLS Foundation Hardening
-- Adds Row-Level Security policies to tenant-scoped tables from migration 00100
-- (Epic 144: Portfolio Performance Analytics).
--
-- NOTE: 00100 already ran `ALTER TABLE ... ENABLE ROW LEVEL SECURITY` on all of
-- these tables, but only `performance_portfolios` got actual policies (and those
-- use the legacy current_setting pattern -- left untouched here, it is not a
-- gap table). The seven tables below had RLS *enabled* but NO policy, meaning
-- non-super-admin access was effectively denied for SELECT yet writes were
-- unconstrained relative to tenancy. This migration adds FORCE + a real policy.
--
--   market_benchmarks       -> own organization_id
--   portfolio_properties_perf -> performance_portfolios.organization_id (fk portfolio_id)
--   property_transactions   -> performance_portfolios.organization_id (fk portfolio_id)
--   property_cash_flows     -> performance_portfolios.organization_id (fk portfolio_id)
--   financial_metrics       -> performance_portfolios.organization_id (fk portfolio_id)
--   benchmark_comparisons   -> performance_portfolios.organization_id (fk portfolio_id)
--   performance_alerts      -> performance_portfolios.organization_id (fk portfolio_id)

-- ============================================
-- market_benchmarks (own-org; ENABLE already done in 00100)
-- ============================================
ALTER TABLE market_benchmarks FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS market_benchmarks_tenant_isolation ON market_benchmarks;
CREATE POLICY market_benchmarks_tenant_isolation ON market_benchmarks
    FOR ALL
    USING (is_super_admin() OR organization_id = get_current_org_id())
    WITH CHECK (is_super_admin() OR organization_id = get_current_org_id());

-- ============================================
-- portfolio_properties_perf (child of performance_portfolios, fk portfolio_id)
-- ============================================
ALTER TABLE portfolio_properties_perf FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS portfolio_properties_perf_tenant_isolation ON portfolio_properties_perf;
CREATE POLICY portfolio_properties_perf_tenant_isolation ON portfolio_properties_perf
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = portfolio_properties_perf.portfolio_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = portfolio_properties_perf.portfolio_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- property_transactions (child of performance_portfolios, fk portfolio_id)
-- ============================================
ALTER TABLE property_transactions FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS property_transactions_tenant_isolation ON property_transactions;
CREATE POLICY property_transactions_tenant_isolation ON property_transactions
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = property_transactions.portfolio_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = property_transactions.portfolio_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- property_cash_flows (child of performance_portfolios, fk portfolio_id)
-- ============================================
ALTER TABLE property_cash_flows FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS property_cash_flows_tenant_isolation ON property_cash_flows;
CREATE POLICY property_cash_flows_tenant_isolation ON property_cash_flows
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = property_cash_flows.portfolio_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = property_cash_flows.portfolio_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- financial_metrics (child of performance_portfolios, fk portfolio_id)
-- ============================================
ALTER TABLE financial_metrics FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS financial_metrics_tenant_isolation ON financial_metrics;
CREATE POLICY financial_metrics_tenant_isolation ON financial_metrics
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = financial_metrics.portfolio_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = financial_metrics.portfolio_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- benchmark_comparisons (child of performance_portfolios, fk portfolio_id)
-- ============================================
ALTER TABLE benchmark_comparisons FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS benchmark_comparisons_tenant_isolation ON benchmark_comparisons;
CREATE POLICY benchmark_comparisons_tenant_isolation ON benchmark_comparisons
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = benchmark_comparisons.portfolio_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = benchmark_comparisons.portfolio_id AND p.organization_id = get_current_org_id()));

-- ============================================
-- performance_alerts (child of performance_portfolios, fk portfolio_id)
-- ============================================
ALTER TABLE performance_alerts FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS performance_alerts_tenant_isolation ON performance_alerts;
CREATE POLICY performance_alerts_tenant_isolation ON performance_alerts
    FOR ALL
    USING (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = performance_alerts.portfolio_id AND p.organization_id = get_current_org_id()))
    WITH CHECK (is_super_admin() OR EXISTS (
        SELECT 1 FROM performance_portfolios p
        WHERE p.id = performance_alerts.portfolio_id AND p.organization_id = get_current_org_id()));
