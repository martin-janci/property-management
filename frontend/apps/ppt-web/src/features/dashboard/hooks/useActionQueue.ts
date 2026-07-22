/**
 * Hook for aggregating action items from multiple sources into a prioritized queue.
 * Used by manager and resident dashboards to show items needing attention.
 *
 * @module features/dashboard/hooks/useActionQueue
 */

import { getMode, parseMode } from '@ppt/dev-panel';
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

export interface ActionQueueData {
  items: ActionItem[];
  total: number;
  counts: {
    urgent: number;
    high: number;
    medium: number;
    low: number;
  };
}

/**
 * Dev-only mock gate.
 *
 * There is no backend action-queue endpoint yet (no `/api/v1/action-queue`
 * route in api-server, no TypeSpec contract). To avoid shipping fabricated
 * action items to production users — and silent no-op approve/reject/dismiss
 * mutations — the mock data + simulated mutations are served ONLY when the app
 * runs in DEV under the dev-panel `mock` API mode. In every other case (prod
 * builds, `local`/`worktree` modes) the queue resolves empty and mutations
 * fail loudly rather than pretending to succeed.
 *
 * Track the backend gap: see PR body "API gap" section.
 */
export function isActionQueueMockEnabled(): boolean {
  return import.meta.env.DEV && getMode(parseMode(import.meta.env.VITE_API_DEFAULT)) === 'mock';
}

/** Empty queue returned when the mock is disabled and no real backend exists. */
function emptyQueue(): ActionQueueData {
  return { items: [], total: 0, counts: { urgent: 0, high: 0, medium: 0, low: 0 } };
}

/** Apply client-side filters to an action-queue payload. Exported for testing. */
export function applyActionQueueFilters(
  data: ActionQueueData,
  filters?: ActionQueueFilters
): ActionQueueData {
  let filteredItems = data.items;

  if (filters?.types?.length) {
    filteredItems = filteredItems.filter((item) => filters.types!.includes(item.type));
  }

  if (filters?.priorities?.length) {
    filteredItems = filteredItems.filter((item) => filters.priorities!.includes(item.priority));
  }

  if (filters?.search) {
    const searchLower = filters.search.toLowerCase();
    filteredItems = filteredItems.filter(
      (item) =>
        item.title.toLowerCase().includes(searchLower) ||
        item.description.toLowerCase().includes(searchLower)
    );
  }

  return {
    items: filteredItems,
    total: filteredItems.length,
    counts: data.counts,
  };
}

