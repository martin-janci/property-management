/**
 * TanStack Query hooks for the leases API (Epic 19) and violation tracking
 * (Epic 142).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  CreateApplicationRequest,
  CreateLeaseAmendmentRequest,
  CreateLeaseRequest,
  CreateLeaseTemplateRequest,
  CreateViolationAppealRequest,
  CreateViolationEvidenceRequest,
  CreateViolationRequest,
  ExpiringLeasesQuery,
  InitiateScreeningRequest,
  ListApplicationsQuery,
  ListLeasesQuery,
  RecordLeasePaymentRequest,
  RenewLeaseRequest,
  ReviewApplicationRequest,
  TerminateLeaseRequest,
  UpdateLeaseRequest,
  UpdateLeaseTemplateRequest,
  UpdateViolationRequest,
  ViolationQuery,
} from './types';

// ============================================================================
// Query keys
// ============================================================================

export const leaseKeys = {
  all: ['leases'] as const,
  lists: () => [...leaseKeys.all, 'list'] as const,
  list: (query: ListLeasesQuery) => [...leaseKeys.lists(), query] as const,
  details: () => [...leaseKeys.all, 'detail'] as const,
  detail: (id: string) => [...leaseKeys.details(), id] as const,
  statistics: (organizationId: string) => [...leaseKeys.all, 'statistics', organizationId] as const,
  expiring: (query: ExpiringLeasesQuery) => [...leaseKeys.all, 'expiring', query] as const,
  templates: (organizationId: string) => [...leaseKeys.all, 'templates', organizationId] as const,
  template: (id: string) => [...leaseKeys.all, 'template', id] as const,
  applicationLists: () => [...leaseKeys.all, 'application-list'] as const,
  applicationList: (query: ListApplicationsQuery) =>
    [...leaseKeys.applicationLists(), query] as const,
  application: (id: string) => [...leaseKeys.all, 'application', id] as const,
  organizationMembers: (organizationId: string) =>
    [...leaseKeys.all, 'organization-members', organizationId] as const,
};

export const violationKeys = {
  all: ['violations'] as const,
  lists: () => [...violationKeys.all, 'list'] as const,
  list: (query: ViolationQuery) => [...violationKeys.lists(), query] as const,
  details: () => [...violationKeys.all, 'detail'] as const,
  detail: (id: string) => [...violationKeys.details(), id] as const,
  evidence: (id: string) => [...violationKeys.all, 'evidence', id] as const,
};

// ============================================================================
// Lease queries
// ============================================================================

/** List leases for an organization. */
export function useLeases(query: ListLeasesQuery | undefined) {
  return useQuery({
    queryKey: leaseKeys.list(query ?? { organization_id: '' }),
    queryFn: () => api.listLeases(query as ListLeasesQuery),
    enabled: !!query?.organization_id,
  });
}

/** Get a lease with its details. */
export function useLease(id: string | undefined) {
  return useQuery({
    queryKey: leaseKeys.detail(id ?? ''),
    queryFn: () => api.getLease(id as string),
    enabled: !!id,
  });
}

/** Lease statistics for the dashboard. */
export function useLeaseStatistics(organizationId: string | undefined) {
  return useQuery({
    queryKey: leaseKeys.statistics(organizationId ?? ''),
    queryFn: () => api.getLeaseStatistics(organizationId as string),
    enabled: !!organizationId,
  });
}

/** Expiring-leases overview. */
export function useExpiringLeases(query: ExpiringLeasesQuery | undefined) {
  return useQuery({
    queryKey: leaseKeys.expiring(query ?? { organization_id: '' }),
    queryFn: () => api.getExpiringLeases(query as ExpiringLeasesQuery),
    enabled: !!query?.organization_id,
  });
}

/** List lease templates for an organization. */
export function useLeaseTemplates(organizationId: string | undefined) {
  return useQuery({
    queryKey: leaseKeys.templates(organizationId ?? ''),
    queryFn: () => api.listLeaseTemplates(organizationId as string),
    enabled: !!organizationId,
  });
}

/** Get a single lease template. */
export function useLeaseTemplate(id: string | undefined) {
  return useQuery({
    queryKey: leaseKeys.template(id ?? ''),
    queryFn: () => api.getLeaseTemplate(id as string),
    enabled: !!id,
  });
}

// ============================================================================
// Application queries
// ============================================================================

/** List tenant applications for an organization. */
export function useApplications(query: ListApplicationsQuery | undefined) {
  return useQuery({
    queryKey: leaseKeys.applicationList(query ?? { organization_id: '' }),
    queryFn: () => api.listApplications(query as ListApplicationsQuery),
    enabled: !!query?.organization_id,
  });
}

/** Get a single tenant application. */
export function useApplication(id: string | undefined) {
  return useQuery({
    queryKey: leaseKeys.application(id ?? ''),
    queryFn: () => api.getApplication(id as string),
    enabled: !!id,
  });
}

// ============================================================================
// Lease mutations
// ============================================================================

/** Create a lease; invalidates the leases caches. */
export function useCreateLease() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateLeaseRequest) => api.createLease(body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: leaseKeys.all });
    },
  });
}

/**
 * Invalidate one lease's detail plus every list / statistics / expiring view
 * after a lease mutation.
 */
function invalidateLease(queryClient: ReturnType<typeof useQueryClient>, id: string) {
  queryClient.invalidateQueries({ queryKey: leaseKeys.detail(id) });
  queryClient.invalidateQueries({ queryKey: leaseKeys.lists() });
  queryClient.invalidateQueries({ queryKey: [...leaseKeys.all, 'statistics'] });
  queryClient.invalidateQueries({ queryKey: [...leaseKeys.all, 'expiring'] });
}

