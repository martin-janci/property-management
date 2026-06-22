import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Units;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Property unit (apartment, office, etc.)")
model Unit {
  id: uuid;
  buildingId: uuid;
  floorId?: uuid;
  unitNumber: string;
  type: UnitType;
  areaM2: float64;
  roomCount?: int32;
  bathroomCount?: int32;
  balconyCount?: int32;
  hasParking: boolean;
  hasStorage: boolean;
  status: UnitStatus;
  ownershipShare?: float64;
  description?: string;
  photoUrls?: url[];
  ...AuditableEntity;
}

enum UnitType {
  Apartment: "apartment",
  Studio: "studio",
  Penthouse: "penthouse",
  Office: "office",
  Retail: "retail",
  Storage: "storage",
  Parking: "parking",
}

enum UnitStatus {
  Occupied: "occupied",
  Vacant: "vacant",
  ForSale: "for_sale",
  ForRent: "for_rent",
  Renovation: "renovation",
}

@doc("Unit owner")
model UnitOwner {
  id: uuid;
  unitId: uuid;
  userId?: uuid;
  name: string;
  email?: email;
  phone?: phoneNumber;
  ownershipPercentage: float64;
  isPrimaryOwner: boolean;
  acquiredAt?: date;
  ...AuditableEntity;
}

@doc("Unit resident")
model UnitResident {
  id: uuid;
  unitId: uuid;
  userId?: uuid;
  name: string;
  email?: email;
  phone?: phoneNumber;
  type: ResidentType;
  moveInDate?: date;
  moveOutDate?: date;
  isActive: boolean;
  ...AuditableEntity;
}

enum ResidentType {
  Owner: "owner",
  Tenant: "tenant",
  FamilyMember: "family_member",
  Subtenant: "subtenant",
}

@route("/api/v1/units")
@tag("Units")
interface UnitsApi {
  @get list(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query buildingId?: uuid,
    @query status?: UnitStatus
  ): PaginatedResponse<Unit>;

  @post create(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: {
      buildingId: uuid;
      floorId?: uuid;
      unitNumber: string;
      type: UnitType;
      areaM2: float64;
      roomCount?: int32;
      bathroomCount?: int32;
    }
  ): { @statusCode statusCode: 201; @body body: Unit; };

  @get @route("/{id}") get(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Unit | NotFoundError;

  @patch @route("/{id}") update(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { unitNumber?: string; areaM2?: float64; status?: UnitStatus; }
  ): Unit;

  @get @route("/{id}/owners") listOwners(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): UnitOwner[];

  @post @route("/{id}/owners") addOwner(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { userId?: uuid; name: string; email?: email; ownershipPercentage: float64; }
  ): { @statusCode statusCode: 201; @body body: UnitOwner; };

  @get @route("/{id}/residents") listResidents(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): UnitResident[];

  @post @route("/{id}/residents") addResident(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { userId?: uuid; name: string; type: ResidentType; moveInDate?: date; }
  ): { @statusCode statusCode: 201; @body body: UnitResident; };
}
