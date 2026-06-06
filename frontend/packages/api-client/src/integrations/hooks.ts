/**
 * External Integrations React Query Hooks
 *
 * React Query hooks for External Integrations API (Epic 61).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import * as api from './api';
import type {
  AccountingExportQuery,
  AccountingSystem,
  AirbnbConnectRequest,
  BookingConnectRequest,
  CalendarEventsQuery,
  CalendarQuery,
  CreateAccountingExport,
  CreateCalendarConnection,
  CreateCalendarEvent,
  CreateESignatureWorkflow,
  CreateVideoConferenceConnection,
  CreateVideoMeeting,
  CreateWebhookSubscription,
  ESignatureQuery,
  SyncCalendarRequest,
  TestWebhookRequest,
  UpdateAccountingExportSettings,
  UpdateCalendarConnection,
  UpdateVideoMeeting,
  UpdateWebhookSubscription,
  VideoMeetingQuery,
} from './types';

// Query keys
export const integrationKeys = {
  all: ['integrations'] as const,
  statistics: (orgId: string) => [...integrationKeys.all, 'statistics', orgId] as const,
  // Calendar
  calendars: (orgId: string) => [...integrationKeys.all, 'calendars', orgId] as const,
  calendar: (id: string) => [...integrationKeys.all, 'calendar', id] as const,
  calendarEvents: (connectionId: string) =>
    [...integrationKeys.all, 'calendar-events', connectionId] as const,
  // Accounting
  accountingExports: (orgId: string) =>
    [...integrationKeys.all, 'accounting-exports', orgId] as const,
  accountingExport: (id: string) => [...integrationKeys.all, 'accounting-export', id] as const,
  accountingSettings: (orgId: string, systemType: string) =>
    [...integrationKeys.all, 'accounting-settings', orgId, systemType] as const,
  // E-Signature
  esignatures: (orgId: string) => [...integrationKeys.all, 'esignatures', orgId] as const,
  esignature: (id: string) => [...integrationKeys.all, 'esignature', id] as const,
  // Video
  videoConnections: (orgId: string) =>
    [...integrationKeys.all, 'video-connections', orgId] as const,
  videoMeetings: (orgId: string) => [...integrationKeys.all, 'video-meetings', orgId] as const,
  videoMeeting: (id: string) => [...integrationKeys.all, 'video-meeting', id] as const,
  // Webhooks
  webhooks: (orgId: string) => [...integrationKeys.all, 'webhooks', orgId] as const,
  webhook: (id: string) => [...integrationKeys.all, 'webhook', id] as const,
  webhookLogs: (id: string) => [...integrationKeys.all, 'webhook-logs', id] as const,
  webhookStats: (id: string) => [...integrationKeys.all, 'webhook-stats', id] as const,
  // Airbnb (Gap 83-1)
  airbnbStatus: (orgId: string) => [...integrationKeys.all, 'airbnb-status', orgId] as const,
  airbnbListings: (orgId: string) => [...integrationKeys.all, 'airbnb-listings', orgId] as const,
  airbnbReservations: (orgId: string) =>
    [...integrationKeys.all, 'airbnb-reservations', orgId] as const,
  // Booking.com (Gap 83.2)
  bookingStatus: (orgId: string) => [...integrationKeys.all, 'booking-status', orgId] as const,
  bookingConflicts: (orgId: string) =>
    [...integrationKeys.all, 'booking-conflicts', orgId] as const,
};

// ============================================
// Statistics
// ============================================

export function useIntegrationStatistics(organizationId: string) {
  return useQuery({
    queryKey: integrationKeys.statistics(organizationId),
    queryFn: () => api.getIntegrationStatistics(organizationId),
  });
}

// ============================================
// Calendar Connections (Story 61.1)
// ============================================

export function useCalendarConnections(organizationId: string, query?: CalendarQuery) {
  return useQuery({
    queryKey: [...integrationKeys.calendars(organizationId), query],
    queryFn: () => api.listCalendarConnections(organizationId, query),
  });
}

export function useCalendarConnection(id: string) {
  return useQuery({
    queryKey: integrationKeys.calendar(id),
    queryFn: () => api.getCalendarConnection(id),
    enabled: !!id,
  });
}

export function useCreateCalendarConnection(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateCalendarConnection) =>
      api.createCalendarConnection(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.calendars(organizationId),
      });
    },
  });
}

export function useUpdateCalendarConnection(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateCalendarConnection }) =>
      api.updateCalendarConnection(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.calendars(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.calendar(id),
      });
    },
  });
}

export function useDeleteCalendarConnection(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteCalendarConnection(id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.calendars(organizationId),
      });
    },
  });
}

export function useSyncCalendar(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data?: SyncCalendarRequest }) =>
      api.syncCalendar(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.calendar(id),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.calendarEvents(id),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.statistics(organizationId),
      });
    },
  });
}

export function useCalendarEvents(connectionId: string, query?: CalendarEventsQuery) {
  return useQuery({
    queryKey: [...integrationKeys.calendarEvents(connectionId), query],
    queryFn: () => api.listCalendarEvents(connectionId, query),
    enabled: !!connectionId,
  });
}

export function useCreateCalendarEvent(connectionId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateCalendarEvent) => api.createCalendarEvent(connectionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.calendarEvents(connectionId),
      });
    },
  });
}

// ============================================
// Accounting Exports (Story 61.2)
// ============================================

export function useAccountingExports(organizationId: string, query?: AccountingExportQuery) {
  return useQuery({
    queryKey: [...integrationKeys.accountingExports(organizationId), query],
    queryFn: () => api.listAccountingExports(organizationId, query),
  });
}

export function useAccountingExport(id: string) {
  return useQuery({
    queryKey: integrationKeys.accountingExport(id),
    queryFn: () => api.getAccountingExport(id),
    enabled: !!id,
  });
}

export function useCreateAccountingExport(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateAccountingExport) => api.createAccountingExport(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.accountingExports(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.statistics(organizationId),
      });
    },
  });
}

export function useAccountingSettings(organizationId: string, systemType: AccountingSystem) {
  return useQuery({
    queryKey: integrationKeys.accountingSettings(organizationId, systemType),
    queryFn: () => api.getAccountingSettings(organizationId, systemType),
  });
}

export function useUpdateAccountingSettings(organizationId: string, systemType: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateAccountingExportSettings) =>
      api.updateAccountingSettings(organizationId, systemType, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.accountingSettings(organizationId, systemType),
      });
    },
  });
}

// ============================================
// E-Signature Workflows (Story 61.3)
// ============================================

export function useESignatureWorkflows(organizationId: string, query?: ESignatureQuery) {
  return useQuery({
    queryKey: [...integrationKeys.esignatures(organizationId), query],
    queryFn: () => api.listESignatureWorkflows(organizationId, query),
  });
}

export function useESignatureWorkflow(id: string) {
  return useQuery({
    queryKey: integrationKeys.esignature(id),
    queryFn: () => api.getESignatureWorkflow(id),
    enabled: !!id,
  });
}

export function useCreateESignatureWorkflow(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateESignatureWorkflow) =>
      api.createESignatureWorkflow(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.esignatures(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.statistics(organizationId),
      });
    },
  });
}

export function useSendESignatureWorkflow(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.sendESignatureWorkflow(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.esignatures(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.esignature(id),
      });
    },
  });
}

export function useVoidESignatureWorkflow(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.voidESignatureWorkflow(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.esignatures(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.esignature(id),
      });
    },
  });
}

export function useSendESignatureReminder() {
  return useMutation({
    mutationFn: (id: string) => api.sendESignatureReminder(id),
  });
}

// ============================================
// Video Conferencing (Story 61.4)
// ============================================

export function useVideoConnections(organizationId: string, query?: CalendarQuery) {
  return useQuery({
    queryKey: [...integrationKeys.videoConnections(organizationId), query],
    queryFn: () => api.listVideoConnections(organizationId, query),
  });
}

export function useCreateVideoConnection(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateVideoConferenceConnection) =>
      api.createVideoConnection(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.videoConnections(organizationId),
      });
    },
  });
}

export function useDeleteVideoConnection(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteVideoConnection(id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.videoConnections(organizationId),
      });
    },
  });
}

export function useVideoMeetings(organizationId: string, query?: VideoMeetingQuery) {
  return useQuery({
    queryKey: [...integrationKeys.videoMeetings(organizationId), query],
    queryFn: () => api.listVideoMeetings(organizationId, query),
  });
}

export function useVideoMeeting(id: string) {
  return useQuery({
    queryKey: integrationKeys.videoMeeting(id),
    queryFn: () => api.getVideoMeeting(id),
    enabled: !!id,
  });
}

export function useCreateVideoMeeting(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateVideoMeeting) => api.createVideoMeeting(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.videoMeetings(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.statistics(organizationId),
      });
    },
  });
}

export function useUpdateVideoMeeting(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateVideoMeeting }) =>
      api.updateVideoMeeting(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.videoMeetings(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.videoMeeting(id),
      });
    },
  });
}

export function useDeleteVideoMeeting(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteVideoMeeting(id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.videoMeetings(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.statistics(organizationId),
      });
    },
  });
}

export function useStartVideoMeeting(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.startVideoMeeting(id),
    onSuccess: (_, id) => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.videoMeetings(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.videoMeeting(id),
      });
    },
  });
}

// ============================================
// Webhook Subscriptions (Story 61.5)
// ============================================

export function useWebhookSubscriptions(organizationId: string) {
  return useQuery({
    queryKey: integrationKeys.webhooks(organizationId),
    queryFn: () => api.listWebhookSubscriptions(organizationId),
  });
}

export function useWebhookSubscription(id: string) {
  return useQuery({
    queryKey: integrationKeys.webhook(id),
    queryFn: () => api.getWebhookSubscription(id),
    enabled: !!id,
  });
}

export function useCreateWebhookSubscription(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateWebhookSubscription) =>
      api.createWebhookSubscription(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.webhooks(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.statistics(organizationId),
      });
    },
  });
}

export function useUpdateWebhookSubscription(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateWebhookSubscription }) =>
      api.updateWebhookSubscription(id, data),
    onSuccess: (_, { id }) => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.webhooks(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.webhook(id),
      });
    },
  });
}

export function useDeleteWebhookSubscription(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteWebhookSubscription(id),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.webhooks(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.statistics(organizationId),
      });
    },
  });
}

export function useTestWebhook() {
  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: TestWebhookRequest }) =>
      api.testWebhook(id, data),
  });
}

export function useWebhookLogs(id: string) {
  return useQuery({
    queryKey: integrationKeys.webhookLogs(id),
    queryFn: () => api.listWebhookLogs(id),
    enabled: !!id,
  });
}

export function useWebhookStatistics(id: string) {
  return useQuery({
    queryKey: integrationKeys.webhookStats(id),
    queryFn: () => api.getWebhookStatistics(id),
    enabled: !!id,
  });
}

// ============================================
// Airbnb Integration (Gap 83-1 / Story 83.1)
// ============================================

export function useAirbnbStatus(organizationId: string) {
  return useQuery({
    queryKey: integrationKeys.airbnbStatus(organizationId),
    queryFn: () => api.getAirbnbStatus(organizationId),
    enabled: !!organizationId,
  });
}

export function useAirbnbListingMappings(organizationId: string) {
  return useQuery({
    queryKey: integrationKeys.airbnbListings(organizationId),
    queryFn: () => api.listAirbnbListingMappings(),
    enabled: !!organizationId,
  });
}

export function useAirbnbReservations(organizationId: string, limit = 50) {
  return useQuery({
    queryKey: [...integrationKeys.airbnbReservations(organizationId), limit],
    queryFn: () => api.listAirbnbReservations(limit),
    enabled: !!organizationId,
  });
}

/**
 * Initiate the Airbnb OAuth connect flow. On success the backend returns an
 * `auth_url` the caller should redirect the browser to.
 */
