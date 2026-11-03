/**
 * Workflow Automation Hooks
 *
 * React Query hooks for the workflow automation API. The backend router is
 * mounted at `/api/v1/automation` (singular — routes/automation.rs); list and
 * create endpoints are organization-scoped, so the active org id comes from
 * the registered org provider (the same value sent as `X-Tenant-ID`).
 *
 * Wire shapes are snake_case; `./mappers` translates them into the camelCase
 * view model in `./types`. Rules and templates are returned as full arrays, so
 * filtering/search/pagination for those happens client-side.
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { getOrg } from '../auth/org-provider';
import {
  type ApiAutomationExecutionList,
  type ApiAutomationExecutionStats,
  type ApiAutomationRule,
  type ApiAutomationTemplate,
  executionFromApi,
  executionStatsFromApi,
  executionStatusToApi,
  matchesSearch,
  paginate,
  ruleFromApi,
  ruleToApi,
  templateFromApi,
} from './mappers';
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

const API_BASE = '/api/v1/automation';

async function requestJson<T>(
  url: string,
  init: RequestInit | undefined,
  failure: string
): Promise<T> {
  const response = await fetch(url, {
    ...init,
    headers: {
      ...(init?.body ? { 'Content-Type': 'application/json' } : {}),
      ...init?.headers,
    },
  });
  if (!response.ok) {
    const body = (await response.json().catch(() => null)) as { message?: string } | null;
    throw new Error(body?.message || failure);
  }
  const text = await response.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

function requireOrg(): string {
  const orgId = getOrg();
  if (!orgId) throw new Error('No active organization');
  return orgId;
}

// Automation Rules Hooks

export function useAutomationRules(query: AutomationRulesQuery = {}) {
  const orgId = getOrg();
  return useQuery({
    queryKey: ['automation-rules', orgId, query],
    enabled: !!orgId,
    queryFn: async (): Promise<WorkflowPaginatedResponse<AutomationRule>> => {
      const rows = await requestJson<ApiAutomationRule[]>(
        `${API_BASE}/organizations/${requireOrg()}/rules`,
        undefined,
        'Failed to fetch automation rules'
      );
      const filtered = rows
        .map(ruleFromApi)
        .filter(
          (rule) =>
            (query.isEnabled === undefined || rule.isEnabled === query.isEnabled) &&
            (!query.triggerType || rule.trigger.type === query.triggerType) &&
            matchesSearch(query.search, rule.name, rule.description)
        );
      return paginate(filtered, query.page, query.pageSize);
    },
  });
}

export function useAutomationRule(id: string) {
  return useQuery({
    queryKey: ['automation-rule', id],
    queryFn: async (): Promise<AutomationRule> =>
      ruleFromApi(
        await requestJson<ApiAutomationRule>(
          `${API_BASE}/rules/${id}`,
          undefined,
          'Failed to fetch automation rule'
        )
      ),
    enabled: !!id,
  });
}

export function useCreateAutomationRule() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (rule: CreateAutomationRuleInput): Promise<AutomationRule> =>
      ruleFromApi(
        await requestJson<ApiAutomationRule>(
          `${API_BASE}/organizations/${requireOrg()}/rules`,
          { method: 'POST', body: JSON.stringify(ruleToApi(rule)) },
          'Failed to create automation rule'
        )
      ),
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
    }): Promise<AutomationRule> =>
      ruleFromApi(
        await requestJson<ApiAutomationRule>(
          `${API_BASE}/rules/${id}`,
          { method: 'PUT', body: JSON.stringify(ruleToApi(data)) },
          'Failed to update automation rule'
        )
      ),
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
      await requestJson<void>(
        `${API_BASE}/rules/${id}`,
        { method: 'DELETE' },
        'Failed to delete automation rule'
      );
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['automation-rules'] });
    },
  });
}

// Template Hooks

export function useAutomationTemplates(query: AutomationTemplatesQuery = {}) {
  return useQuery({
    queryKey: ['automation-templates', query],
    queryFn: async (): Promise<WorkflowPaginatedResponse<AutomationTemplate>> => {
      const rows = await requestJson<ApiAutomationTemplate[]>(
        `${API_BASE}/templates`,
        undefined,
        'Failed to fetch automation templates'
      );
      const filtered = rows
        .map(templateFromApi)
        .filter(
          (template) =>
            (!query.category || template.category === query.category) &&
            matchesSearch(query.search, template.name, template.description)
        );
      return paginate(filtered, query.page, query.pageSize);
    },
  });
}

export function useCreateRuleFromTemplate() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({
      templateId,
      name,
    }: {
      templateId: string;
      name: string;
    }): Promise<AutomationRule> =>
      ruleFromApi(
        await requestJson<ApiAutomationRule>(
          `${API_BASE}/organizations/${requireOrg()}/rules/from-template`,
          { method: 'POST', body: JSON.stringify({ template_id: templateId, name }) },
          'Failed to create rule from template'
        )
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['automation-rules'] });
    },
  });
}

// Execution Hooks

export function useExecutionLogs(query: ExecutionLogsQuery = {}) {
  const orgId = getOrg();
  return useQuery({
    queryKey: ['execution-logs', orgId, query],
    enabled: !!orgId,
    queryFn: async (): Promise<WorkflowPaginatedResponse<ExecutionLog>> => {
      const page = Math.max(1, query.page ?? 1);
      const pageSize = Math.max(1, query.pageSize ?? 50);
      const params = new URLSearchParams();
      params.set('limit', String(pageSize));
      params.set('offset', String((page - 1) * pageSize));
      if (query.status) params.set('status', executionStatusToApi(query.status));
      if (query.ruleId) params.set('rule_id', query.ruleId);
      if (query.since) params.set('since', query.since);

      const list = await requestJson<ApiAutomationExecutionList>(
        `${API_BASE}/organizations/${requireOrg()}/executions?${params}`,
        undefined,
        'Failed to fetch execution logs'
      );
      return {
        data: list.items.map(executionFromApi),
        total: list.total,
        page,
        pageSize,
        totalPages: Math.max(1, Math.ceil(list.total / pageSize)),
      };
    },
  });
}

export function useExecutionStats(query: ExecutionStatsQuery = {}) {
  const orgId = getOrg();
  return useQuery({
    queryKey: ['execution-stats', orgId, query],
    enabled: !!orgId,
    queryFn: async (): Promise<ExecutionStatsResponse> => {
      const params = new URLSearchParams();
      if (query.ruleId) params.set('rule_id', query.ruleId);
      if (query.since) params.set('since', query.since);

      return executionStatsFromApi(
        await requestJson<ApiAutomationExecutionStats>(
          `${API_BASE}/organizations/${requireOrg()}/executions/stats?${params}`,
          undefined,
          'Failed to fetch execution stats'
        )
      );
    },
  });
}
