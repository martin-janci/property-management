import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Buildings;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Building")
model Building {
  @doc("Unique identifier")
  id: uuid;

  @doc("Organization ID")
  organizationId: uuid;

  @doc("Building name")
  name: string;

  @doc("Building address")
  address: Address;

  @doc("GPS location")
  location?: GeoLocation;

  @doc("Building type")
  type: BuildingType;

  @doc("Number of floors")
  floorCount: int32;

  @doc("Number of units")
  unitCount: int32;

  @doc("Year built")
  yearBuilt?: int32;

  @doc("Total building area in m2")
  totalAreaM2?: float64;

  @doc("Building photo URL")
  photoUrl?: url;

  @doc("Building description")
  description?: string;

  @doc("Building status")
  status: BuildingStatus;

  @doc("Manager user ID")
  managerId?: uuid;

  @doc("Technical manager user ID")
  technicalManagerId?: uuid;

  ...AuditableEntity;
}

@doc("Building type")
enum BuildingType {
  Residential: "residential",
  Commercial: "commercial",
  Mixed: "mixed",
  Industrial: "industrial",
}

@doc("Building status")
enum BuildingStatus {
  Active: "active",
  UnderConstruction: "under_construction",
  Renovation: "renovation",
  Inactive: "inactive",
}

@doc("Floor in a building")
model Floor {
  @doc("Unique identifier")
  id: uuid;

  @doc("Building ID")
  buildingId: uuid;

  @doc("Floor number (0 = ground, -1 = basement)")
  number: int32;

  @doc("Floor name/label")
  name?: string;

  @doc("Floor plan URL")
  floorPlanUrl?: url;

  @doc("Number of units on this floor")
  unitCount: int32;
}

@doc("Common area (staircase, elevator, parking, garden, etc.)")
model CommonArea {
  @doc("Unique identifier")
  id: uuid;

  @doc("Building ID")
  buildingId: uuid;

  @doc("Area name")
  name: string;

  @doc("Area type")
  type: CommonAreaType;

  @doc("Description")
  description?: string;

  @doc("Area size in m2")
  areaM2?: float64;

  @doc("Floor ID (if applicable)")
  floorId?: uuid;

  @doc("Photo URL")
  photoUrl?: url;
}

@doc("Common area type")
enum CommonAreaType {
  Staircase: "staircase",
  Elevator: "elevator",
  Lobby: "lobby",
  Hallway: "hallway",
  Basement: "basement",
  Attic: "attic",
  Parking: "parking",
  Garage: "garage",
  Garden: "garden",
  Playground: "playground",
  Pool: "pool",
  Gym: "gym",
  LaundryRoom: "laundry_room",
  StorageRoom: "storage_room",
  Other: "other",
}

@doc("Building document")
model BuildingDocument {
  @doc("Unique identifier")
  id: uuid;

  @doc("Building ID")
  buildingId: uuid;

  @doc("Document title")
  title: string;

  @doc("Document category")
  category: DocumentCategory;

  @doc("File attachment")
  file: Attachment;

  @doc("Description")
  description?: string;

  @doc("Visibility")
  visibility: DocumentVisibility;

  ...AuditableEntity;
}

@doc("Document category")
enum DocumentCategory {
  LegalDocument: "legal_document",
  Insurance: "insurance",
  Maintenance: "maintenance",
  FinancialReport: "financial_report",
  MeetingMinutes: "meeting_minutes",
  Contract: "contract",
  TechnicalDocument: "technical_document",
  Other: "other",
}

@doc("Document visibility")
enum DocumentVisibility {
  Public: "public",
  OwnersOnly: "owners_only",
  ManagersOnly: "managers_only",
}

@doc("Create building request")
model CreateBuildingRequest {
  name: string;
  address: Address;
  location?: GeoLocation;
  type: BuildingType;
  floorCount: int32;
  yearBuilt?: int32;
  totalAreaM2?: float64;
  description?: string;
  managerId?: uuid;
  technicalManagerId?: uuid;
}

@route("/api/v1/buildings")
@tag("Buildings")
interface BuildingsApi {
  @doc("List buildings")
  @get
  list(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query status?: BuildingStatus,
    @query type?: BuildingType
  ): PaginatedResponse<Building> | UnauthorizedError | ForbiddenError;

  @doc("Create new building")
  @post
  create(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: CreateBuildingRequest
  ): {
    @statusCode statusCode: 201;
    @body body: Building;
  } | UnauthorizedError | ForbiddenError | BadRequestError;

  @doc("Get building by ID")
  @get
  @route("/{id}")
  get(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Building | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Update building")
  @patch
  @route("/{id}")
  update(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: {
      name?: string;
      address?: Address;
      location?: GeoLocation;
      type?: BuildingType;
      floorCount?: int32;
      yearBuilt?: int32;
      totalAreaM2?: float64;
      description?: string;
      status?: BuildingStatus;
      managerId?: uuid;
      technicalManagerId?: uuid;
    }
  ): Building | UnauthorizedError | ForbiddenError | NotFoundError | BadRequestError;

  @doc("Delete building")
  @delete
  @route("/{id}")
  delete(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): {
    @statusCode statusCode: 204;
  } | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("List floors in building")
  @get
  @route("/{id}/floors")
  listFloors(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Floor[] | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Create floor")
  @post
  @route("/{id}/floors")
  createFloor(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: {
      number: int32;
      name?: string;
    }
  ): {
    @statusCode statusCode: 201;
    @body body: Floor;
  } | UnauthorizedError | ForbiddenError | NotFoundError | BadRequestError;

  @doc("List common areas")
  @get
  @route("/{id}/common-areas")
  listCommonAreas(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): CommonArea[] | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Create common area")
  @post
  @route("/{id}/common-areas")
  createCommonArea(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: {
      name: string;
      type: CommonAreaType;
      description?: string;
      areaM2?: float64;
      floorId?: uuid;
    }
  ): {
    @statusCode statusCode: 201;
    @body body: CommonArea;
  } | UnauthorizedError | ForbiddenError | NotFoundError | BadRequestError;

  @doc("List building documents")
  @get
  @route("/{id}/documents")
  listDocuments(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    ...PaginationQuery,
    @query category?: DocumentCategory
  ): PaginatedResponse<BuildingDocument> | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Upload building document")
  @post
  @route("/{id}/documents")
  uploadDocument(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: {
      title: string;
      category: DocumentCategory;
      description?: string;
      visibility: DocumentVisibility;
      @doc("Base64 encoded file content")
      fileContent: string;
      fileName: string;
      mimeType: string;
    }
  ): {
    @statusCode statusCode: 201;
    @body body: BuildingDocument;
  } | UnauthorizedError | ForbiddenError | NotFoundError | BadRequestError;
}
