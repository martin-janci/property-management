/**
 * External Integrations Types
 *
 * TypeScript types for External Integrations API (Epic 61).
 */

// ============================================
// Story 61.1: Calendar Integration
// ============================================

export type CalendarProvider = 'google' | 'outlook' | 'apple' | 'caldav';
export type CalendarSyncStatus = 'active' | 'paused' | 'error' | 'disconnected';
export type SyncDirection = 'push' | 'pull' | 'bidirectional';

export interface CalendarConnection {
  id: string;
  organizationId: string;
  userId: string;
  provider: CalendarProvider;
  providerAccountId?: string;
  calendarId?: string;
  calendarName?: string;
  syncStatus: CalendarSyncStatus;
  lastSyncAt?: string;
  lastError?: string;
  syncDirection: SyncDirection;
  createdAt: string;
  updatedAt: string;
}

export interface CreateCalendarConnection {
  provider: CalendarProvider;
  authCode?: string;
  calendarId?: string;
  syncDirection?: SyncDirection;
}

export interface UpdateCalendarConnection {
  calendarId?: string;
  syncDirection?: SyncDirection;
  syncStatus?: CalendarSyncStatus;
}

export interface CalendarEvent {
  id: string;
  connectionId: string;
  externalEventId?: string;
  sourceType: string;
  sourceId?: string;
  title: string;
  description?: string;
  location?: string;
  startTime: string;
  endTime: string;
  allDay: boolean;
  recurrenceRule?: string;
  attendees?: CalendarAttendee[];
  lastSyncedAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface CalendarAttendee {
  email: string;
  name?: string;
  status?: 'pending' | 'accepted' | 'declined' | 'tentative';
}

export interface CreateCalendarEvent {
  connectionId: string;
  sourceType: string;
  sourceId?: string;
  title: string;
  description?: string;
  location?: string;
  startTime: string;
  endTime: string;
  allDay?: boolean;
  recurrenceRule?: string;
  attendees?: CalendarAttendee[];
}

export interface SyncCalendarRequest {
  fullSync?: boolean;
  dateRangeStart?: string;
  dateRangeEnd?: string;
}

export interface CalendarSyncResult {
  eventsCreated: number;
  eventsUpdated: number;
  eventsDeleted: number;
  errors: string[];
  syncedAt: string;
}

// ============================================
// Story 61.2: Accounting System Export
// ============================================

export type AccountingSystem = 'pohoda' | 'money_s3' | 'quickbooks' | 'xero';
export type ExportStatus = 'pending' | 'processing' | 'completed' | 'failed';
export type ExportType = 'invoices' | 'payments' | 'full';

export interface AccountingExport {
  id: string;
  organizationId: string;
  systemType: AccountingSystem;
  exportType: ExportType;
  periodStart: string;
  periodEnd: string;
  status: ExportStatus;
  filePath?: string;
  fileSize?: number;
  recordCount?: number;
  errorMessage?: string;
  exportedBy: string;
  createdAt: string;
  completedAt?: string;
}

export interface CreateAccountingExport {
  systemType: AccountingSystem;
  exportType: ExportType;
  periodStart: string;
  periodEnd: string;
  includeAttachments?: boolean;
  costCenterMapping?: Record<string, string>;
}

export interface AccountingExportSettings {
  id: string;
  organizationId: string;
  systemType: AccountingSystem;
  defaultCostCenter?: string;
  accountMappings?: Record<string, string>;
  vatSettings?: VatSettings;
  autoExportEnabled: boolean;
  autoExportSchedule?: string;
  createdAt: string;
  updatedAt: string;
}

export interface VatSettings {
  defaultVatRate: number;
  vatExemptCategories?: string[];
}

export interface UpdateAccountingExportSettings {
  defaultCostCenter?: string;
  accountMappings?: Record<string, string>;
  vatSettings?: VatSettings;
  autoExportEnabled?: boolean;
  autoExportSchedule?: string;
}

// ============================================
// Story 61.3: E-Signature Integration
// ============================================

export type ESignatureProvider = 'docusign' | 'adobe_sign' | 'hellosign' | 'internal';
export type ESignatureStatus =
  | 'draft'
  | 'sent'
  | 'viewed'
  | 'signed'
  | 'completed'
  | 'declined'
  | 'voided'
  | 'expired';
export type RecipientRole = 'signer' | 'viewer' | 'approver' | 'cc';

export interface ESignatureWorkflow {
  id: string;
  organizationId: string;
  documentId: string;
  provider: ESignatureProvider;
  externalEnvelopeId?: string;
  title: string;
  message?: string;
  status: ESignatureStatus;
  expiresAt?: string;
  reminderEnabled: boolean;
  reminderDays?: number;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
  completedAt?: string;
}

export interface ESignatureRecipient {
  id: string;
  workflowId: string;
  email: string;
  name: string;
  role: RecipientRole;
  signingOrder: number;
  status: ESignatureStatus;
  signedAt?: string;
  declinedAt?: string;
  declineReason?: string;
  reminderSentAt?: string;
  createdAt: string;
  updatedAt: string;
}

export interface ESignatureWorkflowWithRecipients extends ESignatureWorkflow {
  recipients: ESignatureRecipient[];
}

export interface CreateESignatureWorkflow {
  documentId: string;
  provider?: ESignatureProvider;
  title: string;
  message?: string;
  recipients: CreateESignatureRecipient[];
  expiresInDays?: number;
  reminderEnabled?: boolean;
  reminderDays?: number;
}

export interface CreateESignatureRecipient {
  email: string;
  name: string;
  role: RecipientRole;
  signingOrder?: number;
}

// ============================================
// Story 61.4: Video Conferencing
// ============================================

export type VideoProvider = 'zoom' | 'teams' | 'google_meet' | 'webex';
export type MeetingStatus = 'scheduled' | 'started' | 'ended' | 'cancelled';

export interface VideoConferenceConnection {
  id: string;
  organizationId: string;
  userId: string;
  provider: VideoProvider;
  providerUserId?: string;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateVideoConferenceConnection {
  provider: VideoProvider;
  authCode: string;
}

export interface VideoMeeting {
  id: string;
  organizationId: string;
  connectionId: string;
  externalMeetingId?: string;
  sourceType: string;
  sourceId?: string;
  title: string;
  description?: string;
  startTime: string;
  durationMinutes: number;
  timezone?: string;
  joinUrl?: string;
  hostUrl?: string;
  password?: string;
  status: MeetingStatus;
  recordingUrl?: string;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface CreateVideoMeeting {
  connectionId?: string;
  provider?: VideoProvider;
  sourceType: string;
  sourceId?: string;
  title: string;
  description?: string;
  startTime: string;
  durationMinutes: number;
  timezone?: string;
  participants?: MeetingParticipant[];
  settings?: MeetingSettings;
}

export interface MeetingParticipant {
  email: string;
  name?: string;
  isHost?: boolean;
}

export interface MeetingSettings {
  waitingRoom?: boolean;
  muteOnEntry?: boolean;
  autoRecording?: boolean;
  allowScreenShare?: boolean;
}

export interface UpdateVideoMeeting {
  title?: string;
  description?: string;
  startTime?: string;
  durationMinutes?: number;
  status?: MeetingStatus;
}

// ============================================
// Story 61.5: Webhook Notifications
// ============================================

export type WebhookStatus = 'active' | 'paused' | 'disabled';
export type WebhookDeliveryStatus = 'pending' | 'delivered' | 'failed' | 'retrying';

export type WebhookEventType =
  // Fault events
  | 'fault.created'
  | 'fault.updated'
  | 'fault.resolved'
  // Document events
  | 'document.uploaded'
  | 'document.signed'
  // Payment events
  | 'payment.received'
  | 'payment.overdue'
  // Meeting events
  | 'meeting.scheduled'
  | 'meeting.started'
  | 'meeting.ended'
  // Announcement events
  | 'announcement.published'
  // Vote events
  | 'vote.started'
  | 'vote.ended'
  // Visitor events
  | 'visitor.checked_in'
  | 'visitor.checked_out'
  // Package events
  | 'package.received'
  | 'package.picked_up';

export interface WebhookSubscription {
  id: string;
  organizationId: string;
  name: string;
  description?: string;
  url: string;
  secret?: string;
  events: WebhookEventType[];
  status: WebhookStatus;
  headers?: Record<string, string>;
  retryPolicy?: WebhookRetryPolicy;
  createdBy: string;
  createdAt: string;
  updatedAt: string;
}

export interface WebhookRetryPolicy {
  maxRetries: number;
  retryIntervalSeconds: number;
  exponentialBackoff: boolean;
}

export interface CreateWebhookSubscription {
  name: string;
  description?: string;
  url: string;
  events: WebhookEventType[];
  secret?: string;
  headers?: Record<string, string>;
  retryPolicy?: WebhookRetryPolicy;
}

export interface UpdateWebhookSubscription {
  name?: string;
  description?: string;
  url?: string;
  events?: WebhookEventType[];
  secret?: string;
  headers?: Record<string, string>;
  status?: WebhookStatus;
  retryPolicy?: WebhookRetryPolicy;
}

export interface WebhookDeliveryLog {
  id: string;
  subscriptionId: string;
  eventType: WebhookEventType;
  eventId: string;
  payload: unknown;
  status: WebhookDeliveryStatus;
  attempts: number;
  lastAttemptAt?: string;
  nextRetryAt?: string;
  responseStatus?: number;
  responseBody?: string;
  errorMessage?: string;
  durationMs?: number;
  createdAt: string;
}

export interface WebhookDeliveryQuery {
  subscriptionId?: string;
  eventType?: WebhookEventType;
  status?: WebhookDeliveryStatus;
  fromDate?: string;
  toDate?: string;
  limit?: number;
  offset?: number;
}

export interface WebhookStatistics {
  totalDeliveries: number;
  successfulDeliveries: number;
  failedDeliveries: number;
  pendingDeliveries: number;
  averageResponseTimeMs?: number;
  successRate: number;
}

export interface TestWebhookRequest {
  eventType: WebhookEventType;
  payload?: unknown;
}

export interface TestWebhookResponse {
  success: boolean;
  statusCode?: number;
  responseTimeMs?: number;
  error?: string;
}

// ============================================
// Integration Statistics
// ============================================

export interface IntegrationStatistics {
  calendarConnections: number;
  activeCalendarSyncs: number;
  accountingExportsThisMonth: number;
  esignatureWorkflowsPending: number;
  esignatureWorkflowsCompleted: number;
  videoMeetingsScheduled: number;
  webhookSubscriptions: number;
  webhookDeliveriesToday: number;
  webhookSuccessRate: number;
}

// ============================================
// Query Types
// ============================================

export interface CalendarQuery {
  userId?: string;
}

export interface CalendarEventsQuery {
  from?: string;
  to?: string;
}

export interface AccountingExportQuery {
  systemType?: AccountingSystem;
  limit?: number;
}

export interface ESignatureQuery {
  status?: ESignatureStatus;
  limit?: number;
}

export interface VideoMeetingQuery {
  from?: string;
  status?: MeetingStatus;
  limit?: number;
}

// ============================================
// Gap 83.2: Booking.com Channel Integration
// ============================================
//
// NOTE: the Booking.com backend handlers (api-server
// routes/integrations/{install,booking_channel}.rs) serialize with the
// default serde casing (snake_case), so these wire types deliberately use
// snake_case field names to match the JSON on the wire.

/** Connection + sync status for a Booking.com channel connection. */
export interface BookingChannelStatus {
  connected: boolean;
  hotel_id?: string | null;
  last_sync_at?: string | null;
  sync_error?: string | null;
  properties_count: number;
  reservations_count: number;
}

/** Request body for connecting a Booking.com hotel via Supply XML credentials. */
export interface BookingConnectRequest {
  hotel_id: string;
  username: string;
  password: string;
}

/** Response returned by the Booking.com connect endpoint. */
export interface BookingConnectResponse {
  success: boolean;
  message: string;
  hotel_id?: string;
}

/** Response returned by the Booking.com disconnect endpoint. */
export interface BookingDisconnectResponse {
  success: boolean;
  message: string;
}

/** Result of a Booking.com sync operation. */
export interface BookingSyncResult {
  success: boolean;
  items_synced: number;
  synced_at: string;
  error?: string | null;
}

/** A single cross-platform booking conflict (overlapping reservations). */
export interface BookingConflict {
  unit_id: string;
  booking_reservation_id: string;
  conflicting_platform: string;
  conflicting_booking_id: string;
  overlap_start: string;
  overlap_end: string;
}

/** Response from the Booking.com conflict-detection endpoint. */
export interface BookingConflictCheck {
  conflicts_found: number;
  conflicts: BookingConflict[];
  checked_at: string;
  /** True when the upstream query hit its reservation cap (results incomplete). */
  truncated: boolean;
}
