/**
 * Buildings Module
 *
 * API client, hooks, and types for building management (UC-15).
 */

// Direct API functions
// Factory-based API and hooks (legacy pattern)
export { type BuildingsApi, createBuildingsApi, getBuilding, listBuildings } from './api';
// Direct hooks
export {
  type BuildingHooks,
  buildingKeys,
  createBuildingHooks,
  useBuilding,
  useBuildings,
} from './hooks';
export type {
  Address,
  Attachment,
  Building,
  BuildingDocument,
  BuildingDocumentCategory,
  BuildingStatus,
  BuildingSummary,
  BuildingsPaginatedResponse,
  BuildingType,
  CommonArea,
  CommonAreaType,
  CreateBuildingRequest,
  CreateCommonAreaRequest,
  CreateFloorRequest,
  CreateUnitRequest,
  DocumentVisibility,
  Floor,
  GeoLocation,
  ListBuildingDocumentsParams,
  ListBuildingsParams,
  ListUnitsParams,
  Unit,
  UnitOccupancyStatus,
  UnitStatus,
  UnitSummary,
  UnitsListResponse,
  UnitType,
  UpdateBuildingRequest,
  UpdateUnitRequest,
  UploadDocumentRequest,
} from './types';
