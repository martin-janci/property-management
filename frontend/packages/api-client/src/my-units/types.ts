/**
 * My Unit (resident-facing) API types — Epic 3, Story 3.6.
 *
 * Mirrors the backend `MyUnitResponse` shape served by
 * `GET /api/v1/users/me/units` (camelCase, matching the rest of the client).
 * The payload is deliberately PII-free: it carries only the caller's own
 * association plus non-personal unit/building detail. There is no field for
 * other residents or owners.
 */

/** The caller's own association (residency) with a unit. */
export interface MyUnitAssociation {
  residentId: string;
  /** owner | tenant | family_member | subtenant */
  residentType: string;
  residentTypeDisplay: string;
  isPrimary: boolean;
  startDate: string;
  endDate: string | null;
  isActive: boolean;
  receivesNotifications: boolean;
  receivesMail: boolean;
}

/** The unit's own (non-PII) details. */
export interface MyUnitDetails {
  id: string;
  buildingId: string;
  entrance: string | null;
  designation: string;
  floor: number;
  floorDisplay: string;
  unitType: string;
  sizeSqm: string | null;
  rooms: number | null;
  ownershipShare: string;
  occupancyStatus: string;
  description: string | null;
  status: string;
}

/** Minimal building address context. */
export interface MyUnitBuilding {
  id: string;
  name: string | null;
  street: string;
  city: string;
  postalCode: string;
}

/** One "My Unit" entry. */
export interface MyUnit {
  association: MyUnitAssociation;
  unit: MyUnitDetails;
  building: MyUnitBuilding;
}
