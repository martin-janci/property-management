/**
 * Portfolio Performance Analytics hooks (Epic 144).
 *
 * TanStack Query hooks over {@link ./api}, following the same query-key +
 * invalidation conventions as the other api-client modules (e.g. community).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type { ListPortfolioAlertsParams, MetricsSummaryParams } from './types';

export const portfolioPerformanceKeys = {
  all: ['portfolio-performance'] as const,
  portfolio: (id: string) => [...portfolioPerformanceKeys.all, id] as const,
  dashboardSummary: (id: string) =>
    [...portfolioPerformanceKeys.portfolio(id), 'dashboard', 'summary'] as const,
  propertyCards: (id: string) =>
    [...portfolioPerformanceKeys.portfolio(id), 'dashboard', 'property-cards'] as const,
  cashFlowTrend: (id: string, months?: number) =>
    [...portfolioPerformanceKeys.portfolio(id), 'dashboard', 'cash-flow-trend', months] as const,
  alerts: (id: string, params: ListPortfolioAlertsParams) =>
    [...portfolioPerformanceKeys.portfolio(id), 'alerts', params] as const,
  metricsSummary: (id: string, params: MetricsSummaryParams) =>
    [...portfolioPerformanceKeys.portfolio(id), 'metrics', 'summary', params] as const,
};

export function useDashboardSummary(portfolioId: string) {
  return useQuery({
    queryKey: portfolioPerformanceKeys.dashboardSummary(portfolioId),
    queryFn: () => api.getDashboardSummary(portfolioId),
    enabled: !!portfolioId,
  });
}

export function usePropertyPerformanceCards(portfolioId: string) {
  return useQuery({
    queryKey: portfolioPerformanceKeys.propertyCards(portfolioId),
    queryFn: () => api.getPropertyCards(portfolioId),
    enabled: !!portfolioId,
  });
}

export function useCashFlowTrend(portfolioId: string, months?: number) {
  return useQuery({
    queryKey: portfolioPerformanceKeys.cashFlowTrend(portfolioId, months),
    queryFn: () => api.getCashFlowTrend(portfolioId, months),
    enabled: !!portfolioId,
  });
}

export function usePortfolioAlerts(portfolioId: string, params: ListPortfolioAlertsParams = {}) {
  return useQuery({
    queryKey: portfolioPerformanceKeys.alerts(portfolioId, params),
    queryFn: () => api.listAlerts(portfolioId, params),
    enabled: !!portfolioId,
  });
}

export function usePortfolioMetricsSummary(portfolioId: string, params: MetricsSummaryParams) {
  return useQuery({
    queryKey: portfolioPerformanceKeys.metricsSummary(portfolioId, params),
    queryFn: () => api.getMetricsSummary(portfolioId, params),
    enabled: !!portfolioId && !!params.period_start && !!params.period_end,
  });
}

export function useMarkAlertRead(portfolioId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (alertId: string) => api.markAlertRead(portfolioId, alertId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [...portfolioPerformanceKeys.portfolio(portfolioId), 'alerts'],
      });
    },
  });
}

export function useResolveAlert(portfolioId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (alertId: string) => api.resolveAlert(portfolioId, alertId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: [...portfolioPerformanceKeys.portfolio(portfolioId), 'alerts'],
      });
    },
  });
}
