/**
 * Portfolio Performance Analytics API types (Epic 144).
 *
 * These mirror the `db::models::portfolio_performance` structs served by the
 * api-server under `/api/v1/portfolio-performance`. The backend serializes
 * `rust_decimal::Decimal` with the `serde-str` feature, so every monetary /
 * ratio value arrives on the wire as a JSON **string** (e.g. `"2500000.00"`).
 * Those fields are typed as `string` here and converted at the edge by callers.
 * Integer counts (`i32`) and percentiles arrive as real JSON numbers.
 */

/** GET /portfolios/{id}/dashboard/summary */
export interface DashboardSummary {
  total_portfolio_value: string;
  total_equity: string;
  total_debt: string;
  debt_to_equity_ratio: string | null;
  ltv_ratio: string | null;
  ytd_noi: string;
  ytd_cash_flow: string;
  ytd_return_pct: string | null;
  property_count: number;
  total_units: number | null;
  occupied_units: number | null;
  occupancy_rate: string | null;
  currency: string;
  as_of_date: string;
}

/** GET /portfolios/{id}/dashboard/property-cards */
export interface PropertyPerformanceCard {
  property_id: string;
  property_name: string;
  building_address: string | null;
  current_value: string;
  equity: string;
  ltv: string | null;
  noi: string;
  cap_rate: string | null;
  cash_on_cash: string | null;
  dscr: string | null;
  occupancy_rate: string | null;
  monthly_cash_flow: string | null;
  vs_benchmark_pct: string | null;
  performance_status: string;
  currency: string;
}

/** GET /portfolios/{id}/dashboard/cash-flow-trend */
export interface CashFlowTrendPoint {
  period: string;
  period_date: string;
  gross_income: string;
  operating_expenses: string;
  noi: string;
  debt_service: string | null;
  net_cash_flow: string;
}

/** GET /portfolios/{id}/alerts */
export interface PerformanceAlert {
  id: string;
  portfolio_id: string;
  property_id: string | null;
  alert_type: string;
  severity: string;
  title: string;
  message: string;
  metric_name: string | null;
  current_value: string | null;
  threshold_value: string | null;
  is_read: boolean;
  is_resolved: boolean;
  created_at: string;
}

export interface ListPortfolioAlertsParams {
  unread_only?: boolean;
  limit?: number;
}

/** One property row inside {@link PortfolioMetricsSummary}. */
export interface MetricsSummary {
  property_id: string | null;
  property_name: string | null;
  noi: string;
  cap_rate: string | null;
  cash_on_cash: string | null;
  irr: string | null;
  dscr: string | null;
  equity_multiple: string | null;
  period: string;
  currency: string;
}

/** GET /portfolios/{id}/metrics/summary?period_start=&period_end= */
export interface PortfolioMetricsSummary {
  portfolio_id: string;
  portfolio_name: string;
  period_start: string;
  period_end: string;
  total_noi: string;
  weighted_avg_cap_rate: string | null;
  portfolio_cash_on_cash: string | null;
  portfolio_irr: string | null;
  portfolio_dscr: string | null;
  portfolio_equity_multiple: string | null;
  total_value: string;
  total_equity: string;
  total_debt: string;
  leverage_ratio: string | null;
  property_count: number;
  currency: string;
  property_metrics: MetricsSummary[];
}

export interface MetricsSummaryParams {
  period_start: string;
  period_end: string;
}
