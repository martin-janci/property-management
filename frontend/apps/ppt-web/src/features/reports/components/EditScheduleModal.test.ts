/// <reference types="vitest/globals" />
/**
 * Round-trip regression for report-schedule editing — gap-81-1 / issue #616.
 *
 * The bug: cron edits were saved to the backend but, on re-open, the edit
 * modal reconstructed the cron expression from the overloaded `time` /
 * `frequency` / `day_*` fields rather than reading the value the backend
 * actually persisted. A custom expression (e.g. every-15-min weekday business
 * hours) was therefore silently flattened on the next edit.
 *
 * The fix wires the read side to the dedicated `cron_expression` field while
 * keeping the legacy `time`-derived reconstruction as a back-compat fallback
 * for rows that predate the column. These tests pin both halves of that
 * contract.
 */

import type { ReportSchedule } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { scheduleToInitialCron } from './EditScheduleModal';

/** Build a ReportSchedule fixture with sane defaults, overridable per-test. */
function makeSchedule(overrides: Partial<ReportSchedule> = {}): ReportSchedule {
  return {
    id: 'sched-1',
    report_id: 'rep-1',
    organization_id: 'org-1',
    name: 'Test Schedule',
    frequency: 'daily',
    time: '08:00',
    timezone: 'Europe/Bratislava',
    format: 'pdf',
    recipients: ['a@example.com'],
    is_active: true,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...overrides,
  };
}

describe('scheduleToInitialCron — cron round-trip (gap-81-1 / #616)', () => {
  it('surfaces a stored cron_expression verbatim instead of reconstructing from time', () => {
    // Custom expression that CANNOT be reconstructed from time/frequency:
    // every 15 min, weekday business hours.
    const custom = '*/15 9-17 * * 1-5';
    const schedule = makeSchedule({
      cron_expression: custom,
      // Legacy fields are present but must be ignored when a cron is stored.
      time: '08:00',
      frequency: 'daily',
    });

    expect(scheduleToInitialCron(schedule)).toBe(custom);
  });

  it('preserves a stored cron even when frequency/time would derive a different one', () => {
    const schedule = makeSchedule({
      cron_expression: '0 6 * * 0', // weekly Sunday 06:00
      frequency: 'monthly',
      day_of_month: 15,
      time: '23:30',
    });

    expect(scheduleToInitialCron(schedule)).toBe('0 6 * * 0');
  });

  it('ignores trailing/leading whitespace around the stored cron', () => {
    const schedule = makeSchedule({ cron_expression: '  30 14 * * *  ' });
    expect(scheduleToInitialCron(schedule)).toBe('30 14 * * *');
  });

  it('falls back to time-derived reconstruction when cron_expression is absent (legacy row)', () => {
    const schedule = makeSchedule({
      cron_expression: undefined,
      frequency: 'weekly',
      day_of_week: 3,
      time: '09:30',
    });
    // Reconstructed: minute hour * * dow
    expect(scheduleToInitialCron(schedule)).toBe('30 9 * * 3');
  });

  it('falls back to reconstruction when the stored cron is invalid', () => {
    const schedule = makeSchedule({
      cron_expression: 'not a cron',
      frequency: 'daily',
      time: '07:15',
    });
    expect(scheduleToInitialCron(schedule)).toBe('15 7 * * *');
  });

  it('reconstructs monthly schedules from day_of_month for legacy rows', () => {
    const schedule = makeSchedule({
      cron_expression: undefined,
      frequency: 'monthly',
      day_of_month: 12,
      time: '06:00',
    });
    expect(scheduleToInitialCron(schedule)).toBe('0 6 12 * *');
  });
});
