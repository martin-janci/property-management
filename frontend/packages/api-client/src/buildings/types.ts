/**
 * Buildings Types
 *
 * Type definitions for the Buildings API (UC-15).
 */

/** Building type */
export type BuildingType = 'residential' | 'commercial' | 'mixed' | 'industrial';

/** Building status */
export type BuildingStatus = 'active' | 'under_construction' | 'renovation' | 'inactive';

/** Common area type */
export type CommonAreaType =
  | 'staircase'
  | 'elevator'
  | 'lobby'
  | 'hallway'
  | 'basement'
  | 'attic'
  | 'parking'
  | 'garage'
  | 'garden'
  | 'playground'
  | 'pool'
  | 'gym'
  | 'laundry_room'
  | 'storage_room'
  | 'other';

/** Building document category */
export type BuildingDocumentCategory =
  | 'legal_document'
  | 'insurance'
  | 'maintenance'
  | 'financial_report'
  | 'meeting_minutes'
  | 'contract'
  | 'technical_document'
  | 'other';

/** Document visibility */
export type DocumentVisibility = 'public' | 'owners_only' | 'managers_only';

/** GPS location */
export interface GeoLocation {
  latitude: number;
  longitude: number;
}

/** Address */
export interface Address {
  street: string;
  city: string;
  state?: string;
  postalCode: string;
  country: string;
}

/** Building */
export interface Building {
  id: string;
  organizationId: string;
  name: string;
  address: Address;
  location?: GeoLocation;
  type: BuildingType;
  floorCount: number;
  unitCount: number;
  yearBuilt?: number;
  totalAreaM2?: number;
  photoUrl?: string;
  description?: string;
  status: BuildingStatus;
  managerId?: string;
  technicalManagerId?: string;
  createdAt: string;
  updatedAt: string;
}

/** Building summary for list views */
export interface BuildingSummary {
  id: string;
  name: string;
  address: Address;
  type: BuildingType;
  status: BuildingStatus;
  unitCount: number;
  floorCount: number;
  photoUrl?: string;
}

/** Floor in a building */
export interface Floor {
  id: string;
  buildingId: string;
  number: number;
  name?: string;
  floorPlanUrl?: string;
  unitCount: number;
}

/** Common area */
export interface CommonArea {
  id: string;
  buildingId: string;
  name: string;
  type: CommonAreaType;
  description?: string;
  areaM2?: number;
  floorId?: string;
  photoUrl?: string;
}

/** File attachment */
export interface Attachment {
  id: string;
  fileKey: string;
  fileName: string;
  fileType: string;
  fileSize: number;
  url: string;
}

/** Building document */
export interface BuildingDocument {
  id: string;
  buildingId: string;
  title: string;
  category: BuildingDocumentCategory;
  file: Attachment;
  description?: string;
  visibility: DocumentVisibility;
  createdAt: string;
  updatedAt: string;
}

/** Create building request */
export interface CreateBuildingRequest {
  name: string;
  address: Address;
  location?: GeoLocation;
  type: BuildingType;
  floorCount: number;
  yearBuilt?: number;
  totalAreaM2?: number;
  description?: string;
  managerId?: string;
  technicalManagerId?: string;
}

/** Update building request */
export interface UpdateBuildingRequest {
  name?: string;
  address?: Address;
  location?: GeoLocation;
  type?: BuildingType;
  floorCount?: number;
  yearBuilt?: number;
  totalAreaM2?: number;
  description?: string;
  status?: BuildingStatus;
  managerId?: string;
  technicalManagerId?: string;
}

/** Create floor request */
export interface CreateFloorRequest {
  number: number;
  name?: string;
}

/** Create common area request */
export interface CreateCommonAreaRequest {
  name: string;
  type: CommonAreaType;
  description?: string;
  areaM2?: number;
  floorId?: string;
}

/** Upload building document request */
export interface UploadDocumentRequest {
  title: string;
  category: BuildingDocumentCategory;
  description?: string;
  visibility: DocumentVisibility;
  fileContent: string;
  fileName: string;
  mimeType: string;
}

/** List buildings query parameters */
export interface ListBuildingsParams {
  page?: number;
  pageSize?: number;
  status?: BuildingStatus;
  type?: BuildingType;
}

