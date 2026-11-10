/**
 * Organization members + unit residents API client.
 *
 *   GET    /api/v1/organizations/{id}/members
 *   POST   /api/v1/organizations/{id}/members            (by user_id or email)
 *   PUT    /api/v1/organizations/{id}/members/{user_id}  (change role)
 *   DELETE /api/v1/organizations/{id}/members/{user_id}
 *   GET    /api/v1/organizations/{id}/roles
 *   GET    /api/v1/buildings/{b}/units/{u}/residents
 *   POST   /api/v1/buildings/{b}/units/{u}/residents
 *   POST   /api/v1/buildings/{b}/units/{u}/residents/{id}/end
 *   DELETE /api/v1/buildings/{b}/units/{u}/residents/{id}
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  AddOrgMemberRequest,
  AddUnitResidentRequest,
  OrgMembersResponse,
  OrgRolesResponse,
  UnitResident,
  UpdateOrgMemberRequest,
} from './types';

const orgBase = (orgId: string) => `/api/v1/organizations/${encodeURIComponent(orgId)}`;
const residentsBase = (buildingId: string, unitId: string) =>
  `/api/v1/buildings/${encodeURIComponent(buildingId)}/units/${encodeURIComponent(unitId)}/residents`;

export function listOrgMembers(orgId: string): Promise<OrgMembersResponse> {
  return authenticatedFetchJson<OrgMembersResponse>(`${orgBase(orgId)}/members`);
}

export function addOrgMember(
  orgId: string,
  body: AddOrgMemberRequest
): Promise<{ message: string }> {
  return authenticatedFetchJson(`${orgBase(orgId)}/members`, {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

export function updateOrgMember(
  orgId: string,
  userId: string,
  body: UpdateOrgMemberRequest
): Promise<{ message: string }> {
  return authenticatedFetchJson(`${orgBase(orgId)}/members/${encodeURIComponent(userId)}`, {
    method: 'PUT',
    body: JSON.stringify(body),
  });
}

export function removeOrgMember(orgId: string, userId: string): Promise<{ message: string }> {
  return authenticatedFetchJson(`${orgBase(orgId)}/members/${encodeURIComponent(userId)}`, {
    method: 'DELETE',
  });
}

export function listOrgRoles(orgId: string): Promise<OrgRolesResponse> {
  return authenticatedFetchJson<OrgRolesResponse>(`${orgBase(orgId)}/roles`);
}

export function listUnitResidents(buildingId: string, unitId: string): Promise<UnitResident[]> {
  return authenticatedFetchJson<UnitResident[]>(residentsBase(buildingId, unitId));
}

export function addUnitResident(
  buildingId: string,
  unitId: string,
  body: AddUnitResidentRequest
): Promise<unknown> {
  return authenticatedFetchJson(residentsBase(buildingId, unitId), {
    method: 'POST',
    body: JSON.stringify(body),
  });
}

/** End a residency (keeps history). `endDate` is YYYY-MM-DD. */
export function endUnitResidency(
  buildingId: string,
  unitId: string,
  residentId: string,
  endDate: string
): Promise<unknown> {
  return authenticatedFetchJson(
    `${residentsBase(buildingId, unitId)}/${encodeURIComponent(residentId)}/end`,
    { method: 'POST', body: JSON.stringify({ end_date: endDate }) }
  );
}

export function removeUnitResident(
  buildingId: string,
  unitId: string,
  residentId: string
): Promise<unknown> {
  return authenticatedFetchJson(
    `${residentsBase(buildingId, unitId)}/${encodeURIComponent(residentId)}`,
    { method: 'DELETE' }
  );
}
