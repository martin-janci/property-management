// Epic 144: Portfolio Performance Analytics - Portfolio Dashboard Page
import type React from 'react';
import {
  BenchmarkComparisonCard,
  CashFlowChart,
  FinancialMetricsTable,
  MetricsSummaryCard,
  PerformanceAlert,
  PropertyCard,
} from '../components';

interface PortfolioDashboardPageProps {
  portfolioId: string;
}

// Mock data for demonstration - would be fetched from API
const mockDashboardSummary = {
  totalPortfolioValue: 2500000,
  totalEquity: 1200000,
  totalDebt: 1300000,
  debtToEquityRatio: 1.08,
  ltvRatio: 52,
  ytdNoi: 180000,
  ytdCashFlow: 95000,
  ytdReturnPct: 7.9,
  propertyCount: 4,
  currency: 'EUR',
};

const mockPropertyCards = [
  {
    id: '1',
    propertyName: 'Downtown Office Building',
    buildingAddress: '123 Main St',
    currentValue: 800000,
    equity: 400000,
    ltv: 50,
    noi: 64000,
    capRate: 8.0,
    cashOnCash: 12.5,
    dscr: 1.45,
    monthlyCashFlow: 3200,
    performanceStatus: 'excellent',
    currency: 'EUR',
  },
  {
    id: '2',
    propertyName: 'Residential Complex',
    buildingAddress: '456 Oak Ave',
    currentValue: 1200000,
    equity: 500000,
    ltv: 58.3,
    noi: 84000,
    capRate: 7.0,
    cashOnCash: 10.2,
    dscr: 1.25,
    monthlyCashFlow: 4200,
    performanceStatus: 'good',
    currency: 'EUR',
  },
  {
    id: '3',
    propertyName: 'Retail Strip',
    buildingAddress: '789 Commerce Blvd',
    currentValue: 500000,
    equity: 300000,
    ltv: 40,
    noi: 32000,
    capRate: 6.4,
    cashOnCash: 8.5,
    dscr: 1.15,
    monthlyCashFlow: 1800,
    performanceStatus: 'fair',
    currency: 'EUR',
  },
];

const mockCashFlowTrend = [
  {
    period: '2024-01',
    periodDate: '2024-01-01',
    grossIncome: 18000,
    operatingExpenses: 5000,
    noi: 13000,
    debtService: 8000,
    netCashFlow: 5000,
  },
  {
    period: '2024-02',
    periodDate: '2024-02-01',
    grossIncome: 18500,
    operatingExpenses: 5200,
    noi: 13300,
    debtService: 8000,
    netCashFlow: 5300,
  },
  {
    period: '2024-03',
    periodDate: '2024-03-01',
    grossIncome: 17800,
    operatingExpenses: 6500,
    noi: 11300,
    debtService: 8000,
    netCashFlow: 3300,
  },
  {
    period: '2024-04',
    periodDate: '2024-04-01',
    grossIncome: 18200,
    operatingExpenses: 5100,
    noi: 13100,
    debtService: 8000,
    netCashFlow: 5100,
  },
  {
    period: '2024-05',
    periodDate: '2024-05-01',
    grossIncome: 19000,
    operatingExpenses: 5300,
    noi: 13700,
    debtService: 8000,
    netCashFlow: 5700,
  },
  {
    period: '2024-06',
    periodDate: '2024-06-01',
    grossIncome: 18800,
    operatingExpenses: 5400,
    noi: 13400,
    debtService: 8000,
    netCashFlow: 5400,
  },
];

const mockBenchmark = {
  benchmarkName: 'Regional Multi-Family Index',
  benchmarkSource: 'Industry',
  comparisonDate: '2024-06-30',
  metrics: [
    {
      metricName: 'cap_rate',
      actualValue: 7.1,
      benchmarkValue: 6.5,
      variance: 0.6,
      variancePct: 9.2,
      percentile: 72,
      status: 'above_benchmark',
    },
    {
      metricName: 'cash_on_cash',
      actualValue: 10.2,
      benchmarkValue: 9.8,
      variance: 0.4,
      variancePct: 4.1,
      percentile: 65,
      status: 'above_benchmark',
    },
    {
      metricName: 'occupancy',
      actualValue: 94.5,
      benchmarkValue: 95.2,
      variance: -0.7,
      variancePct: -0.7,
      percentile: 45,
      status: 'below_benchmark',
    },
    {
      metricName: 'dscr',
      actualValue: 1.28,
      benchmarkValue: 1.25,
      variance: 0.03,
      variancePct: 2.4,
      percentile: 58,
      status: 'at_benchmark',
    },
  ],
  overallPerformance: 'Good',
  overallPercentile: 62,
  performanceScore: 74,
};