export function useConnectAirbnb(organizationId: string) {
  return useMutation({
    mutationFn: (data?: AirbnbConnectRequest) => api.connectAirbnb(organizationId, data),
  });
}

export function useSyncAirbnb(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => api.syncAirbnb(organizationId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: integrationKeys.airbnbStatus(organizationId) });
      queryClient.invalidateQueries({ queryKey: integrationKeys.airbnbListings(organizationId) });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.airbnbReservations(organizationId),
      });
    },
  });
}

export function useDisconnectAirbnb(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => api.disconnectAirbnb(organizationId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: integrationKeys.airbnbStatus(organizationId) });
      queryClient.invalidateQueries({ queryKey: integrationKeys.airbnbListings(organizationId) });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.airbnbReservations(organizationId),
      });
    },
  });
}

// ============================================
// Booking.com Channel (Gap 83.2)
// ============================================

export function useBookingStatus(organizationId: string) {
  return useQuery({
    queryKey: integrationKeys.bookingStatus(organizationId),
    queryFn: () => api.getBookingStatus(organizationId),
    enabled: !!organizationId,
  });
}

export function useConnectBooking(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: BookingConnectRequest) => api.connectBooking(organizationId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.bookingStatus(organizationId),
      });
      queryClient.invalidateQueries({ queryKey: integrationKeys.statistics(organizationId) });
    },
  });
}

export function useDisconnectBooking(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => api.disconnectBooking(organizationId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.bookingStatus(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.bookingConflicts(organizationId),
      });
      queryClient.invalidateQueries({ queryKey: integrationKeys.statistics(organizationId) });
    },
  });
}

export function useSyncBooking(organizationId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => api.syncBooking(organizationId),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: integrationKeys.bookingStatus(organizationId),
      });
      queryClient.invalidateQueries({
        queryKey: integrationKeys.bookingConflicts(organizationId),
      });
    },
  });
}

export function useBookingConflicts(organizationId: string, enabled = true) {
  return useQuery({
    queryKey: integrationKeys.bookingConflicts(organizationId),
    queryFn: () => api.getBookingConflicts(organizationId),
    enabled: !!organizationId && enabled,
  });
}
