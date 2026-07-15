/**
 * Reports API client (Epic 53).
 *
 * Client functions for reports, schedules, analytics, and trends.
 */

import type { FaultCategoryCount, FaultPriorityCount, FaultStatusCount } from '../faults/types';
import type {
  AnalyticsParams,
  AnalyticsSummary,
  BuildingAnalytics,
  CreateReportDefinition,
  CreateReportSchedule,
  CronScheduleUpdateRequest,
  DataSource,
  ListSchedulesParams,
  ListSchedulesResponse,
  PeriodComparison,
  ReportDefinition,
  ReportResult,
  ReportRun,
  ReportSchedule,
  TrendAnalysis,
  TrendLine,
  TrendParams,
} from './types';

const API_BASE = '/api/v1/reports';

// NOTE: Authentication headers (Authorization, etc.) are handled at a higher level
// by the global fetch interceptor or API client wrapper, not in individual API calls.

async function fetchApi<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(error.message || `HTTP ${response.status}`);
  }

  return response.json();
}

// ============================================================================
// REPORT DEFINITIONS
// ============================================================================

export async function createReport(
  organizationId: string,
  data: CreateReportDefinition
): Promise<ReportDefinition> {
  return fetchApi(`${API_BASE}/definitions`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, ...data }),
  });
}

export async function getReport(id: string): Promise<ReportDefinition> {
  return fetchApi(`${API_BASE}/definitions/${id}`);
}

export async function updateReport(
  id: string,
  data: Partial<CreateReportDefinition>
): Promise<ReportDefinition> {
  return fetchApi(`${API_BASE}/definitions/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(data),
  });
}

export async function deleteReport(id: string): Promise<void> {
  await fetchApi(`${API_BASE}/definitions/${id}`, { method: 'DELETE' });
}

export async function runReport(id: string): Promise<ReportResult> {
  return fetchApi(`${API_BASE}/definitions/${id}/run`, { method: 'POST' });
}

export async function previewReport(data: CreateReportDefinition): Promise<ReportResult> {
  return fetchApi(`${API_BASE}/preview`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

// ============================================================================
// DATA SOURCES
// ============================================================================

export async function listDataSources(organizationId: string): Promise<DataSource[]> {
  return fetchApi(`${API_BASE}/data-sources?organization_id=${organizationId}`);
}

export async function getDataSourceFields(sourceId: string): Promise<DataSource> {
  return fetchApi(`${API_BASE}/data-sources/${sourceId}`);
}

// ============================================================================
// REPORT SCHEDULES
// ============================================================================

export async function createSchedule(
  organizationId: string,
  data: CreateReportSchedule
): Promise<ReportSchedule> {
  return fetchApi(`${API_BASE}/schedules`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, ...data }),
  });
}

export async function listSchedules(params: ListSchedulesParams): Promise<ListSchedulesResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.report_id) searchParams.set('report_id', params.report_id);
  if (params.is_active !== undefined) searchParams.set('is_active', String(params.is_active));
  return fetchApi(`${API_BASE}/schedules?${searchParams}`);
}

export async function getSchedule(id: string): Promise<ReportSchedule> {
  return fetchApi(`${API_BASE}/schedules/${id}`);
}

export async function updateSchedule(
  id: string,
  data: Partial<CreateReportSchedule>
): Promise<ReportSchedule> {
  return fetchApi(`${API_BASE}/schedules/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(data),
  });
}

/**
 * Update a report schedule using the gap-81-1 cron-based endpoint.
 *
 * Sends a PUT request with `cron_expression`, `recipients`, and/or `enabled`.
 * At least one field must be present (validated by the backend).
 */