/** Update a lease's editable fields; invalidates the lease and lists. */
export function useUpdateLease() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateLeaseRequest }) =>
      api.updateLease(id, body),
    onSuccess: (_data, { id }) => invalidateLease(queryClient, id),
  });
}

/** Terminate a lease; invalidates the lease and lists. */
export function useTerminateLease() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: TerminateLeaseRequest }) =>
      api.terminateLease(id, body),
    onSuccess: (_data, { id }) => invalidateLease(queryClient, id),
  });
}

/** Renew a lease (creates a new draft lease); invalidates the old lease and lists. */
export function useRenewLease() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: RenewLeaseRequest }) => api.renewLease(id, body),
    onSuccess: (_data, { id }) => invalidateLease(queryClient, id),
  });
}

/** Add a lease amendment; invalidates the lease detail (amendments are embedded). */
export function useCreateLeaseAmendment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: CreateLeaseAmendmentRequest }) =>
      api.createLeaseAmendment(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: leaseKeys.detail(id) });
    },
  });
}

/** Create a tenant application; invalidates the application caches. */
export function useCreateApplication() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateApplicationRequest) => api.createApplication(body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: leaseKeys.applicationLists() });
    },
  });
}

/** Create a lease template; invalidates every template list. */
export function useCreateLeaseTemplate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateLeaseTemplateRequest) => api.createLeaseTemplate(body),
    onSuccess: (_data, body) => {
      queryClient.invalidateQueries({ queryKey: leaseKeys.templates(body.organization_id) });
    },
  });
}

/** Update (or (de)activate) a lease template; invalidates the list and the entity. */
export function useUpdateLeaseTemplate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateLeaseTemplateRequest }) =>
      api.updateLeaseTemplate(id, body),
    onSuccess: (_data, { id, body }) => {
      queryClient.invalidateQueries({ queryKey: leaseKeys.templates(body.organization_id) });
      queryClient.invalidateQueries({ queryKey: leaseKeys.template(id) });
    },
  });
}

/**
 * Record a payment against a scheduled lease payment via
 * `POST /leases/{leaseId}/payments/{paymentId}`. Invalidates the owning lease
 * detail plus lists/statistics.
 */
export function useRecordLeasePayment() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      leaseId,
      paymentId,
      body,
    }: {
      leaseId: string;
      paymentId: string;
      body: RecordLeasePaymentRequest;
    }) => api.recordLeasePayment(leaseId, paymentId, body),
    onSuccess: (payment, { leaseId }) => {
      queryClient.invalidateQueries({
        queryKey: leaseKeys.detail(leaseId ?? payment.lease_id),
      });
      queryClient.invalidateQueries({ queryKey: leaseKeys.lists() });
      queryClient.invalidateQueries({ queryKey: [...leaseKeys.all, 'statistics'] });
    },
  });
}

/** Approve/reject an application; invalidates the application and lists. */
export function useReviewApplication() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: ReviewApplicationRequest }) =>
      api.reviewApplication(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: leaseKeys.application(id) });
      queryClient.invalidateQueries({ queryKey: leaseKeys.applicationLists() });
      queryClient.invalidateQueries({ queryKey: [...leaseKeys.all, 'statistics'] });
    },
  });
}

/** Initiate screening for an application; invalidates the application and lists. */
export function useInitiateScreening() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: InitiateScreeningRequest }) =>
      api.initiateScreening(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: leaseKeys.application(id) });
      queryClient.invalidateQueries({ queryKey: leaseKeys.applicationLists() });
    },
  });
}

/** Organization members (used as the lease tenant picker source). */
export function useOrganizationMembers(organizationId: string | undefined) {
  return useQuery({
    queryKey: leaseKeys.organizationMembers(organizationId ?? ''),
    queryFn: () => api.listOrganizationMembers(organizationId as string),
    enabled: !!organizationId,
    staleTime: 60_000,
  });
}

// ============================================================================
// Violation queries / mutations
// ============================================================================

/** List violations for the authenticated organization. */
export function useViolations(query: ViolationQuery = {}) {
  return useQuery({
    queryKey: violationKeys.list(query),
    queryFn: () => api.listViolations(query),
  });
}

/** Get a single violation. */
export function useViolation(id: string | undefined) {
  return useQuery({
    queryKey: violationKeys.detail(id ?? ''),
    queryFn: () => api.getViolation(id as string),
    enabled: !!id,
  });
}

/** Create a violation; invalidates the violation list caches. */
export function useCreateViolation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (body: CreateViolationRequest) => api.createViolation(body),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: violationKeys.lists() });
    },
  });
}

/** Update a violation (resolve); invalidates the violation detail and lists. */
export function useUpdateViolation() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: UpdateViolationRequest }) =>
      api.updateViolation(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: violationKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: violationKeys.lists() });
    },
  });
}

/** Dispute a violation via an appeal; invalidates the violation detail and lists. */
export function useCreateViolationAppeal() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: CreateViolationAppealRequest }) =>
      api.createViolationAppeal(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: violationKeys.detail(id) });
      queryClient.invalidateQueries({ queryKey: violationKeys.lists() });
    },
  });
}

/** Evidence attached to a violation. */
export function useViolationEvidence(id: string | undefined) {
  return useQuery({
    queryKey: violationKeys.evidence(id ?? ''),
    queryFn: () => api.listViolationEvidence(id as string),
    enabled: !!id,
  });
}

/** Register an evidence record on a violation; invalidates its evidence list. */
export function useAddViolationEvidence() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, body }: { id: string; body: CreateViolationEvidenceRequest }) =>
      api.addViolationEvidence(id, body),
    onSuccess: (_data, { id }) => {
      queryClient.invalidateQueries({ queryKey: violationKeys.evidence(id) });
    },
  });
}
