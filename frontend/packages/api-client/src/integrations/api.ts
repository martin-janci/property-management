/**
 * External Integrations API Client
 *
 * API functions for External Integrations (Epic 61).
 */

import type {
  AccountingExport,
  AccountingExportQuery,
  AccountingExportSettings,
  BookingChannelStatus,
  BookingConflictCheck,
  BookingConnectRequest,
  BookingConnectResponse,
  BookingDisconnectResponse,
  BookingSyncResult,
  CalendarConnection,
  CalendarEvent,
  CalendarEventsQuery,
  CalendarQuery,
  CalendarSyncResult,
  CreateAccountingExport,
  CreateCalendarConnection,
  CreateCalendarEvent,
  CreateESignatureWorkflow,
  CreateVideoConferenceConnection,
  CreateVideoMeeting,
  CreateWebhookSubscription,
  ESignatureQuery,
  ESignatureWorkflow,
  ESignatureWorkflowWithRecipients,
  IntegrationStatistics,
  SyncCalendarRequest,
  TestWebhookRequest,
  TestWebhookResponse,
  UpdateAccountingExportSettings,
  UpdateCalendarConnection,
  UpdateVideoMeeting,
  UpdateWebhookSubscription,
  VideoConferenceConnection,
  VideoMeeting,
  VideoMeetingQuery,
  WebhookDeliveryLog,
  WebhookStatistics,
  WebhookSubscription,
} from './types';

const API_BASE = '/api/v1/integrations';

// Helper function to build query string
function buildQueryString(params: object): string {
  const searchParams = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && value !== null) {
      searchParams.append(key, String(value));
    }
  }
  const queryString = searchParams.toString();
  return queryString ? `?${queryString}` : '';
}

// Helper for API requests
async function apiRequest<T>(url: string, options: RequestInit = {}): Promise<T> {
  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return response.json();
}

// ============================================
// Statistics
// ============================================

export async function getIntegrationStatistics(
  organizationId: string
): Promise<IntegrationStatistics> {
  return apiRequest<IntegrationStatistics>(`${API_BASE}/organizations/${organizationId}/stats`);
}

// ============================================
// Calendar Connections (Story 61.1)
// ============================================

export async function listCalendarConnections(
  organizationId: string,
  query?: CalendarQuery
): Promise<CalendarConnection[]> {
  const qs = buildQueryString(query || {});
  return apiRequest<CalendarConnection[]>(
    `${API_BASE}/organizations/${organizationId}/calendars${qs}`
  );
}

