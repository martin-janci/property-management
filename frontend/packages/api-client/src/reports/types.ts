/**
 * Reports and Analytics types (Epic 53).
 *
 * Types for custom reports, scheduled reports, analytics dashboards, and trend analysis.
 */

// ============================================================================
// ENUMS
// ============================================================================

export type ReportType = 'financial' | 'occupancy' | 'maintenance' | 'utility' | 'custom';

export type ReportFormat = 'pdf' | 'excel' | 'csv' | 'json';

export type ScheduleFrequency = 'daily' | 'weekly' | 'monthly' | 'quarterly' | 'yearly';

export type AggregationType = 'sum' | 'avg' | 'min' | 'max' | 'count';

export type ChartType = 'line' | 'bar' | 'pie' | 'area' | 'stacked_bar';

export type TrendDirection = 'up' | 'down' | 'stable';

// ============================================================================
// REPORT BUILDER
// ============================================================================

export interface ReportField {
  id: string;
  name: string;
  type: 'text' | 'number' | 'date' | 'currency' | 'percentage';
  source: string;
  aggregation?: AggregationType;
}

export interface ReportFilter {
  field_id: string;
  operator: 'eq' | 'ne' | 'gt' | 'gte' | 'lt' | 'lte' | 'in' | 'between' | 'contains';
  value: string | number | string[] | [number, number];
}

export interface ReportGrouping {
  field_id: string;
  order: 'asc' | 'desc';
}

