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
  otherParticipant: ParticipantInfo;
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

/** Request for starting a new thread */
export interface StartThreadRequest {
  recipientId: string;
  initialMessage?: string;
}

/** Request for sending a message */
export interface SendMessageRequest {
  content: string;
}

/** Query params for listing threads */
export interface ListThreadsParams {
  limit?: number;
  offset?: number;
  /** Free-text search over thread participants / last message */
  search?: string;
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
  otherParticipant: ParticipantInfo;
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
