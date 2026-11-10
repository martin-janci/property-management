import { describe, expect, it } from 'vitest';

import {
  type ApiAutomationRule,
  executionFromApi,
  executionStatsFromApi,
  executionStatusToApi,
  paginate,
  ruleFromApi,
  ruleToApi,
  templateFromApi,
} from './mappers';

const apiRule: ApiAutomationRule = {
  id: 'r1',
  organization_id: 'o1',
  name: 'Weekly report',
  description: null,
  trigger_type: 'schedule',
  trigger_config: { cron: '0 9 * * 1', timezone: 'Europe/Bratislava', name: 'Mondays' },
  conditions: null,
  actions: [{ type: 'send_email', config: { template: 'weekly' } }],
  is_active: true,
  last_run_at: null,
  next_run_at: null,
  run_count: 5,
  error_count: 2,
  last_error: null,
  created_by: 'u1',
  created_at: '2026-01-01T00:00:00Z',
  updated_at: '2026-01-02T00:00:00Z',
};

describe('workflow-automation mappers', () => {
  it('maps a backend rule into the view model', () => {
    const rule = ruleFromApi(apiRule);
    expect(rule.isEnabled).toBe(true);
    expect(rule.trigger).toEqual({
      type: 'time_based',
      name: 'Mondays',
      timeConfig: { schedule: '0 9 * * 1', timezone: 'Europe/Bratislava' },
    });
    expect(rule.actions).toEqual([
      { type: 'send_email', name: 'send_email', config: { template: 'weekly' }, order: 0 },
    ]);
    expect(rule.executionStats).toMatchObject({ totalRuns: 5, successfulRuns: 3, failedRuns: 2 });
  });

  it('round-trips a rule through the write body', () => {
    const body = ruleToApi(ruleFromApi(apiRule));
    expect(body.trigger_type).toBe('schedule');
    expect(body.trigger_config).toEqual({
      name: 'Mondays',
      cron: '0 9 * * 1',
      timezone: 'Europe/Bratislava',
    });
    expect(body.is_active).toBe(true);
    expect(ruleFromApi({ ...apiRule, ...body } as ApiAutomationRule).trigger).toEqual(
      ruleFromApi(apiRule).trigger
    );
  });

  it('sends only the fields present on a partial update', () => {
    expect(ruleToApi({ isEnabled: false })).toEqual({ is_active: false });
  });

  it('maps event triggers to the backend event_type key', () => {
    const body = ruleToApi({
      trigger: { type: 'event_based', name: '', eventConfig: { eventType: 'fault_created' } },
    });
    expect(body.trigger_type).toBe('event');
    expect(body.trigger_config).toEqual({ event_type: 'fault_created' });
  });

  it('maps templates, including category and previews', () => {
    const template = templateFromApi({
      id: 't1',
      slug: 'overdue_payment_reminder',
      name: 'Overdue Payment Reminder',
      description: 'Remind',
      category: 'financial',
      trigger_type: 'condition',
      trigger_config_template: { check: 'invoice.days_overdue', operator: '>=', value: 7 },
      actions_template: [{ type: 'send_notification', config: {} }, { type: 'send_email' }],
      is_system: true,
      created_at: '2026-01-01T00:00:00Z',
    });
    expect(template.slug).toBe('overdue_payment_reminder');
    expect(template.category).toBe('payments');
    expect(template.triggerType).toBe('condition_based');
    expect(template.conditionsPreview).toEqual(['invoice.days_overdue >= 7']);
    expect(template.actionsPreview).toEqual(['send_notification', 'send_email']);
    expect(template.actionsCount).toBe(2);
  });

  it('omits the slug for custom templates', () => {
    const template = templateFromApi({
      id: 't2',
      slug: null,
      name: 'My template',
      category: 'general',
      trigger_type: 'manual',
      trigger_config_template: {},
      actions_template: [],
      is_system: false,
      created_at: '2026-01-01T00:00:00Z',
    });
    expect(template.slug).toBeUndefined();
    expect(template.name).toBe('My template');
  });

  it('maps executions and their status vocabulary', () => {
    const log = executionFromApi({
      id: 'l1',
      rule_id: 'r1',
      rule_name: 'Weekly report',
      trigger_type: 'manual',
      trigger_data: null,
      actions_executed: null,
      status: 'partial_failure',
      started_at: '2026-01-01T00:00:00Z',
      completed_at: null,
      error_message: 'smtp down',
      duration_ms: 120,
    });
    expect(log).toMatchObject({
      status: 'failed',
      triggerType: 'manual',
      duration: 120,
      errorMessage: 'smtp down',
    });
    expect(executionStatusToApi('completed')).toBe('success');
    expect(executionStatusToApi('failed')).toBe('failed');
  });

  it('maps execution stats and derives a success rate', () => {
    expect(
      executionStatsFromApi({
        total: 4,
        successful: 3,
        failed: 1,
        pending: 0,
        average_duration_ms: 150.4,
      })
    ).toEqual({
      totalExecutions: 4,
      successfulExecutions: 3,
      failedExecutions: 1,
      pendingExecutions: 0,
      averageDuration: 150,
      successRate: 75,
    });
    expect(
      executionStatsFromApi({ total: 0, successful: 0, failed: 0, pending: 0 }).successRate
    ).toBeUndefined();
  });

  it('paginates client-side lists', () => {
    expect(paginate([1, 2, 3, 4, 5], 2, 2)).toEqual({
      data: [3, 4],
      total: 5,
      page: 2,
      pageSize: 2,
      totalPages: 3,
    });
  });
});