// Mock data generator — DEV-only (see isActionQueueMockEnabled).
function generateMockData(role: 'manager' | 'resident'): ActionQueueData {
  const now = new Date();
  const items: ActionItem[] = [];

  if (role === 'manager') {
    // Pending faults
    items.push({
      id: 'fault-1',
      type: 'fault_pending',
      title: 'Water leak in Unit 301',
      description: 'Resident reports water leak from bathroom ceiling. Photos attached.',
      priority: 'urgent',
      createdAt: new Date(now.getTime() - 2 * 60 * 60 * 1000).toISOString(),
      entityId: 'f-001',
      entityType: 'fault',
      actions: [
        { id: 'view', label: 'View Details', variant: 'secondary', action: 'view' },
        { id: 'approve', label: 'Assign Contractor', variant: 'primary', action: 'approve' },
        { id: 'escalate', label: 'Escalate', variant: 'danger', action: 'escalate' },
      ],
    });

    items.push({
      id: 'fault-2',
      type: 'fault_pending',
      title: 'Elevator malfunction',
      description: 'Elevator B stuck on floor 5. Technician contacted.',
      priority: 'high',
      createdAt: new Date(now.getTime() - 4 * 60 * 60 * 1000).toISOString(),
      entityId: 'f-002',
      entityType: 'fault',
      actions: [
        { id: 'view', label: 'View Details', variant: 'secondary', action: 'view' },
        { id: 'approve', label: 'Confirm Resolution', variant: 'primary', action: 'approve' },
      ],
    });

    // Approval pending
    items.push({
      id: 'approval-1',
      type: 'approval_pending',
      title: 'Budget Approval: HVAC Replacement',
      description: 'Building C HVAC system replacement. Quote: €45,000',
      priority: 'high',
      dueDate: new Date(now.getTime() + 3 * 24 * 60 * 60 * 1000).toISOString(),
      createdAt: new Date(now.getTime() - 24 * 60 * 60 * 1000).toISOString(),
      entityId: 'ba-001',
      entityType: 'budget_approval',
      metadata: { amount: 45000, currency: 'EUR' },
      actions: [
        { id: 'view', label: 'View Details', variant: 'secondary', action: 'view' },
        { id: 'approve', label: 'Approve', variant: 'primary', action: 'approve' },
        { id: 'reject', label: 'Reject', variant: 'danger', action: 'reject' },
      ],
    });

    // Active votes
    items.push({
      id: 'vote-1',
      type: 'vote_active',
      title: 'Vote: Garden Renovation',
      description: '15 of 24 residents have voted. 3 days remaining.',
      priority: 'medium',
      dueDate: new Date(now.getTime() + 3 * 24 * 60 * 60 * 1000).toISOString(),
      createdAt: new Date(now.getTime() - 4 * 24 * 60 * 60 * 1000).toISOString(),
      entityId: 'v-001',
      entityType: 'vote',
      metadata: { voted: 15, total: 24 },
      actions: [
        { id: 'view', label: 'View Results', variant: 'secondary', action: 'view' },
        { id: 'complete', label: 'Close Early', variant: 'primary', action: 'complete' },
      ],
    });

    // Unread messages
    items.push({
      id: 'message-1',
      type: 'message_unread',
      title: 'New message from Unit 105',
      description: 'Question about parking space assignment',
      priority: 'low',
      createdAt: new Date(now.getTime() - 30 * 60 * 1000).toISOString(),
      entityId: 'm-001',
      entityType: 'message',
      actions: [
        { id: 'view', label: 'View', variant: 'primary', action: 'view' },
        { id: 'dismiss', label: 'Mark Read', variant: 'secondary', action: 'dismiss' },
      ],
    });
  } else {
    // Resident actions
    items.push({
      id: 'vote-r1',
      type: 'vote_active',
      title: 'Vote Required: Garden Renovation',
      description: 'Cast your vote for the proposed garden renovation project.',
      priority: 'high',
      dueDate: new Date(now.getTime() + 3 * 24 * 60 * 60 * 1000).toISOString(),
      createdAt: new Date(now.getTime() - 4 * 24 * 60 * 60 * 1000).toISOString(),
      entityId: 'v-001',
      entityType: 'vote',
      actions: [{ id: 'view', label: 'Cast Vote', variant: 'primary', action: 'view' }],
    });

    items.push({
      id: 'meter-1',
      type: 'meter_due',
      title: 'Submit Meter Reading',
      description: 'Water and electricity meter readings due by end of month.',
      priority: 'medium',
      dueDate: new Date(now.getTime() + 5 * 24 * 60 * 60 * 1000).toISOString(),
      createdAt: new Date(now.getTime() - 2 * 24 * 60 * 60 * 1000).toISOString(),
      entityId: 'mr-001',
      entityType: 'meter_reading',
      actions: [
        { id: 'complete', label: 'Submit Reading', variant: 'primary', action: 'complete' },
      ],
    });

    items.push({
      id: 'pm-1',
      type: 'person_months_due',
      title: 'Person-Months Declaration',
      description: 'Quarterly declaration for utility cost allocation.',
      priority: 'high',
      dueDate: new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000).toISOString(),
      createdAt: new Date(now.getTime() - 1 * 24 * 60 * 60 * 1000).toISOString(),
      entityId: 'pm-001',
      entityType: 'person_months',
      actions: [
        { id: 'complete', label: 'Submit Declaration', variant: 'primary', action: 'complete' },
      ],
    });

    items.push({
      id: 'announcement-1',
      type: 'announcement_unread',
      title: 'New Building Announcement',
      description: 'Important update about scheduled maintenance.',
      priority: 'low',
      createdAt: new Date(now.getTime() - 6 * 60 * 60 * 1000).toISOString(),
      entityId: 'a-001',
      entityType: 'announcement',
      actions: [
        { id: 'view', label: 'Read', variant: 'primary', action: 'view' },
        { id: 'dismiss', label: 'Dismiss', variant: 'secondary', action: 'dismiss' },
      ],
    });
  }

  // Sort by priority
  const priorityOrder: Record<ActionPriority, number> = {
    urgent: 0,
    high: 1,
    medium: 2,
    low: 3,
  };
  items.sort((a, b) => priorityOrder[a.priority] - priorityOrder[b.priority]);

  const counts = items.reduce(
    (acc, item) => {
      acc[item.priority]++;
      return acc;
    },
    { urgent: 0, high: 0, medium: 0, low: 0 }
  );

  return { items, total: items.length, counts };
}

/**
 * Hook for fetching and managing the action queue.
 */
export function useActionQueue(role: 'manager' | 'resident', filters?: ActionQueueFilters) {
  const queryClient = useQueryClient();

  // Fetch action queue data
  const query = useQuery({
    queryKey: ['actionQueue', role, filters],
    queryFn: async (): Promise<ActionQueueData> => {
      // No real backend endpoint exists yet (see isActionQueueMockEnabled).
      // Outside DEV mock mode we return an empty queue so production users
      // never see fabricated action items — the ActionQueue component then
      // renders its "all caught up" empty state.
      if (!isActionQueueMockEnabled()) {
        return emptyQueue();
      }

      // TODO(backend): replace with real call once `/api/v1/action-queue`
      // (and the TypeSpec contract + generated client hook) exist:
      //   const response = await client.actionQueue.list({ role });
      //   return response.data;
      await new Promise((resolve) => setTimeout(resolve, 300)); // Simulate network delay
      return applyActionQueueFilters(generateMockData(role), filters);
    },
    staleTime: 30000, // 30 seconds
    refetchInterval: 60000, // 1 minute
  });

  // Mutation for executing an action
  const executeAction = useMutation({
    mutationFn: async ({ itemId, action }: { itemId: string; action: ActionButton['action'] }) => {
      // Fail loudly outside DEV mock mode instead of returning a fake success:
      // approve/reject/dismiss must not be silent no-ops in production while the
      // backend endpoint is missing. The surfaced error drives the caller's
      // error handling (mutation `isError`) rather than a lie.
      if (!isActionQueueMockEnabled()) {
        throw new Error('Action queue backend is not implemented yet (no /api/v1/action-queue).');
      }

      // TODO(backend): replace with real call once the endpoint exists:
      //   await client.actionQueue.execute({ itemId, action });
      await new Promise((resolve) => setTimeout(resolve, 500)); // Simulate network delay
      return { success: true, itemId, action };
    },
    onSuccess: () => {
      // Invalidate the query to refetch
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
    isExecuteError: executeAction.isError,
    executeError: executeAction.error,
  };
}
