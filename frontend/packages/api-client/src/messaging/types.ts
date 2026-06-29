/**
 * Messaging Types
 *
 * TypeScript types for direct messaging (Epic 6, Story 6.5).
 */

// ============================================================================
// Core Types
// ============================================================================

/** Basic participant info for display */
export interface ParticipantInfo {
  id: string;
  firstName: string;
  lastName: string;
  email: string;
}

/** Message preview for thread list */
export interface MessagePreview {
  id: string;
  content: string;
  senderId: string;
  isFromMe: boolean;
  createdAt: string;
}

/** A conversation thread */
export interface MessageThread {
  id: string;
  organizationId: string;
  participantIds: string[];
  lastMessageAt: string | null;
  createdAt: string;
  updatedAt: string;
}

/** Thread with preview info for list display */
export interface ThreadWithPreview {
  id: string;
  organizationId: string;
  participantIds: string[];
  /**
   * All other participants (everyone except the current user). For a 2-party
   * thread this is a single entry; for an N-party group conversation it lists
   * every other participant ([BIT-206]).
   */
  participants: ParticipantInfo[];
  lastMessage: MessagePreview | null;
  unreadCount: number;
  createdAt: string;
  updatedAt: string;
}

/** An individual message */
export interface Message {
  id: string;
  threadId: string;
  senderId: string;
  content: string;
  readAt: string | null;
  deletedAt: string | null;
  deletedBy: string | null;
  createdAt: string;
  updatedAt: string;
}

/** Message with sender info for display */
export interface MessageWithSender {
  id: string;
  threadId: string;
  sender: ParticipantInfo;
  content: string;
  readAt: string | null;
  isDeleted: boolean;
  createdAt: string;
}

/** A file attachment linked to a message (UC-05.9) */
export interface MessageAttachment {
  id: string;
  messageId: string;
  fileKey: string;
  fileName: string;
  fileType: string;
  fileSize: number;
  createdAt: string;
}

/** A user block record */
export interface UserBlock {
  id: string;
  blockerId: string;
  blockedId: string;
  createdAt: string;
}

/** Block with blocked user info for display */
export interface BlockWithUserInfo {
  id: string;
  blockedUser: ParticipantInfo;
  createdAt: string;
}

// ============================================================================
// Request Types
// ============================================================================

/**
 * Request for starting a new thread.
 *
 * The wire fields are snake_case (the backend `StartThreadRequest` does not
 * rename to camelCase): `recipient_ids` carries the full set of recipients for
 * both 2-party direct messages (one id) and N-party group conversations
 * (UC-05.8 / [BIT-183]). The body is serialized verbatim, so these keys are the
 * wire contract — do not rename to camelCase.
 */
export interface StartThreadRequest {
  recipient_ids: string[];
  initial_message?: string;
}

/** Request for sending a message */
export interface SendMessageRequest {
  content: string;
}

/** Request for a presigned upload URL for a message attachment (UC-05.9) */
export interface AttachmentUploadRequest {
  fileName: string;
  fileType: string;
  fileSize: number;
}

/** Request to link an already-uploaded S3 object to a message */
export interface LinkAttachmentRequest {
  fileKey: string;
  fileName: string;
  fileType: string;
  fileSize: number;
}

/** Query params for listing threads */
export interface ListThreadsParams {
  limit?: number;
  offset?: number;
  /** Free-text search over thread participants / last message */
  search?: string;
  /**
   * When `true`, return only the current user's archived threads; otherwise
   * the default inbox (non-archived). Soft-deleted threads are excluded from
   * both. (BIT-182)
   */
  archived?: boolean;
}

/** Query params for listing messages */
export interface ListMessagesParams {
  limit?: number;
  offset?: number;
}

// ============================================================================
// Response Types
// ============================================================================

/** Response for thread list */
export interface ThreadListResponse {
  threads: ThreadWithPreview[];
  count: number;
  total: number;
}

/** Response for thread detail with messages */
export interface ThreadDetailResponse {
  thread: MessageThread;
  /**
   * All other participants (everyone except the caller). For a 2-party thread
   * this is a single entry; for a group conversation it lists every other
   * participant ([BIT-206]).
   */
  participants: ParticipantInfo[];
  messages: MessageWithSender[];
  messageCount: number;
}

/** Response for message creation */
export interface SendMessageResponse {
  message: string;
  sentMessage: Message;
}

/** Response for unread count */
export interface UnreadMessagesResponse {
  unreadCount: number;
}

/** Response for blocked users list */
export interface BlockedUsersResponse {
  blockedUsers: BlockWithUserInfo[];
  count: number;
}

/** Generic success response */
export interface MessageSuccessResponse {
  message: string;
}

/** Response carrying a presigned PUT URL plus the S3 key to echo back on link */
export interface AttachmentUploadUrlResponse {
  url: string;
  expiresAt: string;
  fileKey: string;
}

/** Response listing a message's attachments */
export interface MessageAttachmentsResponse {
  attachments: MessageAttachment[];
  count: number;
}

/** Response carrying a presigned download URL for an attachment */
export interface AttachmentDownloadResponse {
  url: string;
  expiresAt: string;
}