export async function updateScheduleCron(
  id: string,
  data: CronScheduleUpdateRequest
): Promise<ReportSchedule> {
  return fetchApi(`${API_BASE}/schedules/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deleteSchedule(id: string): Promise<void> {
  await fetchApi(`${API_BASE}/schedules/${id}`, { method: 'DELETE' });
}

export async function toggleSchedule(id: string, isActive: boolean): Promise<ReportSchedule> {
  return fetchApi(`${API_BASE}/schedules/${id}/toggle`, {
    method: 'POST',
    body: JSON.stringify({ is_active: isActive }),
  });
}

export async function runScheduleNow(id: string): Promise<ReportRun> {
  return fetchApi(`${API_BASE}/schedules/${id}/run`, { method: 'POST' });
}

export async function listScheduleRuns(scheduleId: string, limit?: number): Promise<ReportRun[]> {
  const params = limit ? `?limit=${limit}` : '';
  return fetchApi(`${API_BASE}/schedules/${scheduleId}/runs${params}`);
}

// ============================================================================
// EPIC 81: SCHEDULE MANAGEMENT
// ============================================================================

/**
 * Pause a report schedule (Story 81.1).
 */
export async function pauseSchedule(id: string): Promise<ReportSchedule> {
  return fetchApi(`${API_BASE}/schedules/${id}/pause`, { method: 'PUT' });
}

/**
 * Resume a paused report schedule (Story 81.1).
 */
export async function resumeSchedule(id: string): Promise<ReportSchedule> {
  return fetchApi(`${API_BASE}/schedules/${id}/resume`, { method: 'PUT' });
}

// ============================================================================
// EPIC 81: EXECUTION HISTORY
// ============================================================================

import type {
  ReportDownloadUrlResponse,
  ReportExecution,
  ReportExecutionHistoryParams,
  ReportExecutionHistoryResponse,
} from './types';

/**
 * Get execution history for a schedule (Story 81.2).
 */
export async function getReportExecutionHistory(
  params: ReportExecutionHistoryParams
): Promise<ReportExecutionHistoryResponse> {
  const searchParams = new URLSearchParams();
  if (params.status) searchParams.set('status', params.status);
  if (params.dateFrom) searchParams.set('date_from', params.dateFrom);
  if (params.dateTo) searchParams.set('date_to', params.dateTo);
  if (params.limit) searchParams.set('limit', String(params.limit));
  if (params.offset) searchParams.set('offset', String(params.offset));

  const queryString = searchParams.toString();
  const url = `${API_BASE}/schedules/${params.scheduleId}/executions${queryString ? `?${queryString}` : ''}`;
  return fetchApi(url);
}

/**
 * Get a single execution details (Story 81.2).
 */
export async function getExecution(executionId: string): Promise<ReportExecution> {
  return fetchApi(`${API_BASE}/executions/${executionId}`);
}

/**
 * Get download URL for a completed execution (Story 81.2).
 */
export async function getReportExecutionDownloadUrl(
  executionId: string
): Promise<ReportDownloadUrlResponse> {
  return fetchApi(`${API_BASE}/executions/${executionId}/download`);
}

/**
 * Retry a failed report execution (Story 81.2).
 */
export async function retryReportExecution(executionId: string): Promise<ReportExecution> {
  return fetchApi(`${API_BASE}/executions/${executionId}/retry`, { method: 'POST' });
}

// ============================================================================
// ANALYTICS
// ============================================================================

export async function getAnalyticsSummary(params: AnalyticsParams): Promise<AnalyticsSummary> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  if (params.start_date) searchParams.set('start_date', params.start_date);
  if (params.end_date) searchParams.set('end_date', params.end_date);
  return fetchApi(`${API_BASE}/analytics/summary?${searchParams}`);
}

export async function getBuildingAnalytics(
  buildingId: string,
  params?: Omit<AnalyticsParams, 'organization_id' | 'building_id'>
): Promise<BuildingAnalytics> {
  const searchParams = new URLSearchParams();
  if (params?.start_date) searchParams.set('start_date', params.start_date);
  if (params?.end_date) searchParams.set('end_date', params.end_date);
  return fetchApi(`${API_BASE}/analytics/buildings/${buildingId}?${searchParams}`);
}

// ============================================================================
// TREND ANALYSIS
// ============================================================================

export async function getTrendAnalysis(params: TrendParams): Promise<TrendAnalysis> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  searchParams.set('metric', params.metric);
  searchParams.set('period', params.period);
  searchParams.set('start_date', params.start_date);
  searchParams.set('end_date', params.end_date);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  if (params.compare_previous) searchParams.set('compare_previous', 'true');
  return fetchApi(`${API_BASE}/trends?${searchParams}`);
}

export async function getTrendLines(params: TrendParams): Promise<TrendLine[]> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  searchParams.set('metric', params.metric);
  searchParams.set('period', params.period);
  searchParams.set('start_date', params.start_date);
  searchParams.set('end_date', params.end_date);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  return fetchApi(`${API_BASE}/trends/lines?${searchParams}`);
}

export async function comparePeriods(
  organizationId: string,
  metric: string,
  periods: { start_date: string; end_date: string; label: string }[]
): Promise<PeriodComparison> {
  return fetchApi(`${API_BASE}/trends/compare`, {
    method: 'POST',
    body: JSON.stringify({ organization_id: organizationId, metric, periods }),
  });
}

// ============================================================================
// EPIC 55: ADVANCED REPORTS - Types
// ============================================================================

/** Date range for reports. */
export interface DateRange {
  from: string;
  to: string;
}

/** Monthly count data point. */
export interface MonthlyCount {
  year: number;
  month: number;
  count: number;
}

/** Monthly average data point. */
export interface MonthlyAverage {
  year: number;
  month: number;
  average: number;
}

/**
 * Fault statistics summary used in the reports endpoint.
 *
 * Uses the shared count types from the faults module so that status/
 * category/priority buckets are defined once and reused across both the
 * manager dashboard (`/api/v1/faults/statistics`) and the reports endpoint.
 */
export interface FaultStatisticsSummary {
  total_count: number;
  open_count: number;
  closed_count: number;
  by_status: FaultStatusCount[];
  by_category: FaultCategoryCount[];
  by_priority: FaultPriorityCount[];
  average_resolution_time_hours: number | null;
  average_rating: number | null;
}

/** Fault trends over time. */
export interface FaultTrends {
  monthly_counts: MonthlyCount[];
  resolution_time_trend: MonthlyAverage[];
  category_trend: Array<{ category: string; monthly_counts: MonthlyCount[] }>;
}

/** Fault statistics report response. */
export interface FaultStatisticsReportResponse {
  building_id: string | null;
  building_name: string | null;
  date_range: DateRange;
  statistics: FaultStatisticsSummary;
  trends: FaultTrends;
}

/** Voting participation summary. */
export interface VotingParticipationSummary {
  total_votes: number;
  votes_with_quorum: number;
  votes_without_quorum: number;
  average_participation_rate: number;
  total_eligible_voters: number;
  total_responses: number;
}

/** Individual vote participation detail. */
export interface VoteParticipationDetail {
  vote_id: string;
  title: string;
  status: string;
  start_at: string | null;
  end_at: string;
  eligible_count: number;
  response_count: number;
  participation_rate: number;
  quorum_required: number | null;
  quorum_reached: boolean;
}

/** Voting participation report response. */
export interface VotingParticipationReportResponse {
  building_id: string | null;
  building_name: string | null;
  date_range: DateRange;
  summary: VotingParticipationSummary;
  votes: VoteParticipationDetail[];
}

/** Occupancy summary. */
export interface OccupancySummary {
  total_units: number;
  occupied_units: number;
  vacant_units: number;
  occupancy_rate: number;
  total_person_months: number;
  average_occupants_per_unit: number;
}

/** Year-over-year comparison. */
export interface YearComparison {
  current_year: number;
  previous_year: number;
  current_total: number;
  previous_total: number;
  change_percentage: number;
}

/** Occupancy trends. */
export interface OccupancyTrends {
  monthly_total: MonthlyCount[];
  year_over_year_comparison: YearComparison | null;
}

/** Occupancy report response. */
export interface OccupancyReportResponse {
  building_id: string | null;
  building_name: string | null;
  date_range: DateRange;
  summary: OccupancySummary;
  by_unit: Array<{
    unit_id: string;
    unit_designation: string;
    person_months: Array<{ year: number; month: number; count: number }>;
    total_person_months: number;
    average_occupants: number;
  }>;
  trends: OccupancyTrends;
}

/** Consumption summary. */
export interface ConsumptionSummary {
  total_consumption: string;
  total_cost: string;
  meter_count: number;
  average_consumption_per_unit: string;
}

/** Consumption anomaly. */
export interface ConsumptionAnomaly {
  unit_id: string;
  unit_designation: string;
  meter_id: string;
  utility_type: string;
  reading_date: string;
  consumption: string;
  expected_consumption: string;
  deviation_percentage: number;
  severity: 'low' | 'medium' | 'high';
}

/** Consumption report response. */
export interface ConsumptionReportResponse {
  building_id: string | null;
  building_name: string | null;
  date_range: DateRange;
  summary: ConsumptionSummary;
  by_utility_type: Array<{
    utility_type: string;
    total_consumption: string;
    unit: string;
    total_cost: string;
    meter_count: number;
    monthly_data: Array<{ year: number; month: number; consumption: string; cost: string }>;
  }>;
  by_unit: Array<{
    unit_id: string;
    unit_designation: string;
    utility_type: string;
    total_consumption: string;
    average_monthly: string;
    is_above_average: boolean;
    deviation_percentage: number;
  }>;
  anomalies: ConsumptionAnomaly[];
}

/** Export report response. */
export interface ExportReportResponse {
  download_url: string;
  format: string;
  expires_at: string;
}

// ============================================================================
// EPIC 55: ADVANCED REPORTS - Query Parameters
// ============================================================================

export interface FaultStatisticsQuery {
  organization_id: string;
  building_id?: string;
  from_date?: string;
  to_date?: string;
}

export interface VotingParticipationQuery {
  organization_id: string;
  building_id?: string;
  from_date?: string;
  to_date?: string;
}

export interface OccupancyReportQuery {
  organization_id: string;
  building_id?: string;
  year: number;
  month?: number;
  include_comparison?: boolean;
}

export interface ConsumptionReportQuery {
  organization_id: string;
  building_id?: string;
  utility_type?: string;
  from_date: string;
  to_date: string;
  include_anomalies?: boolean;
}

export interface ExportReportRequest {
  organization_id: string;
  report_type: 'faults' | 'voting' | 'occupancy' | 'consumption';
  format: 'pdf' | 'excel' | 'csv';
  building_id?: string;
  from_date?: string;
  to_date?: string;
  params?: Record<string, unknown>;
}

// ============================================================================
// EPIC 55: ADVANCED REPORTS - API Functions
// ============================================================================

/**
 * Get fault statistics report (Story 55.1).
 */
export async function getFaultStatisticsReport(
  params: FaultStatisticsQuery
): Promise<FaultStatisticsReportResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  if (params.from_date) searchParams.set('from_date', params.from_date);
  if (params.to_date) searchParams.set('to_date', params.to_date);
  return fetchApi(`${API_BASE}/faults?${searchParams}`);
}

/**
 * Get voting participation report (Story 55.2).
 */
export async function getVotingParticipationReport(
  params: VotingParticipationQuery
): Promise<VotingParticipationReportResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  if (params.from_date) searchParams.set('from_date', params.from_date);
  if (params.to_date) searchParams.set('to_date', params.to_date);
  return fetchApi(`${API_BASE}/voting?${searchParams}`);
}

/**
 * Get occupancy report (Story 55.3).
 */
export async function getOccupancyReport(
  params: OccupancyReportQuery
): Promise<OccupancyReportResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  searchParams.set('year', String(params.year));
  if (params.month) searchParams.set('month', String(params.month));
  if (params.include_comparison) searchParams.set('include_comparison', 'true');
  return fetchApi(`${API_BASE}/occupancy?${searchParams}`);
}

/**
 * Get consumption report (Story 55.4).
 */
export async function getConsumptionReport(
  params: ConsumptionReportQuery
): Promise<ConsumptionReportResponse> {
  const searchParams = new URLSearchParams();
  searchParams.set('organization_id', params.organization_id);
  if (params.building_id) searchParams.set('building_id', params.building_id);
  if (params.utility_type) searchParams.set('utility_type', params.utility_type);
  searchParams.set('from_date', params.from_date);
  searchParams.set('to_date', params.to_date);
  if (params.include_anomalies) searchParams.set('include_anomalies', 'true');
  return fetchApi(`${API_BASE}/consumption?${searchParams}`);
}

/**
 * Export a report to PDF/Excel/CSV (Story 55.5).
 */
export async function exportAdvancedReport(
  data: ExportReportRequest
): Promise<ExportReportResponse> {
  return fetchApi(`${API_BASE}/export`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

// ============================================================================
// EXPORT (LEGACY FROM EPIC 53)
// ============================================================================

export async function exportReport(
  reportId: string,
  format: 'pdf' | 'excel' | 'csv'
): Promise<{ download_url: string }> {
  return fetchApi(`${API_BASE}/definitions/${reportId}/export`, {
    method: 'POST',
    body: JSON.stringify({ format }),
  });
}

export async function exportDashboard(
  dashboardId: string,
  format: 'pdf'
): Promise<{ download_url: string }> {
  return fetchApi(`${API_BASE}/dashboards/${dashboardId}/export`, {
    method: 'POST',
    body: JSON.stringify({ format }),
  });
}
