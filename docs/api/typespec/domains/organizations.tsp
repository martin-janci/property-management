import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Organizations;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Organization (housing cooperative, property management company)")
model Organization {
  @doc("Unique identifier")
  id: uuid;

  @doc("Organization name")
  name: string;

  @doc("Legal name")
  legalName?: string;

  @doc("Tax/VAT ID")
  taxId?: string;

  @doc("Registration number")
  registrationNumber?: string;

  @doc("Headquarters address")
  address: Address;

  @doc("Contact email")
  email: email;

  @doc("Contact phone")
  phone?: phoneNumber;

  @doc("Website URL")
  website?: url;

  @doc("Logo URL")
  logoUrl?: url;

  @doc("Organization type")
  type: OrganizationType;

  @doc("Subscription tier")
  subscriptionTier: SubscriptionTier;

  @doc("Organization status")
  status: OrganizationStatus;

  @doc("Custom branding settings")
  branding?: OrganizationBranding;

  @doc("Number of buildings")
  buildingCount: int32;

  @doc("Number of units")
  unitCount: int32;

  ...AuditableEntity;
}

@doc("Organization type")
enum OrganizationType {
  HousingCooperative: "housing_cooperative",
  PropertyManagement: "property_management",
  RealEstateAgency: "real_estate_agency",
  Individual: "individual",
}

@doc("Organization status")
enum OrganizationStatus {
  Active: "active",
  Trial: "trial",
  Suspended: "suspended",
  Cancelled: "cancelled",
}

@doc("Subscription tier")
enum SubscriptionTier {
  Free: "free",
  Starter: "starter",
  Professional: "professional",
  Enterprise: "enterprise",
}

@doc("Organization branding settings")
model OrganizationBranding {
  @doc("Primary color (hex)")
  primaryColor?: string;

  @doc("Secondary color (hex)")
  secondaryColor?: string;

  @doc("Custom logo URL")
  logoUrl?: url;

  @doc("Custom favicon URL")
  faviconUrl?: url;

  @doc("Custom email footer")
  emailFooter?: string;
}

@doc("Organization member")
model OrganizationMember {
  @doc("User ID")
  userId: uuid;

  @doc("User email")
  email: email;

  @doc("User display name")
  displayName: string;

  @doc("Role in organization")
  role: TenantRole;

  @doc("Assigned buildings (empty = all)")
  assignedBuildingIds: uuid[];

  @doc("Joined timestamp")
  joinedAt: dateTime;

  @doc("Membership status")
  status: MemberStatus;
}

@doc("Member status")
enum MemberStatus {
  Active: "active",
  Invited: "invited",
  Suspended: "suspended",
}

@doc("Create organization request")
model CreateOrganizationRequest {
  name: string;
  legalName?: string;
  taxId?: string;
  registrationNumber?: string;
  address: Address;
  email: email;
  phone?: phoneNumber;
  website?: url;
  type: OrganizationType;
}

@doc("Invite member request")
model InviteMemberRequest {
  email: email;
  role: TenantRole;
  assignedBuildingIds?: uuid[];
  message?: string;
}

@route("/api/v1/organizations")
@tag("Organizations")
interface OrganizationsApi {
  @doc("List organizations (Super Admin only)")
  @get
  list(
    @header("Authorization") token?: string,
    ...PaginationQuery,
    @query status?: OrganizationStatus
  ): PaginatedResponse<Organization> | UnauthorizedError | ForbiddenError;

  @doc("Create new organization")
  @post
  create(
    @header("Authorization") token?: string,
    @body body: CreateOrganizationRequest
  ): {
    @statusCode statusCode: 201;
    @body body: Organization;
  } | UnauthorizedError | BadRequestError;

  @doc("Get organization by ID")
  @get
  @route("/{id}")
  get(
    @header("Authorization") token?: string,
    @path id: uuid
  ): Organization | UnauthorizedError | NotFoundError;

  @doc("Update organization")
  @patch
  @route("/{id}")
  update(
    @header("Authorization") token?: string,
    @path id: uuid,
    @body body: {
      name?: string;
      legalName?: string;
      taxId?: string;
      registrationNumber?: string;
      address?: Address;
      email?: email;
      phone?: phoneNumber;
      website?: url;
    }
  ): Organization | UnauthorizedError | ForbiddenError | NotFoundError | BadRequestError;

  @doc("Delete organization")
  @delete
  @route("/{id}")
  delete(
    @header("Authorization") token?: string,
    @path id: uuid
  ): {
    @statusCode statusCode: 204;
  } | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Update organization branding")
  @put
  @route("/{id}/branding")
  updateBranding(
    @header("Authorization") token?: string,
    @path id: uuid,
    @body body: OrganizationBranding
  ): Organization | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Get organization statistics")
  @get
  @route("/{id}/stats")
  getStats(
    @header("Authorization") token?: string,
    @path id: uuid
  ): {
    buildingCount: int32;
    unitCount: int32;
    memberCount: int32;
    activeIssues: int32;
    pendingVotes: int32;
  } | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("List organization members")
  @get
  @route("/{id}/members")
  listMembers(
    @header("Authorization") token?: string,
    @path id: uuid,
    ...PaginationQuery
  ): PaginatedResponse<OrganizationMember> | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Invite member to organization")
  @post
  @route("/{id}/members/invite")
  inviteMember(
    @header("Authorization") token?: string,
    @path id: uuid,
    @body body: InviteMemberRequest
  ): {
    @statusCode statusCode: 201;
    @body body: OrganizationMember;
  } | UnauthorizedError | ForbiddenError | NotFoundError | ConflictError;

  @doc("Update member role")
  @patch
  @route("/{id}/members/{userId}")
  updateMember(
    @header("Authorization") token?: string,
    @path id: uuid,
    @path userId: uuid,
    @body body: {
      role?: TenantRole;
      assignedBuildingIds?: uuid[];
    }
  ): OrganizationMember | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Remove member from organization")
  @delete
  @route("/{id}/members/{userId}")
  removeMember(
    @header("Authorization") token?: string,
    @path id: uuid,
    @path userId: uuid
  ): {
    @statusCode statusCode: 204;
  } | UnauthorizedError | ForbiddenError | NotFoundError;

  @doc("Switch current tenant context")
  @post
  @route("/switch/{id}")
  switchTenant(
    @header("Authorization") token?: string,
    @path id: uuid
  ): {
    accessToken: string;
    organization: Organization;
  } | UnauthorizedError | ForbiddenError | NotFoundError;
}
