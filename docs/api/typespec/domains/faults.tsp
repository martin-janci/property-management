import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Faults;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Fault report (UC-03)")
model Fault {
  id: uuid;
  buildingId: uuid;
  unitId?: uuid;
  commonAreaId?: uuid;
  reporterId: uuid;
  title: string;
  description: string;
  category: FaultCategory;
  priority: FaultPriority;
  status: FaultStatus;
  photos?: Attachment[];
  assignedTo?: uuid;
  estimatedCost?: Money;
  actualCost?: Money;
  scheduledDate?: dateTime;
  completedAt?: dateTime;
  resolution?: string;
  ...AuditableEntity;
}

enum FaultCategory {
  Plumbing: "plumbing",
  Electrical: "electrical",
  HVAC: "hvac",
  Structural: "structural",
  Elevator: "elevator",
  Security: "security",
  Cleaning: "cleaning",
  Landscaping: "landscaping",
  Other: "other",
}

enum FaultPriority {
  Low: "low",
  Medium: "medium",
  High: "high",
  Critical: "critical",
}

enum FaultStatus {
  Reported: "reported",
  Acknowledged: "acknowledged",
  InProgress: "in_progress",
  OnHold: "on_hold",
  Resolved: "resolved",
  Closed: "closed",
  Rejected: "rejected",
}

@doc("Fault comment")
model FaultComment {
  id: uuid;
  faultId: uuid;
  authorId: uuid;
  authorName: string;
  content: string;
  attachments?: Attachment[];
  isInternal: boolean;
  createdAt: dateTime;
}

@route("/api/v1/faults")
@tag("Faults")
interface FaultsApi {
  @get list(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query buildingId?: uuid,
    @query status?: FaultStatus,
    @query priority?: FaultPriority,
    @query category?: FaultCategory
  ): PaginatedResponse<Fault>;

  @post create(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: {
      buildingId: uuid;
      unitId?: uuid;
      commonAreaId?: uuid;
      title: string;
      description: string;
      category: FaultCategory;
      priority?: FaultPriority;
    }
  ): { @statusCode statusCode: 201; @body body: Fault; };

  @get @route("/{id}") get(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Fault | NotFoundError;

  @patch @route("/{id}") update(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: {
      title?: string;
      description?: string;
      category?: FaultCategory;
      priority?: FaultPriority;
      status?: FaultStatus;
      assignedTo?: uuid;
      estimatedCost?: Money;
      scheduledDate?: dateTime;
    }
  ): Fault;

  @post @route("/{id}/resolve") resolve(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { resolution: string; actualCost?: Money; }
  ): Fault;

  @get @route("/{id}/comments") listComments(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): FaultComment[];

  @post @route("/{id}/comments") addComment(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { content: string; isInternal?: boolean; }
  ): { @statusCode statusCode: 201; @body body: FaultComment; };

  @post @route("/{id}/photos") uploadPhoto(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { fileContent: string; fileName: string; mimeType: string; }
  ): { @statusCode statusCode: 201; @body body: Attachment; };
}
