import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Listings;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Property listing for sale/rent (UC-31-32)")
model Listing {
  id: uuid;
  unitId: uuid;
  type: ListingType;
  status: ListingStatus;
  title: string;
  description: string;
  price: Money;
  priceType: PriceType;
  features: ListingFeatures;
  photos: Attachment[];
  virtualTourUrl?: url;
  videoUrl?: url;
  agentId?: uuid;
  publishedAt?: dateTime;
  expiresAt?: dateTime;
  viewCount: int32;
  inquiryCount: int32;
  portalListings?: PortalListing[];
  ...AuditableEntity;
}

enum ListingType {
  Sale: "sale",
  LongTermRent: "long_term_rent",
  ShortTermRent: "short_term_rent",
}

enum ListingStatus {
  Draft: "draft",
  Active: "active",
  Pending: "pending",
  Sold: "sold",
  Rented: "rented",
  Expired: "expired",
  Archived: "archived",
}

enum PriceType {
  Fixed: "fixed",
  Negotiable: "negotiable",
  PerMonth: "per_month",
  PerNight: "per_night",
}

model ListingFeatures {
  areaM2: float64;
  roomCount?: int32;
  bathroomCount?: int32;
  floorNumber?: int32;
  totalFloors?: int32;
  yearBuilt?: int32;
  hasBalcony?: boolean;
  hasParking?: boolean;
  hasElevator?: boolean;
  hasFurniture?: boolean;
  petFriendly?: boolean;
  energyRating?: string;
  heatingType?: string;
  amenities?: string[];
}

@doc("Listing on external portal")
model PortalListing {
  portalId: uuid;
  portalName: string;
  externalId: string;
  externalUrl?: url;
  status: PortalListingStatus;
  lastSyncAt?: dateTime;
}

enum PortalListingStatus {
  Published: "published",
  Pending: "pending",
  Error: "error",
  Removed: "removed",
}

@doc("Inquiry from potential buyer/renter")
model Inquiry {
  id: uuid;
  listingId: uuid;
  name: string;
  email: email;
  phone?: phoneNumber;
  message: string;
  source: InquirySource;
  status: InquiryStatus;
  scheduledViewingAt?: dateTime;
  notes?: string;
  ...AuditableEntity;
}

enum InquirySource {
  Website: "website",
  Portal: "portal",
  Phone: "phone",
  Email: "email",
  WalkIn: "walk_in",
}

enum InquiryStatus {
  New: "new",
  Contacted: "contacted",
  ViewingScheduled: "viewing_scheduled",
  ViewingCompleted: "viewing_completed",
  Interested: "interested",
  NotInterested: "not_interested",
  Closed: "closed",
}

@doc("Real estate portal configuration")
model PortalConfig {
  id: uuid;
  organizationId: uuid;
  portalName: string;
  apiEndpoint: url;
  apiKey: string;
  isActive: boolean;
  autoPublish: boolean;
  ...AuditableEntity;
}

@route("/api/v1/listings")
@tag("Real Estate Listings")
interface ListingsApi {
  @get list(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query type?: ListingType,
    @query status?: ListingStatus,
    @query minPrice?: int64,
    @query maxPrice?: int64
  ): PaginatedResponse<Listing>;

  @post create(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: {
      unitId: uuid;
      type: ListingType;
      title: string;
      description: string;
      price: Money;
      priceType: PriceType;
      features: ListingFeatures;
    }
  ): { @statusCode statusCode: 201; @body body: Listing; };

  @get @route("/{id}") get(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Listing | NotFoundError;

  @patch @route("/{id}") update(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { title?: string; description?: string; price?: Money; features?: ListingFeatures; }
  ): Listing;

  @post @route("/{id}/publish") publish(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Listing;

  @post @route("/{id}/unpublish") unpublish(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Listing;

  @post @route("/{id}/photos") uploadPhoto(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { fileContent: string; fileName: string; mimeType: string; }
  ): { @statusCode statusCode: 201; @body body: Attachment; };

  @post @route("/{id}/publish-to-portal") publishToPortal(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { portalId: uuid; }
  ): PortalListing;

  @get @route("/{id}/inquiries") listInquiries(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    ...PaginationQuery
  ): PaginatedResponse<Inquiry>;

  @post @route("/{id}/inquiries") createInquiry(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { name: string; email: email; phone?: phoneNumber; message: string; source?: InquirySource; }
  ): { @statusCode statusCode: 201; @body body: Inquiry; };

  @patch @route("/inquiries/{id}") updateInquiry(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { status?: InquiryStatus; scheduledViewingAt?: dateTime; notes?: string; }
  ): Inquiry;

  @get @route("/portals") listPortals(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid
  ): PortalConfig[];

  @post @route("/portals") configurePortal(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: { portalName: string; apiEndpoint: url; apiKey: string; autoPublish?: boolean; }
  ): { @statusCode statusCode: 201; @body body: PortalConfig; };
}
