/**
 * Buildings Module
 *
 * API client, hooks, and types for building management (UC-15).
 */

// Direct API functions
export { listBuildings, getBuilding } from './api';

// Direct hooks
export { useBuildings, useBuilding } from './hooks';

// Factory-based API and hooks (legacy pattern)
export { createBuildingsApi, type BuildingsApi } from './api';
export { createBuildingHooks, buildingKeys, type BuildingHooks } from './hooks';
export type {
  BuildingType,
  BuildingStatus,
  CommonAreaType,
  BuildingDocumentCategory,
  DocumentVisibility,
  GeoLocation,
  Address,
  Building,
  BuildingSummary,
  Floor,
  CommonArea,
  Attachment,
  BuildingDocument,
  CreateBuildingRequest,
  UpdateBuildingRequest,
  CreateFloorRequest,
  CreateCommonAreaRequest,
  UploadDocumentRequest,
  ListBuildingsParams,
  ListBuildingDocumentsParams,
  BuildingsPaginatedResponse,
} from './types';
