/**
 * Compliance API Types (Epic 90 - Frontend API Integration).
 * Originally from Epic 67, integrated as part of Epic 90.
 *
 * Type definitions for AML, content moderation, and DSA compliance.
 */

// =============================================================================
// Content Moderation Types (Epic 90 - Frontend API Integration)
// =============================================================================

export type ModerationCaseStatus =
  | 'pending'
  | 'under_review'
  | 'appealed'
  | 'removed'
  | 'restricted'
  | 'warned'
  | 'approved';

export type ViolationType =
  | 'spam'
  | 'harassment'
  | 'hate_speech'
  | 'violence'
  | 'illegal_content'
  | 'misinformation'
  | 'fraud'
  | 'privacy'
  | 'inappropriate_content';

export type ContentType =
  | 'listing'
  | 'listing_photo'
  | 'review'
  | 'comment'
  | 'community_post'
  | 'message';

export interface ModerationCase {
  id: string;
  content_id: string;
  content_type: ContentType;
  content_preview: string;
  violation_type: ViolationType;
  status: ModerationCaseStatus;
  priority: number;
  reported_at: string;
  assigned_to_id?: string;
  assigned_to_name?: string;
  owner_id: string;
  owner_name: string;
  is_appeal?: boolean;
  original_case_id?: string;
  sla_deadline?: string;
  is_overdue?: boolean;
}

export interface ModerationQueueStats {
  pending_count: number;
  under_review_count: number;
  by_priority: Array<{ priority: number; count: number }>;
  by_violation_type: Array<{ type: ViolationType; count: number }>;
  avg_resolution_time_hours: number;
  overdue_count: number;
}

export interface ModerationActionTemplate {
  id: string;
  name: string;
  violation_type: ViolationType;
  action_type: string;
  rationale_template: string;
  notify_owner: boolean;
}

export interface TakeModerationActionRequest {
  action_type: 'remove' | 'restrict' | 'warn' | 'approve';
  rationale: string;
  notify_owner: boolean;
}

export interface DecideAppealRequest {
  decision: 'uphold' | 'overturn';
  rationale: string;
}

// =============================================================================
// AML Types (Story 67.1)
// =============================================================================

export type AmlAssessmentStatus =
  | 'pending'
  | 'in_progress'
  | 'completed'
  | 'requires_review'
  | 'approved'
  | 'rejected';

export type AmlRiskLevel = 'low' | 'medium' | 'high' | 'critical';

export interface AmlRiskAssessment {
  id: string;
  subject_id: string;
  subject_type: 'user' | 'organization' | 'transaction';
  subject_name: string;
  status: AmlAssessmentStatus;
  risk_level: AmlRiskLevel;
  risk_score: number;
  flagged_for_review: boolean;
  risk_factors: AmlRiskFactor[];
  // AML-check completion flags, serialized from the backend assessment row
  // (db::models::compliance::AmlRiskAssessment). The list endpoint
  // (GET /api/v1/compliance/aml/assessments) returns an untyped JSON payload
  // (serde_json::Value) with no OpenAPI contract, so these are modeled optional
  // and must be read defensively (older rows may predate the columns).
  id_verified?: boolean;
  source_of_funds_documented?: boolean;
  pep_check_completed?: boolean;
  sanctions_check_completed?: boolean;
  created_at: string;
  updated_at: string;
  reviewed_by_id?: string;
  reviewed_by_name?: string;
  reviewed_at?: string;
}

export interface AmlRiskFactor {
  factor_type: string;
  description: string;
  weight: number;
  triggered: boolean;
}

export interface AmlThresholds {
  transaction_threshold_eur: number;
  transaction_threshold_cents: number;
  cumulative_threshold_eur: number;
  review_threshold_score: number;
}

export interface CountryRisk {
  country_code: string;
  country_name: string;
  risk_rating: AmlRiskLevel;
  is_sanctioned: boolean;
  fatf_status?: string;
}

export interface InitiateEddRequest {
  assessment_id: string;
  reason: string;
  documents_requested?: string[];
}

export interface ReviewAmlAssessmentRequest {
  decision: 'approve' | 'reject' | 'escalate';
  notes: string;
}

// =============================================================================
// DSA Transparency Reports Types (Story 67.3)
// =============================================================================

export type DsaReportStatus = 'draft' | 'generated' | 'published' | 'archived';

export interface DsaTransparencyReport {
  id: string;
  period_start: string;
  period_end: string;
  status: DsaReportStatus;
  summary: DsaReportSummary;
  content_type_breakdown: Array<{ type: ContentType; count: number }>;
  violation_type_breakdown: Array<{ type: ViolationType; count: number }>;
  download_url?: string;
  generated_at: string;
  published_at?: string;
}

export interface DsaReportSummary {
  total_moderation_actions: number;
  content_removed: number;
  content_restricted: number;
  warnings_issued: number;
  user_reports_received: number;
  user_reports_resolved: number;
  avg_resolution_time_hours?: number;
  automated_decisions: number;
  automated_decisions_overturned: number;
  appeals_received: number;
  appeals_upheld: number;
  appeals_rejected: number;
}

export interface DsaMetrics {
  current_period_start: string;
  current_period_end: string;
  moderation_actions_this_period: number;
  pending_cases: number;
  avg_resolution_time_hours: number;
  sla_compliance_rate: number;
}

export interface GenerateDsaReportRequest {
  period_start: string;
  period_end: string;
}

export interface PublishDsaReportRequest {
  report_id: string;
}

// =============================================================================
// API Response Types
// =============================================================================

export interface ModerationCasesResponse {
  cases: ModerationCase[];
  total: number;
}

export interface ModerationStatsResponse {
  stats: ModerationQueueStats;
}

export interface ModerationTemplatesResponse {
  templates: ModerationActionTemplate[];
}

export interface AmlAssessmentsResponse {
  assessments: AmlRiskAssessment[];
  total: number;
}

export interface AmlThresholdsResponse {
  thresholds: AmlThresholds;
}

export interface CountryRisksResponse {
  countries: CountryRisk[];
}

export interface DsaReportsResponse {
  reports: DsaTransparencyReport[];
  total: number;
}

export interface DsaMetricsResponse {
  metrics: DsaMetrics;
}
