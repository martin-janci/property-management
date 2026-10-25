/**
 * Compliance API Client (Epic 90 - Frontend API Integration).
 * Originally from Epic 67, integrated as part of Epic 90.
 *
 * API functions for AML, content moderation, and DSA compliance.
 */

import type {
  AmlAssessmentsResponse,
  AmlThresholdsResponse,
  CountryRisksResponse,
  DecideAppealRequest,
  DsaMetricsResponse,
  DsaReportsResponse,
  DsaTransparencyReport,
  GenerateDsaReportRequest,
  InitiateEddRequest,
  ModerationCase,
  ModerationCasesResponse,
  ModerationStatsResponse,
  ModerationTemplatesResponse,
  ReviewAmlAssessmentRequest,
  TakeModerationActionRequest,
} from './types';

// The AML, DSA, and content-moderation handlers all live on a single router
// mounted at `/api/v1/aml-dsa` (`routes::aml_dsa::router()` in
// `backend/servers/api-server/src/routes/aml_dsa/mod.rs`). The
// `/api/v1/compliance` router (`routes::compliance::router()`) serves only
// audit-logs / gdpr / security and has NO `/moderation/*` routes, so routing
// moderation calls there 404'd — see #3149.
const AML_DSA_BASE = '/api/v1/aml-dsa';

async function fetchApi<T>(url: string, options: RequestInit = {}): Promise<T> {
  // TODO(Phase-1): Add authentication headers to fetchApi
  // Will be implemented when auth context is available
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    const statusInfo = `HTTP ${response.status}${response.statusText ? ` ${response.statusText}` : ''}`;
    const errorMessage = (error as { message?: string }).message;
    const message =
      (typeof errorMessage === 'string' && errorMessage.trim()) || statusInfo || 'Request failed';
    throw new Error(message);
  }

  return response.json();
}

// =============================================================================
// Content Moderation API
// =============================================================================

export async function listModerationCases(params?: {
  status?: string;
  content_type?: string;
  violation_type?: string;
  priority?: number;
  unassigned_only?: boolean;
  overdue?: boolean;
  limit?: number;
  offset?: number;
}): Promise<ModerationCasesResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set('status', params.status);
  if (params?.content_type) searchParams.set('content_type', params.content_type);
  if (params?.violation_type) searchParams.set('violation_type', params.violation_type);
  if (params?.priority) searchParams.set('priority', params.priority.toString());
  if (params?.unassigned_only) searchParams.set('unassigned_only', 'true');
  if (params?.overdue) searchParams.set('overdue', 'true');
  if (params?.limit) searchParams.set('limit', params.limit.toString());
  if (params?.offset) searchParams.set('offset', params.offset.toString());

  const queryString = searchParams.toString();
  // The queue list is served at `/moderation/queue` (not `/moderation/cases`,
  // which is the per-case detail prefix) — see
  // `backend/servers/api-server/src/routes/aml_dsa/mod.rs` (#3157).
  return fetchApi(`${AML_DSA_BASE}/moderation/queue${queryString ? `?${queryString}` : ''}`);
}

export async function getModerationCase(id: string): Promise<{ case_: ModerationCase }> {
  return fetchApi(`${AML_DSA_BASE}/moderation/cases/${id}`);
}

export async function getModerationStats(): Promise<ModerationStatsResponse> {
  // Queue stats live under the queue path: `/moderation/queue/stats` (#3157).
  return fetchApi(`${AML_DSA_BASE}/moderation/queue/stats`);
}

export async function getModerationTemplates(): Promise<ModerationTemplatesResponse> {
  return fetchApi(`${AML_DSA_BASE}/moderation/templates`);
}

export async function assignModerationCase(caseId: string): Promise<{ case_: ModerationCase }> {
  return fetchApi(`${AML_DSA_BASE}/moderation/cases/${caseId}/assign`, { method: 'POST' });
}

export async function unassignModerationCase(caseId: string): Promise<{ case_: ModerationCase }> {
  return fetchApi(`${AML_DSA_BASE}/moderation/cases/${caseId}/unassign`, { method: 'POST' });
}

export async function takeModerationAction(
  caseId: string,
  request: TakeModerationActionRequest
): Promise<{ case_: ModerationCase }> {
  return fetchApi(`${AML_DSA_BASE}/moderation/cases/${caseId}/action`, {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

export async function decideModerationAppeal(
  caseId: string,
  request: DecideAppealRequest
): Promise<{ case_: ModerationCase }> {
  return fetchApi(`${AML_DSA_BASE}/moderation/cases/${caseId}/appeal/decide`, {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

// =============================================================================
// AML API
// =============================================================================

export async function listAmlAssessments(params?: {
  status?: string;
  risk_level?: string;
  flagged_only?: boolean;
  limit?: number;
  offset?: number;
}): Promise<AmlAssessmentsResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set('status', params.status);
  if (params?.risk_level) searchParams.set('risk_level', params.risk_level);
  if (params?.flagged_only) searchParams.set('flagged_only', 'true');
  if (params?.limit) searchParams.set('limit', params.limit.toString());
  if (params?.offset) searchParams.set('offset', params.offset.toString());

  const queryString = searchParams.toString();
  return fetchApi(`${AML_DSA_BASE}/aml/assessments${queryString ? `?${queryString}` : ''}`);
}

export async function getAmlThresholds(): Promise<AmlThresholdsResponse> {
  return fetchApi(`${AML_DSA_BASE}/aml/thresholds`);
}

export async function getCountryRisks(): Promise<CountryRisksResponse> {
  return fetchApi(`${AML_DSA_BASE}/aml/country-risks`);
}

export async function initiateEdd(request: InitiateEddRequest): Promise<{ message: string }> {
  return fetchApi(`${AML_DSA_BASE}/aml/assessments/${request.assessment_id}/edd`, {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

export async function reviewAmlAssessment(
  assessmentId: string,
  request: ReviewAmlAssessmentRequest
): Promise<{ message: string }> {
  return fetchApi(`${AML_DSA_BASE}/aml/assessments/${assessmentId}/review`, {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

// =============================================================================
// DSA Transparency Reports API
// =============================================================================

export async function listDsaReports(params?: {
  status?: string;
  limit?: number;
  offset?: number;
}): Promise<DsaReportsResponse> {
  const searchParams = new URLSearchParams();
  if (params?.status) searchParams.set('status', params.status);
  if (params?.limit) searchParams.set('limit', params.limit.toString());
  if (params?.offset) searchParams.set('offset', params.offset.toString());

  const queryString = searchParams.toString();
  return fetchApi(`${AML_DSA_BASE}/dsa/reports${queryString ? `?${queryString}` : ''}`);
}

export async function getDsaMetrics(): Promise<DsaMetricsResponse> {
  return fetchApi(`${AML_DSA_BASE}/dsa/metrics`);
}

export async function generateDsaReport(
  request: GenerateDsaReportRequest
): Promise<{ report: DsaTransparencyReport }> {
  return fetchApi(`${AML_DSA_BASE}/dsa/reports/generate`, {
    method: 'POST',
    body: JSON.stringify(request),
  });
}

export async function publishDsaReport(
  reportId: string
): Promise<{ report: DsaTransparencyReport }> {
  return fetchApi(`${AML_DSA_BASE}/dsa/reports/${reportId}/publish`, { method: 'POST' });
}

export async function downloadDsaReportPdf(
  reportId: string
): Promise<{ url: string; filename: string }> {
  return fetchApi(`${AML_DSA_BASE}/dsa/reports/${reportId}/download`);
}
