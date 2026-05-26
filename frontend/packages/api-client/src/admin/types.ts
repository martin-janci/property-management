/**
 * Admin (Super-admin Control Plane) API types — Phase 5 / Epic 10A-2.
 *
 * Mirrors the JSON shapes returned by the api-server's `/api/v1/admin/*`
 * endpoints. Hand-written for now; will be replaced by generated types once
 * the OpenAPI spec for the admin surface is regenerated.
 */

export type AgencyStatus = 'active' | 'suspended' | 'archived' | string;

export interface Agency {
  id: string;
  name: string;
  slug: string;
  status: AgencyStatus;
  member_count: number;
}

export interface ListAgenciesParams {
  page?: number;
  page_size?: number;
  q?: string;
}

export interface AdminPaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

// ============================================================
// OAuth Client Management (Epic 10A-2)
// ============================================================

export interface OAuthClientSummary {
  id: string;
  clientId: string;
  name: string;
  description: string | null;
  redirectUris: string[];
  scopes: string[];
  isActive: boolean;
  createdAt: string;
}

export interface RegisterOAuthClientRequest {
  name: string;
  description?: string;
  redirectUris: string[];
  scopes: string[];
  isConfidential?: boolean;
  rotateRefreshTokens?: boolean;
}

export interface RegisterOAuthClientResponse {
  id: string;
  clientId: string;
  clientSecret: string;
  name: string;
  redirectUris: string[];
  scopes: string[];
  createdAt: string;
}

export interface UpdateOAuthClientRequest {
  name?: string;
  description?: string;
  redirectUris?: string[];
  scopes?: string[];
  isActive?: boolean;
  rotateRefreshTokens?: boolean;
}

export interface RegenerateSecretResponse {
  clientSecret: string;
}

/** Known OAuth scopes served by the PPT api-server. */
export const KNOWN_OAUTH_SCOPES = ['profile', 'email', 'org:read', 'full'] as const;
export type KnownOAuthScope = (typeof KNOWN_OAUTH_SCOPES)[number];

// ============================================================
// Platform Health Monitoring (Epic 10B.3)
// ============================================================

export type MetricStatus = 'normal' | 'warning' | 'critical';

export interface CurrentMetric {
  metric_name: string;
  metric_type: string;
  value: number;
  recorded_at: string;
  status: MetricStatus;
}

export interface MetricAlert {
  id: string;
  metric_name: string;
  threshold_type: string;
  value: number;
  created_at: string;
  acknowledged_at: string | null;
  acknowledged_by: string | null;
}

export interface MetricThreshold {
  id: string;
  metric_name: string;
  warning_threshold: number;
  critical_threshold: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface HealthDashboard {
  metrics: CurrentMetric[];
  alerts: MetricAlert[];
  thresholds: MetricThreshold[];
}

export interface MetricDataPoint {
  value: number;
  recorded_at: string;
}

export interface MetricStats {
  min: number;
  max: number;
  avg: number;
  count: number;
}

export interface MetricHistory {
  metric_name: string;
  data_points: MetricDataPoint[];
  stats: MetricStats;
}

export type TimeRange = '1h' | '6h' | '24h' | '7d' | '30d';

export interface UpdateThresholdRequest {
  warning_threshold?: number;
  critical_threshold?: number;
}

// ============================================================
// System Announcements (Epic 10B.4)
// ============================================================

export type SystemAnnouncementSeverity = 'info' | 'warning' | 'critical';

export interface SystemAnnouncement {
  id: string;
  title: string;
  message: string;
  severity: SystemAnnouncementSeverity;
  start_at: string;
  end_at: string | null;
  is_dismissible: boolean;
  requires_acknowledgment: boolean;
  created_by: string;
  created_at: string;
  updated_at: string;
}

export interface CreateSystemAnnouncementRequest {
  title: string;
  message: string;
  severity: SystemAnnouncementSeverity;
  start_at: string;
  end_at?: string | null;
  is_dismissible?: boolean;
  requires_acknowledgment?: boolean;
}

export interface UpdateSystemAnnouncementRequest {
  title?: string;
  message?: string;
  severity?: SystemAnnouncementSeverity;
  start_at?: string;
  end_at?: string | null;
  is_dismissible?: boolean;
  requires_acknowledgment?: boolean;
}

export interface ListSystemAnnouncementsParams {
  include_deleted?: boolean;
}

export interface CreateSystemAnnouncementResponse {
  message: string;
  announcement: SystemAnnouncement;
}
