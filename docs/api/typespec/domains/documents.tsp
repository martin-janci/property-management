import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Documents;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Document (UC-08)")
model Document {
  id: uuid;
  organizationId: uuid;
  buildingId?: uuid;
  unitId?: uuid;
  title: string;
  description?: string;
  category: DocumentCategory;
  file: Attachment;
  visibility: DocumentVisibility;
  tags?: string[];
  expiresAt?: dateTime;
  version: int32;
  previousVersionId?: uuid;
  ...AuditableEntity;
}

enum DocumentCategory {
  Contract: "contract",
  Invoice: "invoice",
  Receipt: "receipt",
  Report: "report",
  Minutes: "minutes",
  Policy: "policy",
  Manual: "manual",
  Certificate: "certificate",
  Permit: "permit",
  Insurance: "insurance",
  Other: "other",
}

enum DocumentVisibility {
  Public: "public",
  BuildingOnly: "building_only",
  OwnersOnly: "owners_only",
  ManagersOnly: "managers_only",
  Private: "private",
}

@doc("Document folder")
model Folder {
  id: uuid;
  organizationId: uuid;
  parentId?: uuid;
  name: string;
  description?: string;
  visibility: DocumentVisibility;
  documentCount: int32;
  ...AuditableEntity;
}

@doc("Presigned-URL response for document download/preview (Story 7A.4)")
model DocumentUrlResponse {
  url: string;
  expiresAt: dateTime;
}

@doc("A single document version (Story 7B.1)")
model DocumentVersion {
  id: uuid;
  versionNumber: int32;
  isCurrentVersion: boolean;
  fileKey: string;
  fileName: string;
  mimeType: string;
  sizeBytes: int64;
  createdBy: uuid;
  createdByName: string;
  createdAt: dateTime;
}

@doc("Full version history for a document (Story 7B.1)")
model DocumentVersionHistory {
  documentId: uuid;
  title: string;
  totalVersions: int32;
  versions: DocumentVersion[];
}

@doc("Envelope returned by the version-history endpoint")
model VersionHistoryResponse {
  history: DocumentVersionHistory;
}

@doc("Result of creating a document version (Story 7B.1)")
model CreateVersionResponse {
  id: uuid;
  versionNumber: int32;
  message: string;
}

@route("/api/v1/documents")
@tag("Documents")
interface DocumentsApi {
  @get list(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    ...SearchQuery,
    @query buildingId?: uuid,
    @query category?: DocumentCategory,
    @query folderId?: uuid
  ): PaginatedResponse<Document>;

  @post upload(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: {
      title: string;
      description?: string;
      category: DocumentCategory;
      buildingId?: uuid;
      unitId?: uuid;
      folderId?: uuid;
      visibility: DocumentVisibility;
      tags?: string[];
      fileContent: string;
      fileName: string;
      mimeType: string;
    }
  ): { @statusCode statusCode: 201; @body body: Document; };

  @get @route("/{id}") get(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): Document | NotFoundError;

  @patch @route("/{id}") update(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { title?: string; description?: string; category?: DocumentCategory; visibility?: DocumentVisibility; tags?: string[]; }
  ): Document;

  @delete @route("/{id}") delete(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): { @statusCode statusCode: 204; };

  // Returns a presigned S3 URL (with Content-Disposition: attachment), not the
  // file bytes — matches the api-server `UrlResponse` handler (Story 7A.4).
  @get @route("/{id}/download") download(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): DocumentUrlResponse;

  // Create a new version. The api-server route is POST /{id}/versions taking a
  // storage-key payload (file already uploaded to S3), not POST /{id}/new-version.
  @post @route("/{id}/versions") uploadVersion(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { fileKey: string; fileName: string; mimeType: string; sizeBytes: int64; }
  ): { @statusCode statusCode: 201; @body body: CreateVersionResponse; };

  @get @route("/{id}/versions") listVersions(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): VersionHistoryResponse;

  @get @route("/folders") listFolders(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @query parentId?: uuid
  ): Folder[];

  @post @route("/folders") createFolder(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: { name: string; description?: string; parentId?: uuid; visibility: DocumentVisibility; }
  ): { @statusCode statusCode: 201; @body body: Folder; };
}
