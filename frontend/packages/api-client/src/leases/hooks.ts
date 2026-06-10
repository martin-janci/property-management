/**
 * TanStack Query hooks for the leases API (Epic 19) and violation tracking
 * (Epic 142).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  CreateApplicationRequest,
  CreateLeaseRequest,
  CreateViolationRequest,
  ExpiringLeasesQuery,
  ListApplicationsQuery,
  ListLeasesQuery,
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
};

export const violationKeys = {
  all: ['violations'] as const,
  lists: () => [...violationKeys.all, 'list'] as const,
  list: (query: ViolationQuery) => [...violationKeys.lists(), query] as const,
  details: () => [...violationKeys.all, 'detail'] as const,
  detail: (id: string) => [...violationKeys.details(), id] as const,
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