export async function createCalendarConnection(
  organizationId: string,
  data: CreateCalendarConnection
): Promise<CalendarConnection> {
  return apiRequest<CalendarConnection>(`${API_BASE}/organizations/${organizationId}/calendars`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function getCalendarConnection(id: string): Promise<CalendarConnection> {
  return apiRequest<CalendarConnection>(`${API_BASE}/calendars/${id}`);
}

export async function updateCalendarConnection(
  id: string,
  data: UpdateCalendarConnection
): Promise<CalendarConnection> {
  return apiRequest<CalendarConnection>(`${API_BASE}/calendars/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deleteCalendarConnection(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/calendars/${id}`, {
    method: 'DELETE',
  });
}

export async function syncCalendar(
  id: string,
  data: SyncCalendarRequest = {}
): Promise<CalendarSyncResult> {
  return apiRequest<CalendarSyncResult>(`${API_BASE}/calendars/${id}/sync`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function listCalendarEvents(
  connectionId: string,
  query?: CalendarEventsQuery
): Promise<CalendarEvent[]> {
  const qs = buildQueryString(query || {});
  return apiRequest<CalendarEvent[]>(`${API_BASE}/calendars/${connectionId}/events${qs}`);
}

export async function createCalendarEvent(
  connectionId: string,
  data: CreateCalendarEvent
): Promise<CalendarEvent> {
  return apiRequest<CalendarEvent>(`${API_BASE}/calendars/${connectionId}/events`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

// ============================================
// Accounting Exports (Story 61.2)
// ============================================

export async function listAccountingExports(
  organizationId: string,
  query?: AccountingExportQuery
): Promise<AccountingExport[]> {
  const qs = buildQueryString(query || {});
  return apiRequest<AccountingExport[]>(
    `${API_BASE}/organizations/${organizationId}/accounting/exports${qs}`
  );
}

export async function createAccountingExport(
  organizationId: string,
  data: CreateAccountingExport
): Promise<AccountingExport> {
  return apiRequest<AccountingExport>(
    `${API_BASE}/organizations/${organizationId}/accounting/exports`,
    {
      method: 'POST',
      body: JSON.stringify(data),
    }
  );
}

export async function getAccountingExport(id: string): Promise<AccountingExport> {
  return apiRequest<AccountingExport>(`${API_BASE}/accounting/exports/${id}`);
}

export function getAccountingExportDownloadUrl(id: string): string {
  return `${API_BASE}/accounting/exports/${id}/download`;
}

export async function getAccountingSettings(
  organizationId: string,
  systemType: string
): Promise<AccountingExportSettings> {
  return apiRequest<AccountingExportSettings>(
    `${API_BASE}/organizations/${organizationId}/accounting/settings/${systemType}`
  );
}

export async function updateAccountingSettings(
  organizationId: string,
  systemType: string,
  data: UpdateAccountingExportSettings
): Promise<AccountingExportSettings> {
  return apiRequest<AccountingExportSettings>(
    `${API_BASE}/organizations/${organizationId}/accounting/settings/${systemType}`,
    {
      method: 'PUT',
      body: JSON.stringify(data),
    }
  );
}

// ============================================
// E-Signature Workflows (Story 61.3)
// ============================================

export async function listESignatureWorkflows(
  organizationId: string,
  query?: ESignatureQuery
): Promise<ESignatureWorkflow[]> {
  const qs = buildQueryString(query || {});
  return apiRequest<ESignatureWorkflow[]>(
    `${API_BASE}/organizations/${organizationId}/esignatures${qs}`
  );
}

export async function createESignatureWorkflow(
  organizationId: string,
  data: CreateESignatureWorkflow
): Promise<ESignatureWorkflowWithRecipients> {
  return apiRequest<ESignatureWorkflowWithRecipients>(
    `${API_BASE}/organizations/${organizationId}/esignatures`,
    {
      method: 'POST',
      body: JSON.stringify(data),
    }
  );
}

export async function getESignatureWorkflow(id: string): Promise<ESignatureWorkflowWithRecipients> {
  return apiRequest<ESignatureWorkflowWithRecipients>(`${API_BASE}/esignatures/${id}`);
}

export async function sendESignatureWorkflow(id: string): Promise<ESignatureWorkflow> {
  return apiRequest<ESignatureWorkflow>(`${API_BASE}/esignatures/${id}/send`, {
    method: 'POST',
  });
}

export async function voidESignatureWorkflow(id: string): Promise<ESignatureWorkflow> {
  return apiRequest<ESignatureWorkflow>(`${API_BASE}/esignatures/${id}/void`, {
    method: 'POST',
  });
}

export async function sendESignatureReminder(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/esignatures/${id}/remind`, {
    method: 'POST',
  });
}

// ============================================
// Video Conferencing (Story 61.4)
// ============================================

export async function listVideoConnections(
  organizationId: string,
  query?: CalendarQuery
): Promise<VideoConferenceConnection[]> {
  const qs = buildQueryString(query || {});
  return apiRequest<VideoConferenceConnection[]>(
    `${API_BASE}/organizations/${organizationId}/video/connections${qs}`
  );
}

export async function createVideoConnection(
  organizationId: string,
  data: CreateVideoConferenceConnection
): Promise<VideoConferenceConnection> {
  return apiRequest<VideoConferenceConnection>(
    `${API_BASE}/organizations/${organizationId}/video/connections`,
    {
      method: 'POST',
      body: JSON.stringify(data),
    }
  );
}

export async function deleteVideoConnection(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/video/connections/${id}`, {
    method: 'DELETE',
  });
}

export async function listVideoMeetings(
  organizationId: string,
  query?: VideoMeetingQuery
): Promise<VideoMeeting[]> {
  const qs = buildQueryString(query || {});
  return apiRequest<VideoMeeting[]>(
    `${API_BASE}/organizations/${organizationId}/video/meetings${qs}`
  );
}

export async function createVideoMeeting(
  organizationId: string,
  data: CreateVideoMeeting
): Promise<VideoMeeting> {
  return apiRequest<VideoMeeting>(`${API_BASE}/organizations/${organizationId}/video/meetings`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function getVideoMeeting(id: string): Promise<VideoMeeting> {
  return apiRequest<VideoMeeting>(`${API_BASE}/video/meetings/${id}`);
}

export async function updateVideoMeeting(
  id: string,
  data: UpdateVideoMeeting
): Promise<VideoMeeting> {
  return apiRequest<VideoMeeting>(`${API_BASE}/video/meetings/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deleteVideoMeeting(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/video/meetings/${id}`, {
    method: 'DELETE',
  });
}

export async function startVideoMeeting(id: string): Promise<VideoMeeting> {
  return apiRequest<VideoMeeting>(`${API_BASE}/video/meetings/${id}/start`, {
    method: 'POST',
  });
}

// ============================================
// Webhook Subscriptions (Story 61.5)
// ============================================

export async function listWebhookSubscriptions(
  organizationId: string
): Promise<WebhookSubscription[]> {
  return apiRequest<WebhookSubscription[]>(`${API_BASE}/organizations/${organizationId}/webhooks`);
}

export async function createWebhookSubscription(
  organizationId: string,
  data: CreateWebhookSubscription
): Promise<WebhookSubscription> {
  return apiRequest<WebhookSubscription>(`${API_BASE}/organizations/${organizationId}/webhooks`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function getWebhookSubscription(id: string): Promise<WebhookSubscription> {
  return apiRequest<WebhookSubscription>(`${API_BASE}/webhooks/${id}`);
}

export async function updateWebhookSubscription(
  id: string,
  data: UpdateWebhookSubscription
): Promise<WebhookSubscription> {
  return apiRequest<WebhookSubscription>(`${API_BASE}/webhooks/${id}`, {
    method: 'PUT',
    body: JSON.stringify(data),
  });
}

export async function deleteWebhookSubscription(id: string): Promise<void> {
  return apiRequest<void>(`${API_BASE}/webhooks/${id}`, {
    method: 'DELETE',
  });
}

export async function testWebhook(
  id: string,
  data: TestWebhookRequest
): Promise<TestWebhookResponse> {
  return apiRequest<TestWebhookResponse>(`${API_BASE}/webhooks/${id}/test`, {
    method: 'POST',
    body: JSON.stringify(data),
  });
}

export async function listWebhookLogs(id: string): Promise<WebhookDeliveryLog[]> {
  return apiRequest<WebhookDeliveryLog[]>(`${API_BASE}/webhooks/${id}/logs`);
}

export async function getWebhookStatistics(id: string): Promise<WebhookStatistics> {
  return apiRequest<WebhookStatistics>(`${API_BASE}/webhooks/${id}/stats`);
}

// ============================================
// Booking.com Channel (Gap 83.2)
// ============================================

export async function getBookingStatus(organizationId: string): Promise<BookingChannelStatus> {
  return apiRequest<BookingChannelStatus>(
    `${API_BASE}/organizations/${organizationId}/booking/status`
  );
}

export async function connectBooking(
  organizationId: string,
  data: BookingConnectRequest
): Promise<BookingConnectResponse> {
  return apiRequest<BookingConnectResponse>(
    `${API_BASE}/organizations/${organizationId}/booking/connect`,
    {
      method: 'POST',
      body: JSON.stringify(data),
    }
  );
}

export async function disconnectBooking(
  organizationId: string
): Promise<BookingDisconnectResponse> {
  return apiRequest<BookingDisconnectResponse>(
    `${API_BASE}/organizations/${organizationId}/booking`,
    {
      method: 'DELETE',
    }
  );
}

export async function syncBooking(organizationId: string): Promise<BookingSyncResult> {
  return apiRequest<BookingSyncResult>(`${API_BASE}/organizations/${organizationId}/booking/sync`, {
    method: 'POST',
  });
}

export async function getBookingConflicts(organizationId: string): Promise<BookingConflictCheck> {
  return apiRequest<BookingConflictCheck>(
    `${API_BASE}/organizations/${organizationId}/booking/conflicts`
  );
}
