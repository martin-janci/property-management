/**
 * Workflow Automation wire mappers
 *
 * The backend (`/api/v1/automation`, routes/automation.rs) speaks snake_case
 * rows that mirror the `workflow_automation_*` tables. The ppt-web feature is
 * built on the richer camelCase view model in `./types`. These pure functions
 * translate between the two so pages never see wire shapes.
 */

import type {
  ActionResult,
  ActionType,
  AutomationAction,
  AutomationRule,
  AutomationTemplate,
  AutomationTrigger,
  CreateAutomationRuleInput,
  EventTriggerType,
  ExecutionLog,
  ExecutionStatsResponse,
  ExecutionStatus,
  TemplateCategory,
  TriggerCondition,
  TriggerType,
} from './types';

// ---------------------------------------------------------------------------
// Wire shapes (backend `db::models::automation`)
// ---------------------------------------------------------------------------

export type ApiTriggerType = 'schedule' | 'event' | 'condition' | 'manual';

type JsonObject = Record<string, unknown>;

export interface ApiAutomationRule {
  id: string;
  organization_id: string;
  name: string;
  description?: string | null;
  trigger_type: string;
  trigger_config: unknown;
  conditions?: unknown;
  actions: unknown;
  is_active: boolean;
  last_run_at?: string | null;
  next_run_at?: string | null;
  run_count: number;
  error_count: number;
  last_error?: string | null;
  created_by?: string | null;
  created_at: string;
  updated_at: string;
}

export interface ApiAutomationRuleWrite {
  name?: string;
  description?: string | null;
  trigger_type?: ApiTriggerType;
  trigger_config?: JsonObject;
  conditions?: TriggerCondition[] | null;
  actions?: JsonObject[];
  is_active?: boolean;
}

export interface ApiAutomationTemplate {
  id: string;
  slug?: string | null;
  name: string;
  description?: string | null;
  category: string;
  trigger_type: string;
  trigger_config_template: unknown;
  actions_template: unknown;
  is_system: boolean;
  created_at: string;
}

export interface ApiAutomationExecution {
  id: string;
  rule_id: string;
  rule_name: string;
  trigger_type: string;
  trigger_data?: unknown;
  actions_executed?: unknown;
  status: string;
  started_at: string;
  completed_at?: string | null;
  error_message?: string | null;
  duration_ms?: number | null;
}

export interface ApiAutomationExecutionList {
  items: ApiAutomationExecution[];
  total: number;
}