/** List building documents query parameters */
export interface ListBuildingDocumentsParams {
  page?: number;
  pageSize?: number;
  category?: BuildingDocumentCategory;
}

/** Paginated response for buildings */
export interface BuildingsPaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// ============================================
// Units (Story 3.2 — Unit management UI)
// ============================================
//
// NOTE: Unlike the legacy camelCase `Building`/`Floor`/`CommonArea` types above
// (Story 81.2), the unit types below use **snake_case** field names to match the
// JSON the Rust api-server actually serializes. The backend uses serde defaults
// (no `rename_all`), so the wire format is snake_case and `rust_decimal` is built
// with the `serde-str` feature — decimals (`size_sqm`, `ownership_share`) cross
// the wire as strings. These property names match the JSON on the wire, the same
// convention documented in the `disputes` and `voting` api-client modules.

/** Unit type (matches DB CHECK constraint on `units.unit_type`). */
export type UnitType = 'apartment' | 'commercial' | 'parking' | 'storage' | 'other';

/** Occupancy status (matches DB CHECK constraint on `units.occupancy_status`). */
export type UnitOccupancyStatus = 'owner_occupied' | 'rented' | 'vacant' | 'unknown';

/** Unit lifecycle status (matches DB CHECK constraint on `units.status`). */
export type UnitStatus = 'active' | 'archived';

/** Slim unit shape returned by the list endpoint (`UnitSummary`). */
export interface UnitSummary {
  id: string;
  building_id: string;
  designation: string;
  /** Floor number: 0 = ground, negative = basement. */
  floor: number;
  unit_type: UnitType;
  occupancy_status: UnitOccupancyStatus;
  status: UnitStatus;
}

/** Full unit shape returned by get/create/update endpoints (`UnitResponse`). */
export interface Unit {
  id: string;
  building_id: string;
  entrance: string | null;
  designation: string;
  /** Floor number: 0 = ground, negative = basement. */
  floor: number;
  /** Pre-formatted floor label, e.g. "G", "B1", "3". */
  floor_display: string;
  unit_type: UnitType;
  /** Pre-formatted unit-type label, e.g. "Apartment". */
  unit_type_display: string;
  /** Decimal serialized as a string (e.g. "72.50"); null when unset. */
  size_sqm: string | null;
  rooms: number | null;
  /** Decimal serialized as a string (e.g. "100.00"). */
  ownership_share: string;
  occupancy_status: UnitOccupancyStatus;
  description: string | null;
  notes: string | null;
  status: UnitStatus;
  created_at: string;
  updated_at: string;
}

/** Paginated units response (`UnitsListResponse`). */
export interface UnitsListResponse {
  units: UnitSummary[];
  total: number;
  offset: number;
  limit: number;
}

/** List units query parameters. */
export interface ListUnitsParams {
  offset?: number;
  limit?: number;
  /** Include archived units in the result (defaults to false server-side). */
  include_archived?: boolean;
  unit_type?: UnitType;
  floor?: number;
}

/** Create unit request body (`CreateUnitRequest`). */
export interface CreateUnitRequest {
  /** Unit designation, e.g. "3B", "101". */
  designation: string;
  entrance?: string;
  /** Floor number: 0 = ground, negative = basement (defaults to 0). */
  floor?: number;
  /** Defaults to "apartment" server-side. */
  unit_type?: UnitType;
  /** Decimal as a string (e.g. "72.50"). */
  size_sqm?: string;
  rooms?: number;
  /** Ownership share 0–100 as a decimal string (defaults to "100.00"). */
  ownership_share?: string;
  description?: string;
}

/** Update unit request body (`UpdateUnitRequest`). All fields optional/partial. */
export interface UpdateUnitRequest {
  entrance?: string;
  designation?: string;
  floor?: number;
  unit_type?: UnitType;
  /** Decimal as a string (e.g. "72.50"). */
  size_sqm?: string;
  rooms?: number;
  /** Ownership share 0–100 as a decimal string. */
  ownership_share?: string;
  occupancy_status?: UnitOccupancyStatus;
  description?: string;
  notes?: string;
}
