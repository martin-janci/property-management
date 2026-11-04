/**
 * Leases API client functions (Epic 19: Lease Management & Tenant Screening,
 * plus Epic 142 violation tracking).
 *
 * Thin typed wrappers over `/api/v1/leases/*` and `/api/v1/violations/*`.
 * Mirrors the structure of the meters module (its own `fetchApi`/`buildQueryString`
 * helpers) so the modules stay consistent.
 *
 * Every request goes through the shared authenticated transport
 * (`authenticatedFetchJson`) so it carries the bearer token — every lease and
 * violation route is behind `AuthUser` / `RlsConnection`, and a bare `fetch`
 * would be rejected with 401.
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  CreateApplicationRequest,
  CreateLeaseRequest,
  CreateLeaseTemplateRequest,
  CreateViolationAppealRequest,
  CreateViolationEvidenceRequest,
  CreateViolationRequest,
  ExpirationOverview,
  ExpiringLeasesQuery,
  InitiateScreeningRequest,
  Lease,
  LeasePayment,
  LeaseStatistics,
  LeaseTemplate,
  LeaseWithDetails,
  ListApplicationsQuery,
  ListApplicationsResponse,
  ListLeasesQuery,
  ListLeasesResponse,
  OrganizationMembersResponse,
  RecordLeasePaymentRequest,
  ReviewApplicationRequest,
  TenantApplication,
  TenantScreening,
  UpdateLeaseTemplateRequest,
  UpdateViolationRequest,
  Violation,
  ViolationAppeal,
  ViolationEvidence,
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

/** Authenticated JSON fetch (bearer token + MFA retry); throws `ApiError` on non-2xx. */
function fetchApi<T>(url: string, options?: RequestInit): Promise<T> {
  return authenticatedFetchJson<T>(url, options);
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

/** Create a lease template (`POST /api/v1/leases/templates`). */
export async function createLeaseTemplate(
  body: CreateLeaseTemplateRequest
): Promise<LeaseTemplate> {
  return fetchApi<LeaseTemplate>(`${LEASES_BASE}/templates`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/** Update a lease template, incl. (de)activation (`PUT /api/v1/leases/templates/{id}`). */
export async function updateLeaseTemplate(
  id: string,
  body: UpdateLeaseTemplateRequest
): Promise<LeaseTemplate> {
  return fetchApi<LeaseTemplate>(`${LEASES_BASE}/templates/${id}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

// ============================================================================
// Payments
// ============================================================================

/**
 * Record a payment against an existing scheduled lease payment row.
 *
 * NOTE: the backend registers this as `POST /api/v1/leases/{id}/payments`, but
 * its handler (`routes/leases.rs::record_payment`) extracts the single `{id}`
 * path segment as the **payment** id and runs `UPDATE lease_payments … WHERE
 * id = $1`. Sending the lease id there would always 500 (row not found), so the
 * client deliberately puts the payment id in that segment to match the live
 * server.
 */
export async function recordLeasePayment(
  paymentId: string,
  body: RecordLeasePaymentRequest
): Promise<LeasePayment> {
  return fetchApi<LeasePayment>(`${LEASES_BASE}/${paymentId}/payments`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
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

/** Approve or reject an application (`POST /api/v1/leases/applications/{id}/review`). Manager-only. */
export async function reviewApplication(
  id: string,
  body: ReviewApplicationRequest
): Promise<TenantApplication> {
  return fetchApi<TenantApplication>(`${LEASES_BASE}/applications/${id}/review`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/**
 * Initiate tenant screening for an application
 * (`POST /api/v1/leases/applications/{id}/screening`). Returns one row per
 * requested screening type, each in `pending_consent`.
 */
export async function initiateScreening(
  applicationId: string,
  body: InitiateScreeningRequest
): Promise<TenantScreening[]> {
  return fetchApi<TenantScreening[]>(`${LEASES_BASE}/applications/${applicationId}/screening`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

// ============================================================================
// Organization members (tenant picker for lease creation)
// ============================================================================

/** List organization members (`GET /api/v1/organizations/{id}/members`). */
export async function listOrganizationMembers(
  organizationId: string
): Promise<OrganizationMembersResponse> {
  return fetchApi<OrganizationMembersResponse>(`/api/v1/organizations/${organizationId}/members`);
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

/** Update a violation — used to resolve it (`PUT /api/v1/violations/{id}`). Manager-only. */
export async function updateViolation(
  id: string,
  body: UpdateViolationRequest
): Promise<Violation> {
  return fetchApi<Violation>(`${VIOLATIONS_BASE}/${id}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

/**
 * Dispute a violation by filing an appeal (`POST /api/v1/violations/{id}/appeals`).
 * The server flips the violation status to `disputed`.
 */
export async function createViolationAppeal(
  id: string,
  body: CreateViolationAppealRequest
): Promise<ViolationAppeal> {
  return fetchApi<ViolationAppeal>(`${VIOLATIONS_BASE}/${id}/appeals`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/** List evidence attached to a violation (`GET /api/v1/violations/{id}/evidence`). */
export async function listViolationEvidence(id: string): Promise<ViolationEvidence[]> {
  return fetchApi<ViolationEvidence[]>(`${VIOLATIONS_BASE}/${id}/evidence`);
}

/**
 * Attach an evidence record to a violation (`POST /api/v1/violations/{id}/evidence`).
 * The bytes are uploaded first (presigned `documents/upload-url`); this call
 * registers the metadata with the returned storage key.
 */
export async function addViolationEvidence(
  id: string,
  body: CreateViolationEvidenceRequest
): Promise<ViolationEvidence> {
  return fetchApi<ViolationEvidence>(`${VIOLATIONS_BASE}/${id}/evidence`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}
