// Epic 144: Portfolio Performance Analytics - Portfolio Dashboard Page
import {
  type CashFlowTrendPoint,
  type DashboardSummary,
  type MetricsSummary,
  type PerformanceAlert as PerformanceAlertDto,
  type PropertyPerformanceCard,
  useCashFlowTrend,
  useDashboardSummary,
  useMarkAlertRead,
  usePortfolioAlerts,
  usePortfolioMetricsSummary,
  usePropertyPerformanceCards,
  useResolveAlert,
} from '@ppt/api-client';
import type React from 'react';
import {
  CashFlowChart,
  FinancialMetricsTable,
  MetricsSummaryCard,
  PerformanceAlert,
  PropertyCard,
} from '../components';

interface PortfolioDashboardPageProps {
  portfolioId: string;
}

// --- Decimal helpers ------------------------------------------------------
// The api-server serializes `rust_decimal::Decimal` with the `serde-str`
// feature, so monetary / ratio fields arrive as strings. Convert at the edge.
const num = (v: string | null | undefined): number => (v == null ? 0 : Number(v));
const optNum = (v: string | null | undefined): number | undefined =>
  v == null ? undefined : Number(v);

// --- Response → component-prop mappers (snake_case → camelCase) -----------
function mapPropertyCard(card: PropertyPerformanceCard) {
  return {
    id: card.property_id,
    propertyName: card.property_name,
    buildingAddress: card.building_address ?? undefined,
    currentValue: num(card.current_value),
    equity: num(card.equity),
    ltv: optNum(card.ltv),
    noi: num(card.noi),
    capRate: optNum(card.cap_rate),
    cashOnCash: optNum(card.cash_on_cash),
    dscr: optNum(card.dscr),
    occupancyRate: optNum(card.occupancy_rate),
    monthlyCashFlow: optNum(card.monthly_cash_flow),
    performanceStatus: card.performance_status,
    currency: card.currency,
  };
}

function mapCashFlowPoint(point: CashFlowTrendPoint) {
  return {
    period: point.period,
    periodDate: point.period_date,
    grossIncome: num(point.gross_income),
    operatingExpenses: num(point.operating_expenses),
    noi: num(point.noi),
    debtService: optNum(point.debt_service),
    netCashFlow: num(point.net_cash_flow),
  };
}

function mapMetric(metric: MetricsSummary) {
  return {
    propertyId: metric.property_id ?? undefined,
    propertyName: metric.property_name ?? undefined,
    noi: num(metric.noi),
    capRate: optNum(metric.cap_rate),
    cashOnCash: optNum(metric.cash_on_cash),
    irr: optNum(metric.irr),
    dscr: optNum(metric.dscr),
    equityMultiple: optNum(metric.equity_multiple),
    period: metric.period,
    currency: metric.currency,
  };
}

// Year-to-date window used for the metrics summary (per-property table).
function ytdRange(): { period_start: string; period_end: string } {
  const now = new Date();
  const iso = (d: Date) => d.toISOString().slice(0, 10);
  return {
    period_start: `${now.getUTCFullYear()}-01-01`,
    period_end: iso(now),
  };
}

const SummaryCards: React.FC<{ summary: DashboardSummary }> = ({ summary }) => (
  <div className="grid grid-cols-5 gap-4">
    <MetricsSummaryCard
      title="Portfolio Value"
      value={num(summary.total_portfolio_value)}
      format="currency"
      currency={summary.currency}
    />
    <MetricsSummaryCard
      title="Total Equity"
      value={num(summary.total_equity)}
      format="currency"
      currency={summary.currency}
    />
    <MetricsSummaryCard
      title="YTD NOI"
      value={num(summary.ytd_noi)}
      format="currency"
      currency={summary.currency}
    />
    <MetricsSummaryCard
      title="YTD Cash Flow"
      value={num(summary.ytd_cash_flow)}
      format="currency"
      currency={summary.currency}
      change={optNum(summary.ytd_return_pct)}
      changeLabel="YTD return"
      trend={num(summary.ytd_cash_flow) >= 0 ? 'up' : 'down'}
    />
    <MetricsSummaryCard
      title="LTV Ratio"
      value={num(summary.ltv_ratio)}
      format="percent"
      subtitle={
        summary.debt_to_equity_ratio != null
          ? `D/E: ${num(summary.debt_to_equity_ratio).toFixed(2)}x`
          : undefined
      }
    />
  </div>
);

