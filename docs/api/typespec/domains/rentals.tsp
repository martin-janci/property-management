import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Rentals;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Short-term rental reservation (UC-29-30)")
model Reservation {
  id: uuid;
  unitId: uuid;
  platform: RentalPlatform;
  externalId: string;
  guestName: string;
  guestEmail?: email;
  guestPhone?: phoneNumber;
  guestCount: int32;
  checkIn: dateTime;
  checkOut: dateTime;
  status: ReservationStatus;
  totalPrice: Money;
  platformFee?: Money;
  cleaningFee?: Money;
  accessCode?: string;
  notes?: string;
  guestRegistrationId?: uuid;
  ...AuditableEntity;
}

enum RentalPlatform {
  Airbnb: "airbnb",
  Booking: "booking",
  Vrbo: "vrbo",
  Direct: "direct",
  Other: "other",
}

enum ReservationStatus {
  Pending: "pending",
  Confirmed: "confirmed",
  CheckedIn: "checked_in",
  CheckedOut: "checked_out",
  Cancelled: "cancelled",
  NoShow: "no_show",
}

@doc("Guest registration for police/authorities")
model GuestRegistration {
  id: uuid;
  reservationId?: uuid;
  unitId: uuid;
  firstName: string;
  lastName: string;
  dateOfBirth: date;
  nationality: string;
  documentType: DocumentType;
  documentNumber: string;
  documentExpiry?: date;
  documentScanUrl?: url;
  arrivalDate: date;
  departureDate: date;
  submittedToAuthorities: boolean;
  submittedAt?: dateTime;
  authorityReference?: string;
  ...AuditableEntity;
}

enum DocumentType {
  Passport: "passport",
  IdCard: "id_card",
  DriversLicense: "drivers_license",
  Other: "other",
}

@doc("Platform connection settings")
model PlatformConnection {
  id: uuid;
  unitId: uuid;
  platform: RentalPlatform;
  externalListingId?: string;
  apiKey?: string;
  isActive: boolean;
  lastSyncAt?: dateTime;
  syncStatus: SyncStatus;
  ...AuditableEntity;
}

enum SyncStatus {
  Synced: "synced",
  Pending: "pending",
  Error: "error",
  Disabled: "disabled",
}

@route("/api/v1/rentals")
@tag("Short-term Rentals")
interface RentalsApi {
  @get @route("/reservations") listReservations(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query unitId?: uuid,
    @query status?: ReservationStatus,
    @query from?: date,
    @query to?: date
  ): PaginatedResponse<Reservation>;

  @post @route("/reservations") createReservation(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: {
      unitId: uuid;
      platform: RentalPlatform;
      guestName: string;
      guestEmail?: email;
      guestPhone?: phoneNumber;
      guestCount: int32;
      checkIn: dateTime;
      checkOut: dateTime;
      totalPrice: Money;
    }
  ): { @statusCode statusCode: 201; @body body: Reservation; };

  @get @route("/reservations/{id}") getReservation(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Reservation | NotFoundError;

  @post @route("/reservations/{id}/check-in") checkIn(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Reservation;

  @post @route("/reservations/{id}/check-out") checkOut(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Reservation;

  @post @route("/reservations/{id}/generate-access-code") generateAccessCode(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): { accessCode: string; expiresAt: dateTime; };

  @get @route("/guests") listGuests(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query unitId?: uuid
  ): PaginatedResponse<GuestRegistration>;

  @post @route("/guests") registerGuest(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: {
      reservationId?: uuid;
      unitId: uuid;
      firstName: string;
      lastName: string;
      dateOfBirth: date;
      nationality: string;
      documentType: DocumentType;
      documentNumber: string;
      documentExpiry?: date;
      arrivalDate: date;
      departureDate: date;
    }
  ): { @statusCode statusCode: 201; @body body: GuestRegistration; };

  @post @route("/guests/{id}/submit-to-authorities") submitToAuthorities(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): GuestRegistration;

  @post @route("/sync") syncPlatforms(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @query unitId?: uuid
  ): { syncedCount: int32; errors: string[]; };

  @get @route("/connections") listConnections(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @query unitId?: uuid
  ): PlatformConnection[];

  @post @route("/connections") createConnection(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: { unitId: uuid; platform: RentalPlatform; externalListingId?: string; apiKey?: string; }
  ): { @statusCode statusCode: 201; @body body: PlatformConnection; };
}
