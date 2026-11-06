/**
 * TanStack Query hooks for organization members + unit residents.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type { AddOrgMemberRequest, AddUnitResidentRequest, UpdateOrgMemberRequest } from './types';

export const membersKeys = {
  all: ['org-members'] as const,
  list: (orgId: string) => [...membersKeys.all, 'list', orgId] as const,
  roles: (orgId: string) => [...membersKeys.all, 'roles', orgId] as const,
  residents: (buildingId: string, unitId: string) =>
    ['unit-residents', buildingId, unitId] as const,
};

export function useOrgMembers(orgId: string | undefined) {
  return useQuery({
    queryKey: membersKeys.list(orgId ?? ''),
    queryFn: () => api.listOrgMembers(orgId as string),
    enabled: !!orgId,
  });
}

export function useOrgRoles(orgId: string | undefined) {
  return useQuery({
    queryKey: membersKeys.roles(orgId ?? ''),
    queryFn: () => api.listOrgRoles(orgId as string),
    enabled: !!orgId,
  });
}

export function useAddOrgMember(orgId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: AddOrgMemberRequest) => api.addOrgMember(orgId, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: membersKeys.list(orgId) }),
  });
}

export function useUpdateOrgMember(orgId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ userId, body }: { userId: string; body: UpdateOrgMemberRequest }) =>
      api.updateOrgMember(orgId, userId, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: membersKeys.list(orgId) }),
  });
}

export function useRemoveOrgMember(orgId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (userId: string) => api.removeOrgMember(orgId, userId),
    onSuccess: () => qc.invalidateQueries({ queryKey: membersKeys.list(orgId) }),
  });
}

export function useUnitResidents(buildingId: string, unitId: string | undefined) {
  return useQuery({
    queryKey: membersKeys.residents(buildingId, unitId ?? ''),
    queryFn: () => api.listUnitResidents(buildingId, unitId as string),
    enabled: !!buildingId && !!unitId,
  });
}

export function useAddUnitResident(buildingId: string, unitId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (body: AddUnitResidentRequest) => api.addUnitResident(buildingId, unitId, body),
    onSuccess: () => qc.invalidateQueries({ queryKey: membersKeys.residents(buildingId, unitId) }),
  });
}

export function useEndUnitResidency(buildingId: string, unitId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ residentId, endDate }: { residentId: string; endDate: string }) =>
      api.endUnitResidency(buildingId, unitId, residentId, endDate),
    onSuccess: () => qc.invalidateQueries({ queryKey: membersKeys.residents(buildingId, unitId) }),
  });
}

export function useRemoveUnitResident(buildingId: string, unitId: string) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (residentId: string) => api.removeUnitResident(buildingId, unitId, residentId),
    onSuccess: () => qc.invalidateQueries({ queryKey: membersKeys.residents(buildingId, unitId) }),
  });
}
