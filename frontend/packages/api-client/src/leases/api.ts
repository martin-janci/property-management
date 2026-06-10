/**
 * Leases API client functions (Epic 19: Lease Management & Tenant Screening,
 * plus Epic 142 violation tracking).
 *
 * Thin typed `fetch` wrappers over `/api/v1/leases/*` and `/api/v1/violations/*`.
 * Mirrors the structure of the meters module (its own `fetchApi`/`buildQueryString`
 * helpers) so the modules stay consistent.
 */

import type {
  CreateApplicationRequest,
  CreateLeaseRequest,
  CreateViolationRequest,
  ExpirationOverview,
  ExpiringLeasesQuery,
  Lease,
  LeaseStatistics,
  LeaseTemplate,
  LeaseWithDetails,
  ListApplicationsQuery,
  ListApplicationsResponse,
  ListLeasesQuery,
  ListLeasesResponse,
  TenantApplication,
  Violation,
  ViolationQuery,
  ViolationSummary,
} from './types';

const LEASES_BASE = '/api/v1/leases';
const VIOLATIONS_BASE = '/api/v1/violations';

/** Build a query string from params, skipping empty values. */
function buildQueryString(
  params: Record<string, string | number | boolean | undefined | null>
): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null && value !== '') {
      searchParams.append(key, String(value));
    }
  }
  const query = searchParams.toString();
  return query ? `?${query}` : '';
}

/** Generic fetch wrapper with error handling. */
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
// Leases
// ============================================================================

/** List leases for an organization (paginated). */
export async function listLeases(query: ListLeasesQuery): Promise<ListLeasesResponse> {
  const qs = buildQueryString({
    organization_id: query.organization_id,
    unit_id: query.unit_id,
    tenant_id: query.tenant_id,
    status: query.status,
    expiring_within_days: query.expiring_within_days,
    limit: query.limit,
    offset: query.offset,
  });
  return fetchApi<ListLeasesResponse>(`${LEASES_BASE}${qs}`);
}

/** Get a single lease with its amendments, upcoming payments and reminders. */
export async function getLease(id: string): Promise<LeaseWithDetails> {
  return fetchApi<LeaseWithDetails>(`${LEASES_BASE}/${id}`);
}

/** Create a lease. */
export async function createLease(body: CreateLeaseRequest): Promise<Lease> {
  return fetchApi<Lease>(`${LEASES_BASE}`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/** Lease statistics for the dashboard (requires an organization scope). */
export async function getLeaseStatistics(organizationId: string): Promise<LeaseStatistics> {
  const qs = buildQueryString({ organization_id: organizationId });
  return fetchApi<LeaseStatistics>(`${LEASES_BASE}/statistics${qs}`);
}

/** Expiring-leases overview (requires an organization scope). */
export async function getExpiringLeases(query: ExpiringLeasesQuery): Promise<ExpirationOverview> {
  const qs = buildQueryString({
    organization_id: query.organization_id,
    days: query.days,
  });
  return fetchApi<ExpirationOverview>(`${LEASES_BASE}/expiring${qs}`);
}

// ============================================================================
// Lease templates
// ============================================================================

/** List lease templates for an organization. */
export async function listLeaseTemplates(organizationId: string): Promise<LeaseTemplate[]> {
  const qs = buildQueryString({ organization_id: organizationId });
  return fetchApi<LeaseTemplate[]>(`${LEASES_BASE}/templates${qs}`);
}

/** Get a single lease template. */
export async function getLeaseTemplate(id: string): Promise<LeaseTemplate> {
  return fetchApi<LeaseTemplate>(`${LEASES_BASE}/templates/${id}`);
}

// ============================================================================
// Applications
// ============================================================================

/** List tenant applications for an organization (paginated). */
export async function listApplications(
  query: ListApplicationsQuery
): Promise<ListApplicationsResponse> {
  const qs = buildQueryString({
    organization_id: query.organization_id,
    unit_id: query.unit_id,
    status: query.status,
    limit: query.limit,
    offset: query.offset,
  });
  return fetchApi<ListApplicationsResponse>(`${LEASES_BASE}/applications${qs}`);
}

/** Get a single tenant application. */
export async function getApplication(id: string): Promise<TenantApplication> {
  return fetchApi<TenantApplication>(`${LEASES_BASE}/applications/${id}`);
}

/** Create a tenant application. */
export async function createApplication(
  body: CreateApplicationRequest
): Promise<TenantApplication> {
  return fetchApi<TenantApplication>(`${LEASES_BASE}/applications`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

// ============================================================================
// Violations
// ============================================================================

/** List violations for the authenticated organization. */
export async function listViolations(query: ViolationQuery = {}): Promise<ViolationSummary[]> {
  const qs = buildQueryString({
    building_id: query.building_id,
    unit_id: query.unit_id,
    category: query.category,
    severity: query.severity,
    status: query.status,
    violator_id: query.violator_id,
    assigned_to: query.assigned_to,
    from_date: query.from_date,
    to_date: query.to_date,
    limit: query.limit,
    offset: query.offset,
  });
  return fetchApi<ViolationSummary[]>(`${VIOLATIONS_BASE}${qs}`);
}

/** Get a single violation. */
export async function getViolation(id: string): Promise<Violation> {
  return fetchApi<Violation>(`${VIOLATIONS_BASE}/${id}`);
}

/** Create a violation. */
export async function createViolation(body: CreateViolationRequest): Promise<Violation> {
  return fetchApi<Violation>(`${VIOLATIONS_BASE}`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}
