import "../shared/models.tsp";
import "../shared/errors.tsp";
import "../shared/pagination.tsp";
import "../shared/tenant.tsp";

namespace PropertyManagement.Announcements;

using TypeSpec.Http;
using PropertyManagement.Shared;

@doc("Announcement (UC-06)")
model Announcement {
  id: uuid;
  organizationId: uuid;
  authorId: uuid;
  title: string;
  @doc("Markdown formatted content")
  content: string;
  targetType: AnnouncementTargetType;
  @doc("Array of target UUIDs (buildings, units, or role names)")
  targetIds: string[];
  status: AnnouncementStatus;
  scheduledAt?: dateTime;
  publishedAt?: dateTime;
  pinned: boolean;
  pinnedAt?: dateTime;
  pinnedBy?: uuid;
  commentsEnabled: boolean;
  acknowledgmentRequired: boolean;
  ...AuditableEntity;
}

enum AnnouncementStatus {
  Draft: "draft",
  Scheduled: "scheduled",
  Published: "published",
  Archived: "archived",
}

enum AnnouncementTargetType {
  All: "all",
  Building: "building",
  Units: "units",
  Roles: "roles",
}

@doc("Announcement summary for list view")
model AnnouncementSummary {
  id: uuid;
  title: string;
  status: AnnouncementStatus;
  targetType: AnnouncementTargetType;
  publishedAt?: dateTime;
  pinned: boolean;
  commentsEnabled: boolean;
  acknowledgmentRequired: boolean;
}

@doc("Announcement with additional details")
model AnnouncementWithDetails {
  ...Announcement;
  authorName: string;
  readCount: int64;
  acknowledgedCount: int64;
  commentCount: int64;
  attachmentCount: int64;
}

@doc("Announcement attachment")
model AnnouncementAttachment {
  id: uuid;
  announcementId: uuid;
  fileKey: string;
  fileName: string;
  fileType: string;
  fileSize: int64;
  createdAt: dateTime;
}

@doc("Announcement read record")
model AnnouncementRead {
  id: uuid;
  announcementId: uuid;
  userId: uuid;
  readAt: dateTime;
  acknowledgedAt?: dateTime;
}

@doc("Announcement statistics")
model AnnouncementStatistics {
  total: int64;
  published: int64;
  draft: int64;
  scheduled: int64;
  archived: int64;
}

@route("/api/v1/announcements")
@tag("Announcements")
interface AnnouncementsApi {
  @doc("List announcements with filters (managers)")
  @get list(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery,
    @query status?: string,
    @query targetType?: AnnouncementTargetType,
    @query authorId?: uuid,
    @query pinned?: boolean,
    @query fromDate?: string,
    @query toDate?: string
  ): PaginatedResponse<AnnouncementSummary>;

  @doc("List published announcements (all users)")
  @get @route("/published") listPublished(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    ...PaginationQuery
  ): PaginatedResponse<AnnouncementSummary>;

  @doc("Create a new announcement")
  @post create(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @body body: {
      title: string;
      content: string;
      targetType: AnnouncementTargetType;
      targetIds?: uuid[];
      scheduledAt?: dateTime;
      commentsEnabled?: boolean;
      acknowledgmentRequired?: boolean;
    }
  ): { @statusCode statusCode: 201; @body body: { id: uuid; message: string; }; };

  @doc("Get announcement details")
  @get @route("/{id}") get(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): { announcement: AnnouncementWithDetails; attachments: AnnouncementAttachment[]; } | NotFoundError;

  @doc("Update an announcement (draft/scheduled only)")
  @put @route("/{id}") update(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: {
      title?: string;
      content?: string;
      targetType?: AnnouncementTargetType;
      targetIds?: uuid[];
      scheduledAt?: dateTime;
      commentsEnabled?: boolean;
      acknowledgmentRequired?: boolean;
    }
  ): { message: string; announcement: Announcement; };

  @doc("Delete an announcement (draft only)")
  @delete @route("/{id}") delete(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): { @statusCode statusCode: 204; };

  @doc("Publish an announcement immediately")
  @post @route("/{id}/publish") publish(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): { message: string; announcement: Announcement; };

  @doc("Schedule an announcement for future publishing")
  @post @route("/{id}/schedule") schedule(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { scheduledAt: dateTime; }
  ): { message: string; announcement: Announcement; };

  @doc("Archive an announcement")
  @post @route("/{id}/archive") archive(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): { message: string; announcement: Announcement; };

  @doc("Pin or unpin an announcement")
  @post @route("/{id}/pin") pin(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: { pinned: boolean; }
  ): { message: string; announcement: Announcement; };

  @doc("List announcement attachments")
  @get @route("/{id}/attachments") listAttachments(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): AnnouncementAttachment[];

  @doc("Add an attachment to an announcement")
  @post @route("/{id}/attachments") addAttachment(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @body body: {
      fileKey: string;
      fileName: string;
      fileType: string;
      fileSize: int64;
    }
  ): { @statusCode statusCode: 201; @body body: AnnouncementAttachment; };

  @doc("Delete an attachment")
  @delete @route("/{id}/attachments/{attachmentId}") deleteAttachment(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid,
    @path attachmentId: uuid
  ): { @statusCode statusCode: 204; };

  @doc("Mark an announcement as read")
  @post @route("/{id}/read") markRead(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): { @statusCode statusCode: 200; };

  @doc("Acknowledge an announcement")
  @post @route("/{id}/acknowledge") acknowledge(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid,
    @path id: uuid
  ): { @statusCode statusCode: 200; };

  @doc("Get announcement statistics")
  @get @route("/statistics") getStatistics(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid
  ): AnnouncementStatistics;

  @doc("Get unread announcement count")
  @get @route("/unread-count") getUnreadCount(
    @header("Authorization") token?: string,
    @header("X-Tenant-ID") tenantId?: uuid
  ): { unreadCount: int64; };
}