export const PortfolioDashboardPage: React.FC<PortfolioDashboardPageProps> = ({ portfolioId }) => {
  const summaryQuery = useDashboardSummary(portfolioId);
  const cardsQuery = usePropertyPerformanceCards(portfolioId);
  const trendQuery = useCashFlowTrend(portfolioId);
  const alertsQuery = usePortfolioAlerts(portfolioId);
  const metricsQuery = usePortfolioMetricsSummary(portfolioId, ytdRange());
  const markRead = useMarkAlertRead(portfolioId);
  const resolve = useResolveAlert(portfolioId);

  // The summary is the primary resource — gate the page shell on it.
  if (summaryQuery.isLoading) {
    return (
      <div className="flex items-center justify-center py-24 text-sm text-gray-500">
        Loading portfolio dashboard…
      </div>
    );
  }

  if (summaryQuery.isError || !summaryQuery.data) {
    return (
      <div className="rounded-lg border border-red-200 bg-red-50 p-6">
        <h2 className="text-sm font-semibold text-red-800">Failed to load portfolio dashboard</h2>
        <p className="mt-1 text-sm text-red-700">
          {summaryQuery.error instanceof Error
            ? summaryQuery.error.message
            : 'The portfolio summary could not be retrieved. Please try again.'}
        </p>
        <button
          type="button"
          onClick={() => summaryQuery.refetch()}
          className="mt-4 px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700"
        >
          Retry
        </button>
      </div>
    );
  }

  const summary = summaryQuery.data;
  const alerts = alertsQuery.data ?? [];
  const propertyCards = cardsQuery.data ?? [];
  const cashFlowTrend = trendQuery.data ?? [];
  const metrics = metricsQuery.data?.property_metrics ?? [];

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Portfolio Dashboard</h1>
          <p className="text-sm text-gray-500">
            Portfolio ID: {portfolioId} | As of: {summary.as_of_date}
          </p>
        </div>
        <div className="flex space-x-3">
          <button
            type="button"
            className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50"
          >
            Export Report
          </button>
          <button
            type="button"
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700"
          >
            Calculate Metrics
          </button>
        </div>
      </div>

      {/* Summary Cards */}
      <SummaryCards summary={summary} />

      {/* Alerts Section */}
      {alerts.length > 0 && (
        <div className="space-y-3">
          <h2 className="text-lg font-semibold text-gray-900">Alerts</h2>
          {alerts.map((alert: PerformanceAlertDto) => (
            <PerformanceAlert
              key={alert.id}
              id={alert.id}
              alertType={alert.alert_type}
              severity={alert.severity}
              title={alert.title}
              message={alert.message}
              metricName={alert.metric_name ?? undefined}
              currentValue={optNum(alert.current_value)}
              thresholdValue={optNum(alert.threshold_value)}
              isRead={alert.is_read}
              isResolved={alert.is_resolved}
              createdAt={alert.created_at}
              onMarkRead={() => markRead.mutate(alert.id)}
              onResolve={() => resolve.mutate(alert.id)}
            />
          ))}
        </div>
      )}

      {/* Cash Flow Chart */}
      <div>
        {trendQuery.isLoading ? (
          <div className="rounded-lg border border-gray-200 bg-white p-6 text-sm text-gray-500">
            Loading cash-flow trend…
          </div>
        ) : trendQuery.isError ? (
          <div className="rounded-lg border border-red-200 bg-red-50 p-6 text-sm text-red-700">
            Failed to load cash-flow trend.
          </div>
        ) : (
          <CashFlowChart data={cashFlowTrend.map(mapCashFlowPoint)} currency={summary.currency} />
        )}
      </div>

      {/* Property Cards */}
      <div>
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Property Performance</h2>
          <button type="button" className="text-sm text-blue-600 hover:text-blue-800 font-medium">
            View All Properties
          </button>
        </div>
        {cardsQuery.isLoading ? (
          <div className="text-sm text-gray-500">Loading properties…</div>
        ) : cardsQuery.isError ? (
          <div className="text-sm text-red-700">Failed to load property performance.</div>
        ) : propertyCards.length === 0 ? (
          <div className="text-sm text-gray-500">No properties in this portfolio yet.</div>
        ) : (
          <div className="grid grid-cols-3 gap-4">
            {propertyCards.map((property: PropertyPerformanceCard) => (
              <PropertyCard
                key={property.property_id}
                {...mapPropertyCard(property)}
                onClick={() => {
                  // TODO: navigate to property detail route
                }}
              />
            ))}
          </div>
        )}
      </div>

      {/* Financial Metrics Table */}
      {metricsQuery.isError ? (
        <div className="rounded-lg border border-red-200 bg-red-50 p-6 text-sm text-red-700">
          Failed to load financial metrics.
        </div>
      ) : (
        <FinancialMetricsTable metrics={metrics.map(mapMetric)} title="Financial Metrics Summary" />
      )}
    </div>
  );
};
