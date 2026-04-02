/**
 * Hook for aggregating action items from multiple sources into a prioritized queue.
 * Used by manager and resident dashboards to show items needing attention.
 *
 * @module features/dashboard/hooks/useActionQueue
 */

import { getToken } from '@ppt/api-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useCallback, useMemo } from 'react';

// Action item types
export type ActionType =
  | 'fault_pending'
  | 'fault_escalated'
  | 'approval_pending'
  | 'vote_active'
  | 'message_unread'
  | 'meter_due'
  | 'person_months_due'
  | 'announcement_unread';

export type ActionPriority = 'urgent' | 'high' | 'medium' | 'low';

export interface ActionItem {
  id: string;
  type: ActionType;
  title: string;
  description: string;
  priority: ActionPriority;
  dueDate?: string;
  createdAt: string;
  entityId: string;
  entityType: string;
  metadata?: Record<string, unknown>;
  actions: ActionButton[];
}

export interface ActionButton {
  id: string;
  label: string;
  variant: 'primary' | 'secondary' | 'danger';
  action: 'approve' | 'reject' | 'view' | 'dismiss' | 'complete' | 'escalate';
}

export interface ActionQueueFilters {
  types?: ActionType[];
  priorities?: ActionPriority[];
  search?: string;
}

interface ActionQueueData {
  items: ActionItem[];
  total: number;
  counts: {
    urgent: number;
    high: number;
    medium: number;
    low: number;
  };
}

function getAuthHeaders(): HeadersInit {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function fetchActionQueue(
  role: 'manager' | 'resident',
  filters?: ActionQueueFilters
): Promise<ActionQueueData> {
  const params = new URLSearchParams({ role });
  if (filters?.types?.length) {
    for (const t of filters.types) params.append('type', t);
  }
  if (filters?.priorities?.length) {
    for (const p of filters.priorities) params.append('priority', p);
  }
  if (filters?.search) {
    params.set('search', filters.search);
  }

  const response = await fetch(`/api/v1/action-queue?${params.toString()}`, {
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error((error as { message?: string }).message || `HTTP ${response.status}`);
  }

  return response.json();
}

async function executeActionApi(
  itemId: string,
  action: string
): Promise<{ success: boolean; itemId: string; action: string }> {
  const response = await fetch(`/api/v1/action-queue/${itemId}/execute`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      ...getAuthHeaders(),
    },
    body: JSON.stringify({ action }),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({}));
    throw new Error((error as { message?: string }).message || `HTTP ${response.status}`);
  }

  return response.json();
}

/**
 * Hook for fetching and managing the action queue.
 */
export function useActionQueue(role: 'manager' | 'resident', filters?: ActionQueueFilters) {
  const queryClient = useQueryClient();

  // Fetch action queue data from API
  const query = useQuery({
    queryKey: ['actionQueue', role, filters],
    queryFn: () => fetchActionQueue(role, filters),
    staleTime: 30000, // 30 seconds
    refetchInterval: 60000, // 1 minute
  });

  // Mutation for executing an action
  const executeAction = useMutation({
    mutationFn: async ({
      itemId,
      action,
    }: {
      itemId: string;
      action: ActionButton['action'];
    }) => executeActionApi(itemId, action),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['actionQueue', role] });
    },
  });

  // Dismiss an item (mark as handled)
  const dismissItem = useCallback(
    (itemId: string) => {
      executeAction.mutate({ itemId, action: 'dismiss' });
    },
    [executeAction]
  );

  // Execute a specific action
  const handleAction = useCallback(
    (itemId: string, action: ActionButton['action']) => {
      executeAction.mutate({ itemId, action });
    },
    [executeAction]
  );

  // Computed values
  const stats = useMemo(() => {
    if (!query.data) {
      return { total: 0, urgent: 0, high: 0, medium: 0, low: 0 };
    }
    return {
      total: query.data.total,
      ...query.data.counts,
    };
  }, [query.data]);

  return {
    items: query.data?.items ?? [],
    total: query.data?.total ?? 0,
    stats,
    isLoading: query.isLoading,
    isError: query.isError,
    error: query.error,
    refetch: query.refetch,
    dismissItem,
    handleAction,
    isExecuting: executeAction.isPending,
  };
}
