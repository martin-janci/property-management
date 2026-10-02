/**
 * Workflow Automation Hooks
 *
 * React Query hooks for workflow automation API (Epic 43).
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import type {
  AutomationRule,
  AutomationRulesQuery,
  AutomationTemplate,
  AutomationTemplatesQuery,
  CreateAutomationRuleInput,
  ExecutionLog,
  ExecutionLogsQuery,
  ExecutionStatsQuery,
  ExecutionStatsResponse,
  WorkflowPaginatedResponse,
} from './types';

const API_BASE = '/api/v1/automations';

// Automation Rules Hooks

export function useAutomationRules(query: AutomationRulesQuery = {}) {
  return useQuery({
    queryKey: ['automation-rules', query],
    queryFn: async (): Promise<WorkflowPaginatedResponse<AutomationRule>> => {
      const params = new URLSearchParams();
      if (query.page) params.set('page', String(query.page));
      if (query.pageSize) params.set('pageSize', String(query.pageSize));
      if (query.isEnabled !== undefined) params.set('isEnabled', String(query.isEnabled));
      if (query.triggerType) params.set('triggerType', query.triggerType);
      if (query.search) params.set('search', query.search);

      const response = await fetch(`${API_BASE}/rules?${params}`);
      if (!response.ok) throw new Error('Failed to fetch automation rules');
      return response.json();
    },
  });
}

export function useAutomationRule(id: string) {
  return useQuery({
    queryKey: ['automation-rule', id],
    queryFn: async (): Promise<AutomationRule> => {
      const response = await fetch(`${API_BASE}/rules/${id}`);
      if (!response.ok) throw new Error('Failed to fetch automation rule');
      return response.json();
    },
    enabled: !!id,
  });
}

export function useCreateAutomationRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (rule: CreateAutomationRuleInput): Promise<AutomationRule> => {
      const response = await fetch(`${API_BASE}/rules`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(rule),
      });
      if (!response.ok) throw new Error('Failed to create automation rule');
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['automation-rules'] });
    },
  });
}

export function useUpdateAutomationRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      id,
      data,
    }: {
      id: string;
      data: Partial<AutomationRule>;
    }): Promise<AutomationRule> => {
      const response = await fetch(`${API_BASE}/rules/${id}`, {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
      });
      if (!response.ok) throw new Error('Failed to update automation rule');
      return response.json();
    },
    onSuccess: (_data, variables) => {
      queryClient.invalidateQueries({ queryKey: ['automation-rules'] });
      queryClient.invalidateQueries({ queryKey: ['automation-rule', variables.id] });
    },
  });
}

export function useDeleteAutomationRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const response = await fetch(`${API_BASE}/rules/${id}`, {
        method: 'DELETE',
      });
      if (!response.ok) throw new Error('Failed to delete automation rule');
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['automation-rules'] });
    },
  });
}

export function useRunAutomationRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (id: string): Promise<ExecutionLog> => {
      const response = await fetch(`${API_BASE}/rules/${id}/run`, {
        method: 'POST',
      });
      if (!response.ok) throw new Error('Failed to run automation rule');
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['execution-logs'] });
      queryClient.invalidateQueries({ queryKey: ['execution-stats'] });
    },
  });
}

// Template Hooks

export function useAutomationTemplates(query: AutomationTemplatesQuery = {}) {
  return useQuery({
    queryKey: ['automation-templates', query],
    queryFn: async (): Promise<WorkflowPaginatedResponse<AutomationTemplate>> => {
      const params = new URLSearchParams();
      if (query.page) params.set('page', String(query.page));
      if (query.pageSize) params.set('pageSize', String(query.pageSize));
      if (query.category) params.set('category', query.category);
      if (query.search) params.set('search', query.search);

      const response = await fetch(`${API_BASE}/templates?${params}`);
      if (!response.ok) throw new Error('Failed to fetch automation templates');
      return response.json();
    },
  });
}

export function useCreateRuleFromTemplate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (templateId: string): Promise<AutomationRule> => {
      const response = await fetch(`${API_BASE}/templates/${templateId}/use`, {
        method: 'POST',
      });
      if (!response.ok) throw new Error('Failed to create rule from template');
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['automation-rules'] });
    },
  });
}

// Execution Hooks

export function useExecutionLogs(query: ExecutionLogsQuery = {}) {
  return useQuery({
    queryKey: ['execution-logs', query],
    queryFn: async (): Promise<WorkflowPaginatedResponse<ExecutionLog>> => {
      const params = new URLSearchParams();
      if (query.page) params.set('page', String(query.page));
      if (query.pageSize) params.set('pageSize', String(query.pageSize));
      if (query.status) params.set('status', query.status);
      if (query.ruleId) params.set('ruleId', query.ruleId);
      if (query.since) params.set('since', query.since);

      const response = await fetch(`${API_BASE}/executions?${params}`);
      if (!response.ok) throw new Error('Failed to fetch execution logs');
      return response.json();
    },
  });
}

export function useExecutionStats(query: ExecutionStatsQuery = {}) {
  return useQuery({
    queryKey: ['execution-stats', query],
    queryFn: async (): Promise<ExecutionStatsResponse> => {
      const params = new URLSearchParams();
      if (query.ruleId) params.set('ruleId', query.ruleId);
      if (query.since) params.set('since', query.since);

      const response = await fetch(`${API_BASE}/executions/stats?${params}`);
      if (!response.ok) throw new Error('Failed to fetch execution stats');
      return response.json();
    },
  });
}

export function useRetryExecution() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (executionId: string): Promise<ExecutionLog> => {
      const response = await fetch(`${API_BASE}/executions/${executionId}/retry`, {
        method: 'POST',
      });
      if (!response.ok) throw new Error('Failed to retry execution');
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['execution-logs'] });
      queryClient.invalidateQueries({ queryKey: ['execution-stats'] });
    },
  });
}