export interface ApiAutomationExecutionStats {
  total: number;
  successful: number;
  failed: number;
  pending: number;
  average_duration_ms?: number | null;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function asObject(value: unknown): JsonObject {
  return value && typeof value === 'object' && !Array.isArray(value) ? (value as JsonObject) : {};
}

function asArray(value: unknown): unknown[] {
  return Array.isArray(value) ? value : [];
}

function asString(value: unknown): string | undefined {
  return typeof value === 'string' ? value : undefined;
}

function asNumber(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined;
}

const TRIGGER_FROM_API: Record<ApiTriggerType, TriggerType> = {
  schedule: 'time_based',
  event: 'event_based',
  condition: 'condition_based',
  manual: 'manual',
};

const TRIGGER_TO_API: Record<TriggerType, ApiTriggerType> = {
  time_based: 'schedule',
  event_based: 'event',
  condition_based: 'condition',
  manual: 'manual',
};

export function triggerTypeFromApi(value: string): TriggerType {
  return TRIGGER_FROM_API[value as ApiTriggerType] ?? 'manual';
}

export function triggerTypeToApi(value: TriggerType): ApiTriggerType {
  return TRIGGER_TO_API[value];
}

const CATEGORY_FROM_API: Record<string, TemplateCategory> = {
  maintenance: 'maintenance',
  financial: 'payments',
  payments: 'payments',
  communication: 'communications',
  communications: 'communications',
  documents: 'documents',
  faults: 'faults',
};

export function categoryFromApi(value: string): TemplateCategory {
  return CATEGORY_FROM_API[value] ?? 'general';
}

/** Backend log status → view-model execution status. */
export function executionStatusFromApi(value: string): ExecutionStatus {
  switch (value) {
    case 'success':
    case 'completed':
      return 'completed';
    case 'failed':
    case 'partial_failure':
      return 'failed';
    case 'pending':
    case 'running':
    case 'cancelled':
    case 'skipped':
      return value;
    default:
      return 'pending';
  }
}

/** View-model execution status → backend log status filter value. */
export function executionStatusToApi(value: ExecutionStatus): string {
  return value === 'completed' ? 'success' : value;
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

function triggerFromApi(rule: ApiAutomationRule): AutomationTrigger {
  const type = triggerTypeFromApi(rule.trigger_type);
  const config = asObject(rule.trigger_config);
  const trigger: AutomationTrigger = { type, name: asString(config.name) ?? '' };

  if (type === 'time_based') {
    trigger.timeConfig = {
      schedule: asString(config.cron) ?? asString(config.schedule) ?? '',
      timezone: asString(config.timezone),
    };
  } else if (type === 'event_based') {
    const eventType = asString(config.event_type) ?? asString(config.eventType);
    if (eventType) {
      trigger.eventConfig = {
        eventType: eventType as EventTriggerType,
        filters: (config.filters ?? config.conditions) as Record<string, unknown> | undefined,
      };
    }
  }

  const conditions = asArray(rule.conditions);
  if (conditions.length > 0) {
    trigger.conditions = conditions as TriggerCondition[];
  }
  return trigger;
}

function actionsFromApi(raw: unknown): AutomationAction[] {
  return asArray(raw).map((entry, index) => {
    const action = asObject(entry);
    const type = (asString(action.type) ?? 'send_notification') as ActionType;
    return {
      type,
      name: asString(action.name) ?? type,
      config: asObject(action.config),
      order: asNumber(action.order) ?? index,
    };
  });
}

export function ruleFromApi(rule: ApiAutomationRule): AutomationRule {
  return {
    id: rule.id,
    name: rule.name,
    description: rule.description ?? undefined,
    isEnabled: rule.is_active,
    trigger: triggerFromApi(rule),
    actions: actionsFromApi(rule.actions),
    executionStats: {
      totalRuns: rule.run_count,
      successfulRuns: Math.max(0, rule.run_count - rule.error_count),
      failedRuns: rule.error_count,
      lastRunAt: rule.last_run_at ?? undefined,
    },
    createdAt: rule.created_at,
    updatedAt: rule.updated_at,
    createdBy: rule.created_by ?? '',
  };
}

function triggerConfigToApi(trigger: AutomationTrigger): JsonObject {
  const config: JsonObject = {};
  if (trigger.name) config.name = trigger.name;
  if (trigger.type === 'time_based' && trigger.timeConfig) {
    config.cron = trigger.timeConfig.schedule;
    if (trigger.timeConfig.timezone) config.timezone = trigger.timeConfig.timezone;
  }
  if (trigger.type === 'event_based' && trigger.eventConfig) {
    config.event_type = trigger.eventConfig.eventType;
    if (trigger.eventConfig.filters) config.filters = trigger.eventConfig.filters;
  }
  return config;
}

function actionsToApi(actions: AutomationAction[]): JsonObject[] {
  return actions.map((action, index) => ({
    type: action.type,
    name: action.name,
    config: action.config ?? {},
    order: action.order ?? index,
  }));
}

/** Map a (possibly partial) view-model rule to the backend create/update body. */
export function ruleToApi(
  rule: Partial<CreateAutomationRuleInput> | Partial<AutomationRule>
): ApiAutomationRuleWrite {
  const body: ApiAutomationRuleWrite = {};
  if (rule.name !== undefined) body.name = rule.name;
  if (rule.description !== undefined) body.description = rule.description;
  if (rule.isEnabled !== undefined) body.is_active = rule.isEnabled;
  if (rule.trigger) {
    body.trigger_type = triggerTypeToApi(rule.trigger.type);
    body.trigger_config = triggerConfigToApi(rule.trigger);
    body.conditions = rule.trigger.conditions ?? [];
  }
  if (rule.actions) body.actions = actionsToApi(rule.actions);
  return body;
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

export function templateFromApi(template: ApiAutomationTemplate): AutomationTemplate {
  const config = asObject(template.trigger_config_template);
  const actions = asArray(template.actions_template).map(asObject);
  const triggerType = triggerTypeFromApi(template.trigger_type);
  const schedule = asString(config.cron);
  const eventType = asString(config.event_type);
  const check = asString(config.check);

  return {
    id: template.id,
    ...(template.slug ? { slug: template.slug } : {}),
    name: template.name,
    description: template.description ?? '',
    category: categoryFromApi(template.category),
    triggerType,
    triggerDetails: schedule || eventType ? { schedule, eventType } : undefined,
    conditionsPreview: check
      ? [`${check} ${asString(config.operator) ?? ''} ${String(config.value ?? '')}`.trim()]
      : undefined,
    actionsPreview: actions.map((a) => asString(a.name) ?? asString(a.type) ?? '').filter(Boolean),
    actionsCount: actions.length,
    tags: template.is_system ? ['system'] : undefined,
  };
}

// ---------------------------------------------------------------------------
// Executions
// ---------------------------------------------------------------------------

function actionResultsFromApi(raw: unknown): ActionResult[] | undefined {
  const entries = asArray(raw);
  if (entries.length === 0) return undefined;
  return entries.map((entry) => {
    const result = asObject(entry);
    return {
      actionName: asString(result.name) ?? asString(result.type) ?? '',
      status: executionStatusFromApi(asString(result.status) ?? 'completed'),
      duration: asNumber(result.duration_ms) ?? asNumber(result.duration),
      output: result.output,
      error: asString(result.error),
    };
  });
}

export function executionFromApi(execution: ApiAutomationExecution): ExecutionLog {
  const context = asObject(execution.trigger_data);
  return {
    id: execution.id,
    ruleId: execution.rule_id,
    ruleName: execution.rule_name,
    status: executionStatusFromApi(execution.status),
    triggerType: triggerTypeFromApi(execution.trigger_type),
    triggerContext: Object.keys(context).length > 0 ? context : undefined,
    startedAt: execution.started_at,
    completedAt: execution.completed_at ?? undefined,
    duration: execution.duration_ms ?? undefined,
    actionResults: actionResultsFromApi(execution.actions_executed),
    errorMessage: execution.error_message ?? undefined,
  };
}

export function executionStatsFromApi(stats: ApiAutomationExecutionStats): ExecutionStatsResponse {
  return {
    totalExecutions: stats.total,
    successfulExecutions: stats.successful,
    failedExecutions: stats.failed,
    pendingExecutions: stats.pending,
    averageDuration:
      stats.average_duration_ms === null || stats.average_duration_ms === undefined
        ? undefined
        : Math.round(stats.average_duration_ms),
    successRate: stats.total > 0 ? Math.round((stats.successful / stats.total) * 100) : undefined,
  };
}

// ---------------------------------------------------------------------------
// Client-side list shaping (rules/templates endpoints return full arrays)
// ---------------------------------------------------------------------------

export function paginate<T>(items: T[], page = 1, pageSize = 50) {
  const size = Math.max(1, pageSize);
  const current = Math.max(1, page);
  const start = (current - 1) * size;
  return {
    data: items.slice(start, start + size),
    total: items.length,
    page: current,
    pageSize: size,
    totalPages: Math.max(1, Math.ceil(items.length / size)),
  };
}

export function matchesSearch(search: string | undefined, ...fields: (string | undefined)[]) {
  if (!search) return true;
  const needle = search.trim().toLowerCase();
  if (!needle) return true;
  return fields.some((field) => field?.toLowerCase().includes(needle));
}