const mockAlerts = [
  {
    id: '1',
    alertType: 'metric_threshold',
    severity: 'warning',
    title: 'DSCR Below Target',
    message: 'Retail Strip property DSCR has fallen below your target of 1.25x',
    metricName: 'DSCR',
    currentValue: 1.15,
    thresholdValue: 1.25,
    isRead: false,
    isResolved: false,
    createdAt: '2024-06-28T10:30:00Z',
  },
  {
    id: '2',
    alertType: 'market_comparison',
    severity: 'info',
    title: 'Portfolio Outperforming Market',
    message: 'Your portfolio cap rate is 9.2% above the regional benchmark',
    isRead: true,
    isResolved: false,
    createdAt: '2024-06-25T14:15:00Z',
  },
];

const mockMetrics = [
  {
    propertyId: '1',
    propertyName: 'Downtown Office Building',
    noi: 64000,
    capRate: 8.0,
    cashOnCash: 12.5,
    irr: 15.2,
    dscr: 1.45,
    equityMultiple: 1.85,
    period: 'YTD 2024',
    currency: 'EUR',
  },
  {
    propertyId: '2',
    propertyName: 'Residential Complex',
    noi: 84000,
    capRate: 7.0,
    cashOnCash: 10.2,
    irr: 12.8,
    dscr: 1.25,
    equityMultiple: 1.62,
    period: 'YTD 2024',
    currency: 'EUR',
  },
  {
    propertyId: '3',
    propertyName: 'Retail Strip',
    noi: 32000,
    capRate: 6.4,
    cashOnCash: 8.5,
    irr: 10.5,
    dscr: 1.15,
    equityMultiple: 1.45,
    period: 'YTD 2024',
    currency: 'EUR',
  },
];

export const PortfolioDashboardPage: React.FC<PortfolioDashboardPageProps> = ({ portfolioId }) => {
  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Portfolio Dashboard</h1>
          <p className="text-sm text-gray-500">
            Portfolio ID: {portfolioId} | Last updated: June 30, 2024
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
      <div className="grid grid-cols-5 gap-4">
        <MetricsSummaryCard
          title="Portfolio Value"
          value={mockDashboardSummary.totalPortfolioValue}
          format="currency"
          currency={mockDashboardSummary.currency}
          change={5.2}
          changeLabel="vs last year"
          trend="up"
        />
        <MetricsSummaryCard
          title="Total Equity"
          value={mockDashboardSummary.totalEquity}
          format="currency"
          currency={mockDashboardSummary.currency}
          change={8.1}
          changeLabel="vs last year"
          trend="up"
        />
        <MetricsSummaryCard
          title="YTD NOI"
          value={mockDashboardSummary.ytdNoi}
          format="currency"
          currency={mockDashboardSummary.currency}
          change={3.5}
          changeLabel="vs prior YTD"
          trend="up"
        />
        <MetricsSummaryCard
          title="YTD Cash Flow"
          value={mockDashboardSummary.ytdCashFlow}
          format="currency"
          currency={mockDashboardSummary.currency}
          change={-2.1}
          changeLabel="vs prior YTD"
          trend="down"
        />
        <MetricsSummaryCard
          title="LTV Ratio"
          value={mockDashboardSummary.ltvRatio}
          format="percent"
          subtitle={`D/E: ${mockDashboardSummary.debtToEquityRatio.toFixed(2)}x`}
        />
      </div>

      {/* Alerts Section */}
      {mockAlerts.length > 0 && (
        <div className="space-y-3">
          <h2 className="text-lg font-semibold text-gray-900">Alerts</h2>
          {mockAlerts.map((alert) => (
            <PerformanceAlert
              key={alert.id}
              {...alert}
              onMarkRead={() => {
                // TODO: wire to alerts API (mark-read mutation)
              }}
              onResolve={() => {
                // TODO: wire to alerts API (resolve mutation)
              }}
            />
          ))}
        </div>
      )}

      {/* Main Content Grid */}
      <div className="grid grid-cols-3 gap-6">
        {/* Cash Flow Chart - 2 columns */}
        <div className="col-span-2">
          <CashFlowChart data={mockCashFlowTrend} currency={mockDashboardSummary.currency} />
        </div>

        {/* Benchmark Comparison - 1 column */}
        <div>
          <BenchmarkComparisonCard {...mockBenchmark} />
        </div>
      </div>

      {/* Property Cards */}
      <div>
        <div className="flex justify-between items-center mb-4">
          <h2 className="text-lg font-semibold text-gray-900">Property Performance</h2>
          <button type="button" className="text-sm text-blue-600 hover:text-blue-800 font-medium">
            View All Properties
          </button>
        </div>
        <div className="grid grid-cols-3 gap-4">
          {mockPropertyCards.map((property) => (
            <PropertyCard
              key={property.id}
              {...property}
              onClick={() => {
                // TODO: navigate to property detail route
              }}
            />
          ))}
        </div>
      </div>

      {/* Financial Metrics Table */}
      <FinancialMetricsTable metrics={mockMetrics} title="Financial Metrics Summary" />
    </div>
  );
};
