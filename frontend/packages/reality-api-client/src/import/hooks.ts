/**
 * Import Hooks
 *
 * React Query hooks for property import functionality (Epic 46).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { notAvailable } from '../agency/hooks';
import { getApiBase, portalFetch } from '../config';
import type {
  ColumnMapping,
  CreateCrmConnectionRequest,
  CreateFeedSourceRequest,
  CrmConnection,
  CrmConnectionTestResult,
  CrmCredentials,
  CsvImportPreview,
  CsvImportResult,
  FeedFormat,
  FeedPreview,
  FeedSource,
  FeedStatus,
  SyncFrequency,
  SyncHistoryItem,
  SyncSchedule,
  UpdateCrmConnectionRequest,
  UpdateFeedSourceRequest,
  UpdateSyncScheduleRequest,
} from './types';

const API_PREFIX = '/api/v1';

async function fetchApi<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await portalFetch(`${getApiBase()}${API_PREFIX}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
    credentials: 'include',
  });

  if (!response.ok) {
    let errorMessage = `API error: ${response.status}`;
    try {
      const errorBody = await response.json();
      if (errorBody?.message) {
        errorMessage += ` - ${errorBody.message}`;
      }
    } catch {
      // Ignore parsing errors
    }
    throw new Error(errorMessage);
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return response.json();
}

// ---------------------------------------------------------------------------
// reality-server contract notes
//
// The server offers feed subscriptions under `/api/v1/imports/feeds` (agency is
// derived from the signed-in principal) and provider imports under
// `/api/v1/agencies/{id}/imports`. CSV upload, CRM connections, sync schedules,
// feed preview / delete and per-feed sync history have no server routes yet, so
// those hooks reject with `NotAvailableError` (status 501) without a request.
// ---------------------------------------------------------------------------

// CSV Import Hooks
export function useCsvPreview(agencyId: string, file: File | null) {
  return useQuery({
    queryKey: ['csv-preview', agencyId, file?.name],
    queryFn: () => notAvailable<CsvImportPreview>('CSV import'),
    enabled: !!agencyId && !!file,
    retry: false,
    staleTime: 0,
  });
}

export function useCsvImport(_agencyId: string) {
  return useMutation({
    mutationFn: (_args: { file: File; mapping: ColumnMapping; skipInvalid: boolean }) =>
      notAvailable<CsvImportResult>('CSV import'),
  });
}

// CRM Connection Hooks
export function useCrmConnections(agencyId: string) {
  return useQuery({
    queryKey: ['crm-connections', agencyId],
    queryFn: () => notAvailable<CrmConnection[]>('CRM connections'),
    enabled: !!agencyId,
    retry: false,
  });
}

export function useCrmConnection(agencyId: string, connectionId: string) {
  return useQuery({
    queryKey: ['crm-connection', agencyId, connectionId],
    queryFn: () => notAvailable<CrmConnection>('CRM connections'),
    enabled: !!agencyId && !!connectionId,
    retry: false,
  });
}

export function useTestCrmConnection(_agencyId: string) {
  return useMutation({
    mutationFn: (_credentials: CrmCredentials & { provider: string }) =>
      notAvailable<CrmConnectionTestResult>('CRM connections'),
  });
}

export function useCreateCrmConnection(_agencyId: string) {
  return useMutation({
    mutationFn: (_request: CreateCrmConnectionRequest) =>
      notAvailable<CrmConnection>('CRM connections'),
  });
}

export function useUpdateCrmConnection(_agencyId: string, _connectionId: string) {
  return useMutation({
    mutationFn: (_request: UpdateCrmConnectionRequest) =>
      notAvailable<CrmConnection>('CRM connections'),
  });
}

export function useDeleteCrmConnection(_agencyId: string) {
  return useMutation({
    mutationFn: (_connectionId: string) => notAvailable<void>('CRM connections'),
  });
}

export function useSyncCrmConnection(_agencyId: string, _connectionId: string) {
  return useMutation({
    mutationFn: () => notAvailable<SyncHistoryItem>('CRM connections'),
  });
}

// Sync Schedule Hooks
export function useSyncSchedule(agencyId: string, connectionId: string) {
  return useQuery({
    queryKey: ['sync-schedule', agencyId, connectionId],
    queryFn: () => notAvailable<SyncSchedule>('Sync schedules'),
    enabled: !!agencyId && !!connectionId,
    retry: false,
  });
}

export function useUpdateSyncSchedule(_agencyId: string, _connectionId: string) {
  return useMutation({
    mutationFn: (_request: UpdateSyncScheduleRequest) =>
      notAvailable<SyncSchedule>('Sync schedules'),
  });
}

export function useSyncHistory(agencyId: string, connectionId: string, limit = 10) {
  return useQuery({
    queryKey: ['sync-history', agencyId, connectionId, limit],
    queryFn: () => notAvailable<SyncHistoryItem[]>('Sync history'),
    enabled: !!agencyId && !!connectionId,
    retry: false,
  });
}

// Feed Source Hooks — `/api/v1/imports/feeds` (db `FeedSubscription`).

/** Wire shape of a reality-server feed subscription (snake_case). */
interface WireFeedSubscription {
  id: string;
  agency_id: string;
  name: string;
  feed_url: string;
  feed_type: string;
  sync_interval: string;
  is_active: boolean;
  last_sync_at?: string | null;
  next_sync_at?: string | null;
  last_error?: string | null;
  consecutive_failures: number;
  total_imported: number;
  created_at: string;
  updated_at: string;
}

