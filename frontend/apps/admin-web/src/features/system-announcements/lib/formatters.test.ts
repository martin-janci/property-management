/**
 * Tests for the system-announcement formatter helpers (#521).
 */
/// <reference types="vitest/globals" />

import type { SystemAnnouncement } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { isActive, isScheduled, lifecycleOf, toIso, toLocalDatetimeInput } from './formatters';

function ann(start: string, end?: string | null): SystemAnnouncement {
  return {
    id: 'a',
    title: 't',
    message: 'm',
    severity: 'info',
    start_at: start,
    end_at: end ?? null,
    is_dismissible: true,
    requires_acknowledgment: false,
    is_deleted: false,
    created_at: start,
    updated_at: start,
  } as unknown as SystemAnnouncement;
}

const NOW = new Date('2026-06-01T12:00:00Z').getTime();

describe('lifecycle helpers', () => {
  it('marks an announcement active when now ∈ [start, end)', () => {
    const a = ann('2026-06-01T10:00:00Z', '2026-06-01T14:00:00Z');
    expect(isActive(a, NOW)).toBe(true);
    expect(isScheduled(a, NOW)).toBe(false);
    expect(lifecycleOf(a, NOW)).toBe('active');
  });

  it('marks future starts as scheduled', () => {
    const a = ann('2026-06-02T10:00:00Z');
    expect(isScheduled(a, NOW)).toBe(true);
    expect(lifecycleOf(a, NOW)).toBe('scheduled');
  });

  it('marks past-end announcements as expired', () => {
    const a = ann('2026-05-30T10:00:00Z', '2026-05-31T10:00:00Z');
    expect(lifecycleOf(a, NOW)).toBe('expired');
  });

  it('treats null end as never-expiring', () => {
    const a = ann('2026-05-30T10:00:00Z', null);
    expect(lifecycleOf(a, NOW)).toBe('active');
  });

  it('boundary: exactly at end is expired (half-open interval)', () => {
    const at = new Date('2026-06-01T14:00:00Z').getTime();
    const a = ann('2026-06-01T10:00:00Z', '2026-06-01T14:00:00Z');
    expect(lifecycleOf(a, at)).toBe('expired');
  });
});

describe('datetime input round-trip', () => {
  it('toLocalDatetimeInput returns empty for null/undefined', () => {
    expect(toLocalDatetimeInput(null)).toBe('');
    expect(toLocalDatetimeInput(undefined)).toBe('');
  });

  it('toIso accepts a local datetime-local string', () => {
    const iso = toIso('2026-06-01T09:30');
    expect(() => new Date(iso).toISOString()).not.toThrow();
  });

  it('toIso returns empty string for empty input', () => {
    expect(toIso('')).toBe('');
  });
});