export interface ReportDefinition {
  id: string;
  organization_id: string;
  name: string;
  description?: string;
  report_type: ReportType;
  data_source: string;
  fields: ReportField[];
  filters: ReportFilter[];
  groupings: ReportGrouping[];
  chart_type?: ChartType;
  is_public: boolean;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface CreateReportDefinition {
  name: string;
  description?: string;
  report_type?: ReportType;
  data_source: string;
  fields: Omit<ReportField, 'id'>[];
  filters?: ReportFilter[];
  groupings?: ReportGrouping[];
  chart_type?: ChartType;
  is_public?: boolean;
}

export interface ReportResult {
  report_id: string;
  generated_at: string;
  data: Record<string, unknown>[];
  totals?: Record<string, number>;
  metadata: {
    row_count: number;
    execution_time_ms: number;
  };
}

// ============================================================================
// SCHEDULED REPORTS
// ============================================================================

export interface ReportSchedule {
  id: string;
  report_id: string;
  organization_id: string;
  name: string;
  frequency: ScheduleFrequency;
  day_of_week?: number; // 0-6 for weekly
  day_of_month?: number; // 1-31 for monthly
  time: string; // HH:mm format (legacy; prefer cron_expression for new code)
  /**
   * 5-field UNIX cron expression (gap-81-1 / issue #616).
   *
   * Source of truth for the schedule when present — the dedicated
   * `cron_expression` column lets edits round-trip losslessly instead of
   * being reconstructed from the overloaded `time`/`frequency` fields.
   * Optional for back-compat with legacy rows that predate the column.
   */
  cron_expression?: string;
  timezone: string;
  format: ReportFormat;
  recipients: string[];
  is_active: boolean;
  last_run_at?: string;
  next_run_at?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateReportSchedule {
  report_id: string;
  name: string;
  frequency: ScheduleFrequency;
  day_of_week?: number;
  day_of_month?: number;
  time: string;
  timezone?: string;
  format?: ReportFormat;
  recipients: string[];
}

export interface ReportRun {
  id: string;
  schedule_id: string;
  status: 'pending' | 'running' | 'completed' | 'failed';
  started_at: string;
  completed_at?: string;
  file_url?: string;
  error_message?: string;
  recipients_notified: number;
}

// ============================================================================
// EPIC 81: EXECUTION HISTORY
// ============================================================================

export type ReportExecutionStatus =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled'
  | 'skipped';

export interface ReportExecutionError {
  code: string;
  message: string;
  details?: string;
}

export interface ReportExecution {
  id: string;
  scheduleId: string;
  status: ReportExecutionStatus;
  startedAt: string;
  completedAt?: string;
  durationMs?: number;
  fileKey?: string;
  fileName?: string;
  fileSize?: number;
  error?: ReportExecutionError;
  createdAt: string;
}

export interface ReportExecutionHistoryParams {
  scheduleId: string;
  status?: ReportExecutionStatus;
  dateFrom?: string;
  dateTo?: string;
  limit?: number;
  offset?: number;
}

export interface ReportExecutionHistoryResponse {
  executions: ReportExecution[];
  total: number;
  hasMore: boolean;
}

export interface ReportDownloadUrlResponse {
  url: string;
  expiresAt: string;
  fileName: string;
  contentType: string;
}

export interface UpdateScheduleRequest {
  name?: string;
  report_id?: string;
  frequency?: ScheduleFrequency;
  day_of_week?: number;
  day_of_month?: number;
  time?: string;
  timezone?: string;
  format?: ReportFormat;
  recipients?: string[];
  is_active?: boolean;
}

/**
 * Request body for the gap-81-1 PUT /api/v1/reports/schedules/{id} endpoint.
 *
 * The backend accepts a cron expression (5-field UNIX), a recipient list,
 * and an enabled flag — all optional (at least one must be present).
 */
export interface CronScheduleUpdateRequest {
  /** 5-field UNIX cron expression, e.g. "0 8 * * 1" */
  cron_expression?: string;
  /** Full replacement list of recipient email addresses */
  recipients?: string[];
  /** Whether the schedule is active/enabled */
  enabled?: boolean;
}

// ============================================================================
// DASHBOARD ANALYTICS
// ============================================================================

export interface DashboardWidget {
  id: string;
  dashboard_id: string;
  widget_type: 'kpi' | 'chart' | 'table' | 'list';
  title: string;
  data_source: string;
  config: Record<string, unknown>;
  position: { x: number; y: number; w: number; h: number };
  refresh_interval?: number; // seconds
}

export interface Dashboard {
  id: string;
  organization_id: string;
  name: string;
  description?: string;
  widgets: DashboardWidget[];
  is_default: boolean;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface KPIMetric {
  id: string;
  name: string;
  value: number;
  previous_value?: number;
  change_percentage?: number;
  trend: TrendDirection;
  unit?: string;
  target?: number;
}

export interface OccupancyMetrics {
  total_units: number;
  occupied_units: number;
  vacant_units: number;
  occupancy_rate: number;
  average_rent: number;
  rent_collected: number;
}

export interface MaintenanceMetrics {
  open_faults: number;
  resolved_this_month: number;
  average_resolution_days: number;
  by_priority: { high: number; medium: number; low: number };
  by_category: Record<string, number>;
}

export interface FinancialMetrics {
  total_revenue: number;
  total_expenses: number;
  net_income: number;
  outstanding_invoices: number;
  collection_rate: number;
  budget_variance: number;
}

export interface UtilityMetrics {
  total_consumption: Record<string, number>; // by utility type
  total_cost: number;
  average_per_unit: Record<string, number>;
  year_over_year_change: Record<string, number>;
}

export interface BuildingAnalytics {
  building_id: string;
  building_name: string;
  occupancy: OccupancyMetrics;
  maintenance: MaintenanceMetrics;
  financial: FinancialMetrics;
  utility: UtilityMetrics;
}

// ============================================================================
// TREND ANALYSIS
// ============================================================================

export interface TimeSeriesPoint {
  date: string;
  value: number;
  label?: string;
}

export interface TrendLine {
  id: string;
  name: string;
  data: TimeSeriesPoint[];
  color?: string;
  trend: TrendDirection;
  change_percentage: number;
}

export interface TrendAnalysis {
  metric: string;
  period: string;
  current_value: number;
  previous_value: number;
  change: number;
  change_percentage: number;
  trend: TrendDirection;
  forecast?: number;
  anomalies: TimeSeriesPoint[];
}

export interface PeriodComparison {
  metric: string;
  periods: {
    label: string;
    start_date: string;
    end_date: string;
    value: number;
  }[];
  difference: number;
  difference_percentage: number;
}

// ============================================================================
// DATA SOURCES
// ============================================================================

export interface DataSource {
  id: string;
  name: string;
  description: string;
  available_fields: ReportField[];
  category: 'financial' | 'operations' | 'occupancy' | 'utility' | 'maintenance';
}

// ============================================================================
// QUERY PARAMS
// ============================================================================

export interface ListReportsParams {
  organization_id: string;
  type?: ReportType;
  created_by?: string;
  limit?: number;
  offset?: number;
}

export interface ListSchedulesParams {
  organization_id: string;
  report_id?: string;
  is_active?: boolean;
}

export interface AnalyticsParams {
  organization_id: string;
  building_id?: string;
  start_date?: string;
  end_date?: string;
}

export interface TrendParams {
  organization_id: string;
  building_id?: string;
  metric: string;
  period: 'daily' | 'weekly' | 'monthly' | 'quarterly' | 'yearly';
  start_date: string;
  end_date: string;
  compare_previous?: boolean;
}

// ============================================================================
// RESPONSES
// ============================================================================

export interface ListReportsResponse {
  reports: ReportDefinition[];
  total: number;
}

export interface ListSchedulesResponse {
  schedules: ReportSchedule[];
  total: number;
}

export interface AnalyticsSummary {
  kpis: KPIMetric[];
  buildings: BuildingAnalytics[];
  trends: TrendLine[];
}
