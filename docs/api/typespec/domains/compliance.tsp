import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Compliance;

using TypeSpec.Http;
using PropertyManagement.Shared;

// GDPR and compliance (UC-26)

@doc("GDPR data export request")
model DataExportRequest {
  id: uuid;
  userId: uuid;
  status: ExportStatus;
  format: ExportFormat;
  requestedAt: dateTime;
  completedAt?: dateTime;
  downloadUrl?: urlString;
  expiresAt?: dateTime;
  includeCategories: DataCategory[];
  ...AuditableEntity;
}

enum ExportStatus {
  Pending: "pending",
  Processing: "processing",
  Completed: "completed",
  Expired: "expired",
  Failed: "failed",
}

enum ExportFormat {
  JSON: "json",
  CSV: "csv",
  PDF: "pdf",
}

enum DataCategory {
  Profile: "profile",
  Units: "units",
  Documents: "documents",
  Messages: "messages",
  Votes: "votes",
  Faults: "faults",
  Payments: "payments",
  AuditLogs: "audit_logs",
}

@doc("GDPR data deletion request")
model DataDeletionRequest {
  id: uuid;
  userId: uuid;
  status: DeletionStatus;
  reason?: string;
  requestedAt: dateTime;
  scheduledFor: dateTime;
  completedAt?: dateTime;
  retainedDataCategories?: DataCategory[];
  ...AuditableEntity;
}

enum DeletionStatus {
  Pending: "pending",
  Scheduled: "scheduled",
  Processing: "processing",
  Completed: "completed",
  Cancelled: "cancelled",
}

@doc("User consent record")
model Consent {
  id: uuid;
  userId: uuid;
  consentType: ConsentType;
  granted: boolean;
  grantedAt?: dateTime;
  revokedAt?: dateTime;
  ipAddress?: string;
  userAgent?: string;
  version: string;
}

enum ConsentType {
  TermsOfService: "terms_of_service",
  PrivacyPolicy: "privacy_policy",
  Marketing: "marketing",
  Analytics: "analytics",
  ThirdPartySharing: "third_party_sharing",
  Cookies: "cookies",
}

@doc("Audit log entry")
model AuditLog {
  id: uuid;
  organizationId: uuid;
  userId?: uuid;
  action: string;
  resourceType: string;
  resourceId?: uuid;
  oldValue?: string;
  newValue?: string;
  ipAddress?: string;
  userAgent?: string;
  timestamp: dateTime;
}

@route("/api/v1/compliance")
@tag("Compliance & GDPR")
interface ComplianceApi {
  @doc("Request data export (GDPR)")
  @post
  @route("/gdpr/export")
  requestExport(
    @header("Authorization") token?: string,
    @body body: { format?: ExportFormat; includeCategories?: DataCategory[]; }
  ): { @statusCode statusCode: 202; @body body: DataExportRequest; };

  @doc("Get export request status")
  @get
  @route("/gdpr/export/{id}")
  getExportStatus(
    @header("Authorization") token?: string,
    @path id: uuid
  ): DataExportRequest | NotFoundError;

  @doc("Download exported data")
  @get
  @route("/gdpr/export/{id}/download")
  downloadExport(
    @header("Authorization") token?: string,
    @path id: uuid
  ): { @header("Content-Type") contentType: string; @body body: bytes; } | NotFoundError;

  @doc("Request data deletion (GDPR)")
  @post
  @route("/gdpr/delete")
  requestDeletion(
    @header("Authorization") token?: string,
    @body body: { reason?: string; retainedDataCategories?: DataCategory[]; }
  ): { @statusCode statusCode: 202; @body body: DataDeletionRequest; };

  @doc("Cancel deletion request")
  @post
  @route("/gdpr/delete/{id}/cancel")
  cancelDeletion(
    @header("Authorization") token?: string,
    @path id: uuid
  ): DataDeletionRequest | NotFoundError;

  @doc("Get deletion request status")
  @get
  @route("/gdpr/delete/{id}")
  getDeletionStatus(
    @header("Authorization") token?: string,
    @path id: uuid
  ): DataDeletionRequest | NotFoundError;

  @doc("Get current consents")
  @get
  @route("/consents")
  getConsents(
    @header("Authorization") token?: string
  ): Consent[];

  @doc("Update consent")
  @post
  @route("/consents")
  updateConsent(
    @header("Authorization") token?: string,
    @body body: { consentType: ConsentType; granted: boolean; }
  ): Consent;

  @doc("Get audit logs (admin)")
  @get
  @route("/audit-logs")
  getAuditLogs(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query userId?: uuid,
    @query action?: string,
    @query resourceType?: string,
    @query from?: dateTime,
    @query to?: dateTime
  ): PaginatedResponse<AuditLog>;
}