const FEED_FORMATS: FeedFormat[] = ['xml', 'rss', 'atom', 'json'];
const SYNC_FREQUENCIES: SyncFrequency[] = ['manual', 'hourly', 'daily', 'weekly'];

/** Map a server feed subscription onto the UI's `FeedSource`. */
export function toFeedSource(w: WireFeedSubscription): FeedSource {
  const format = (FEED_FORMATS as string[]).includes(w.feed_type)
    ? (w.feed_type as FeedFormat)
    : 'xml';
  const syncFrequency = (SYNC_FREQUENCIES as string[]).includes(w.sync_interval)
    ? (w.sync_interval as SyncFrequency)
    : 'daily';
  const status: FeedStatus = !w.is_active
    ? 'paused'
    : w.last_error || w.consecutive_failures > 0
      ? 'error'
      : 'active';
  return {
    id: w.id,
    agencyId: w.agency_id,
    name: w.name,
    url: w.feed_url,
    format,
    status,
    lastFetchAt: w.last_sync_at ?? undefined,
    nextFetchAt: w.next_sync_at ?? undefined,
    syncFrequency,
    // The server stores no per-feed field mapping.
    fieldMapping: {},
    totalListings: w.total_imported,
    createdAt: w.created_at,
    updatedAt: w.updated_at,
  };
}

export function useFeedSources(agencyId: string) {
  return useQuery({
    queryKey: ['feed-sources', agencyId],
    queryFn: async () => {
      const { feeds } = await fetchApi<{ feeds: WireFeedSubscription[]; total: number }>(
        '/imports/feeds'
      );
      return feeds.map(toFeedSource);
    },
    enabled: !!agencyId,
  });
}

export function useFeedSource(agencyId: string, feedId: string) {
  return useQuery({
    queryKey: ['feed-source', agencyId, feedId],
    queryFn: async () => {
      const { feed } = await fetchApi<{ feed: WireFeedSubscription }>(
        `/imports/feeds/${encodeURIComponent(feedId)}`
      );
      return toFeedSource(feed);
    },
    enabled: !!agencyId && !!feedId,
  });
}

export function useFeedPreview(_agencyId: string) {
  return useMutation({
    mutationFn: (_url: string) => notAvailable<FeedPreview>('Feed preview'),
  });
}

export function useCreateFeedSource(agencyId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: CreateFeedSourceRequest) => {
      const { feed } = await fetchApi<{ feed: WireFeedSubscription }>('/imports/feeds', {
        method: 'POST',
        body: JSON.stringify({
          name: request.name,
          feed_url: request.url,
          feed_type: request.format,
          sync_interval: request.syncFrequency,
        }),
      });
      return toFeedSource(feed);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['feed-sources', agencyId] });
    },
  });
}

export function useUpdateFeedSource(agencyId: string, feedId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (request: UpdateFeedSourceRequest) => {
      const { feed } = await fetchApi<{ feed: WireFeedSubscription }>(
        `/imports/feeds/${encodeURIComponent(feedId)}`,
        {
          method: 'PUT',
          body: JSON.stringify({
            name: request.name,
            feed_url: request.url,
            sync_interval: request.syncFrequency,
            is_active: request.status === undefined ? undefined : request.status !== 'paused',
          }),
        }
      );
      return toFeedSource(feed);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['feed-sources', agencyId] });
      queryClient.invalidateQueries({ queryKey: ['feed-source', agencyId, feedId] });
    },
  });
}

export function useDeleteFeedSource(agencyId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (feedId: string) =>
      fetchApi<void>(`/imports/feeds/${encodeURIComponent(feedId)}`, { method: 'DELETE' }),
    onSuccess: (_data, feedId) => {
      queryClient.invalidateQueries({ queryKey: ['feed-sources', agencyId] });
      queryClient.removeQueries({ queryKey: ['feed-source', agencyId, feedId] });
    },
  });
}

export function useSyncFeedSource(agencyId: string, feedId: string) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (): Promise<SyncHistoryItem> => {
      // `POST /imports/feeds/{id}/sync` schedules the sync and returns the feed.
      const { feed } = await fetchApi<{ feed: WireFeedSubscription }>(
        `/imports/feeds/${encodeURIComponent(feedId)}/sync`,
        { method: 'POST' }
      );
      return {
        id: feed.id,
        connectionId: feed.id,
        startedAt: new Date().toISOString(),
        status: 'running',
        recordsProcessed: 0,
        recordsCreated: 0,
        recordsUpdated: 0,
        recordsFailed: 0,
      };
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['feed-sources', agencyId] });
      queryClient.invalidateQueries({ queryKey: ['feed-source', agencyId, feedId] });
    },
  });
}

export function useFeedSyncHistory(agencyId: string, feedId: string, limit = 10) {
  return useQuery({
    queryKey: ['feed-sync-history', agencyId, feedId, limit],
    queryFn: () => notAvailable<SyncHistoryItem[]>('Feed sync history'),
    enabled: !!agencyId && !!feedId,
    retry: false,
  });
}
