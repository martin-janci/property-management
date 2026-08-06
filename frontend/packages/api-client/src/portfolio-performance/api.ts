/**
 * Portfolio Performance Analytics API client (Epic 144).
 *
 * Thin wrappers over the api-server routes mounted at
 * `/api/v1/portfolio-performance`. Requests go through the shared
 * {@link authenticatedFetchJson} helper so they inherit the centralized bearer
 * token injection and MFA-retry behaviour; the configured client `baseUrl` is
 * prefixed so an absolute API host (prod) is honoured, matching the financial
 * module's approach.
 */

import { client } from '../generated/client.gen';
import { authenticatedFetchJson } from '../lib/fetch';
import type {
  CashFlowTrendPoint,
  DashboardSummary,
  ListPortfolioAlertsParams,
  MetricsSummaryParams,
  PerformanceAlert,
  PortfolioMetricsSummary,
  PropertyPerformanceCard,
} from './types';

const API_BASE = '/api/v1/portfolio-performance';

function baseUrl(): string {
  return client.getConfig().baseUrl ?? '';
}

function buildQueryString(params: Record<string, unknown>): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  const qs = searchParams.toString();
  return qs ? `?${qs}` : '';
}

export function getDashboardSummary(portfolioId: string): Promise<DashboardSummary> {
  return authenticatedFetchJson<DashboardSummary>(
    `${baseUrl()}${API_BASE}/portfolios/${portfolioId}/dashboard/summary`
  );
}

export function getPropertyCards(portfolioId: string): Promise<PropertyPerformanceCard[]> {
  return authenticatedFetchJson<PropertyPerformanceCard[]>(
    `${baseUrl()}${API_BASE}/portfolios/${portfolioId}/dashboard/property-cards`
  );
}

export function getCashFlowTrend(
  portfolioId: string,
  months?: number
): Promise<CashFlowTrendPoint[]> {
  const qs = months !== undefined ? buildQueryString({ months }) : '';
  return authenticatedFetchJson<CashFlowTrendPoint[]>(
    `${baseUrl()}${API_BASE}/portfolios/${portfolioId}/dashboard/cash-flow-trend${qs}`
  );
}

export function listAlerts(
  portfolioId: string,
  params: ListPortfolioAlertsParams = {}
): Promise<PerformanceAlert[]> {
  return authenticatedFetchJson<PerformanceAlert[]>(
    `${baseUrl()}${API_BASE}/portfolios/${portfolioId}/alerts${buildQueryString({ ...params })}`
  );
}

export function getMetricsSummary(
  portfolioId: string,
  params: MetricsSummaryParams
): Promise<PortfolioMetricsSummary> {
  return authenticatedFetchJson<PortfolioMetricsSummary>(
    `${baseUrl()}${API_BASE}/portfolios/${portfolioId}/metrics/summary${buildQueryString({
      period_start: params.period_start,
      period_end: params.period_end,
    })}`
  );
}

export function markAlertRead(portfolioId: string, alertId: string): Promise<void> {
  return authenticatedFetchJson<void>(
    `${baseUrl()}${API_BASE}/portfolios/${portfolioId}/alerts/${alertId}/read`,
    { method: 'POST' }
  );
}

export function resolveAlert(portfolioId: string, alertId: string): Promise<void> {
  return authenticatedFetchJson<void>(
    `${baseUrl()}${API_BASE}/portfolios/${portfolioId}/alerts/${alertId}/resolve`,
    { method: 'POST' }
  );
}
